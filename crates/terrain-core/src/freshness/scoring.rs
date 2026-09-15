//! Freshness score computation helpers.

use chrono::{DateTime, Utc};

use super::git::{DriftBasis, GitDrift};
use super::FRESH_THRESHOLD;

/// Commit drift penalty per commit behind baseline (capped).
pub(crate) const COMMITS_PENALTY_PER: i32 = 2;
pub(crate) const COMMITS_PENALTY_CAP: i32 = 40;

/// Changed-file ratio penalty cap.
pub(crate) const FILES_PENALTY_CAP: i32 = 30;

/// Calendar age since last asset sync — light penalty so stale packs are not over-penalized.
pub(crate) const SYNC_AGE_PENALTY_PER_DAY: i32 = 1;
pub(crate) const SYNC_AGE_PENALTY_CAP: i32 = 5;

/// Uncommitted source changes in the working tree.
pub(crate) const DIRTY_TREE_PENALTY: i32 = 5;

/// Deducted when drift could not be measured at all — an unreachable baseline commit, or an
/// asset that records no baseline inside a Git repo.
///
/// We cannot tell whether the code moved, so the layer is scored as if it had drifted as far
/// as the measurable penalties allow (commits + files + age). That lands it below
/// [`FRESH_THRESHOLD`] and below `MACRO_PRELOAD_THRESHOLD`, so preloaded macro context is
/// withheld until the assets are regenerated against a reachable commit.
pub(crate) const UNMEASURED_DRIFT_PENALTY: i32 =
    COMMITS_PENALTY_CAP + FILES_PENALTY_CAP + SYNC_AGE_PENALTY_CAP;

/// `None` days-since-sync is unknown, not "today": charge the saturated age penalty.
const UNKNOWN_AGE_DAYS: u32 = SYNC_AGE_PENALTY_CAP as u32;

/// `agent/context.md` is an LLM-generated derivative of the source pack, so its score is
/// discounted — the architecture layer never claims to be as trustworthy as raw source.
/// Consequence: this layer caps at 90, and so does the overall (minimum) score.
pub(crate) const CONTEXT_DISCOUNT: f32 = 0.9;

/// Discounted context score before the source-index ceiling is applied.
pub(crate) fn discount_context_score(raw: u8) -> u8 {
    ((raw as f32) * CONTEXT_DISCOUNT) as u8
}

/// Final context score: discounted raw score, never above the source index it derives from.
pub(crate) fn context_score_from_raw(raw: u8, pack_score: u8) -> u8 {
    discount_context_score(raw).min(pack_score)
}

/// Score one asset layer from its drift and sync age.
///
/// The single fail-closed entry point for scoring. It exists so the two ways an asset can look
/// fresh precisely because it cannot be checked — unmeasurable drift, and an unparseable sync
/// timestamp — are decided in one place rather than at each call site:
///
/// - a non-measured [`GitDrift`] is never read as "in sync";
/// - `days_since_sync: None` is charged the full age cap rather than counting as 0 days.
pub(crate) fn score_layer(
    drift: &GitDrift,
    days_since_sync: Option<u32>,
    total_tracked_estimate: u32,
    working_tree_dirty: bool,
) -> u8 {
    if !drift.is_measured() {
        let mut score = 100 - UNMEASURED_DRIFT_PENALTY;
        if working_tree_dirty {
            score -= DIRTY_TREE_PENALTY;
        }
        return score.clamp(0, 100) as u8;
    }
    score_asset(
        drift.commits_since_baseline,
        drift.changed_files.len() as u32,
        total_tracked_estimate,
        days_since_sync.unwrap_or(UNKNOWN_AGE_DAYS),
        working_tree_dirty,
    )
}

/// Compute asset freshness score (0–100) from measured drift counts.
pub fn score_asset(
    commits_since: u32,
    changed_files_count: u32,
    total_tracked_estimate: u32,
    days_since_sync: u32,
    working_tree_dirty: bool,
) -> u8 {
    let mut score: i32 = 100;
    score -= (commits_since as i32 * COMMITS_PENALTY_PER).min(COMMITS_PENALTY_CAP);
    if total_tracked_estimate > 0 {
        let ratio = (changed_files_count as f64 / total_tracked_estimate as f64).min(1.0);
        score -= (ratio * FILES_PENALTY_CAP as f64).round() as i32;
    } else if changed_files_count > 0 {
        score -= (changed_files_count as i32).min(FILES_PENALTY_CAP);
    }
    score -= (days_since_sync as i32 * SYNC_AGE_PENALTY_PER_DAY).min(SYNC_AGE_PENALTY_CAP);
    if working_tree_dirty {
        score -= DIRTY_TREE_PENALTY;
    }
    score.clamp(0, 100) as u8
}

/// Days since an RFC 3339 timestamp, or `None` when it is missing or malformed.
///
/// `None` means "unknown" and must never be read as "synced today" — otherwise a corrupted or
/// absent timestamp silently buys the asset a full score.
pub(crate) fn days_since_rfc3339(value: &str) -> Option<u32> {
    DateTime::parse_from_rfc3339(value).ok().map(|dt| {
        let now = Utc::now();
        let synced = dt.with_timezone(&Utc);
        now.signed_duration_since(synced).num_days().max(0) as u32
    })
}

/// Why a layer is stale, if it is.
///
/// An unmeasurable drift basis is reported first and unconditionally: it is a reason to distrust
/// the asset regardless of the score, so it must not fall through to the generic causes below.
pub(crate) fn stale_reason_for(
    score: u8,
    commits: u32,
    dirty: bool,
    ready: bool,
    basis: DriftBasis,
) -> Option<String> {
    if !ready {
        return Some("asset_not_ready".into());
    }
    if let Some(reason) = basis.stale_reason() {
        return Some(reason.to_string());
    }
    if score >= FRESH_THRESHOLD {
        return None;
    }
    if commits > 0 {
        return Some(format!("repo_advanced_{commits}_commits"));
    }
    if dirty {
        return Some("working_tree_dirty".into());
    }
    Some("sync_age".into())
}

/// Overall freshness is the minimum across all three asset layers (including 0 = not ready).
pub(crate) fn overall_freshness_score(pack_score: u8, ctx_score: u8, human_score: u8) -> u8 {
    [pack_score, ctx_score, human_score]
        .into_iter()
        .min()
        .unwrap_or(0)
}

pub(crate) fn short_git_ref(value: Option<&str>) -> Option<String> {
    value.map(|h| {
        let h = h.trim();
        if h.len() <= 7 {
            h.to_string()
        } else {
            h.chars().take(7).collect()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::freshness::{FRESH_THRESHOLD, MACRO_PRELOAD_THRESHOLD};

    #[test]
    fn overall_score_includes_not_ready_layers() {
        assert_eq!(overall_freshness_score(100, 0, 100), 0);
        assert_eq!(overall_freshness_score(0, 100, 100), 0);
        assert_eq!(overall_freshness_score(100, 80, 100), 80);
        assert_eq!(overall_freshness_score(50, 60, 70), 50);
    }

    #[test]
    fn sync_age_penalty_capped_at_five() {
        assert_eq!(score_asset(0, 0, 100, 0, false), 100);
        assert_eq!(score_asset(0, 0, 100, 3, false), 97);
        assert_eq!(score_asset(0, 0, 100, 7, false), 95);
        assert_eq!(score_asset(0, 0, 100, 30, false), 95);
    }

    #[test]
    fn score_decreases_with_commits_and_age() {
        let fresh = score_asset(0, 0, 100, 0, false);
        assert!(fresh >= 95);

        let stale = score_asset(25, 50, 100, 10, true);
        assert!(stale < FRESH_THRESHOLD);
        assert!(stale < MACRO_PRELOAD_THRESHOLD);
    }

    #[test]
    fn unmeasured_drift_is_scored_as_stale() {
        use crate::freshness::git::{DriftBasis, GitDrift};

        for basis in [DriftBasis::BaselineUnreachable, DriftBasis::BaselineMissing] {
            let drift = GitDrift::unmeasured(basis);
            assert!(!drift.is_measured(), "{basis:?}");

            let score = score_layer(&drift, Some(0), 24, false);
            assert_eq!(score, 25, "{basis:?}");
            // Unmeasurable drift must withhold preloaded macro context, not merely warn.
            assert!(score < MACRO_PRELOAD_THRESHOLD, "{basis:?}");
            assert!(score < FRESH_THRESHOLD, "{basis:?}");
            // A dirty tree costs the same 5 points as everywhere else.
            assert_eq!(score_layer(&drift, Some(0), 24, true), 20, "{basis:?}");

            assert_eq!(
                stale_reason_for(score, 0, false, true, basis).as_deref(),
                basis.stale_reason()
            );
        }
    }

    #[test]
    fn measured_zero_drift_still_earns_full_marks() {
        use crate::freshness::git::{DriftBasis, GitDrift};

        let drift = GitDrift {
            commits_since_baseline: 0,
            changed_files: vec![],
            basis: DriftBasis::Measured,
        };
        assert_eq!(score_layer(&drift, Some(0), 24, false), 100);
        assert_eq!(
            stale_reason_for(100, 0, false, true, DriftBasis::Measured),
            None
        );
    }

    #[test]
    fn unknown_sync_age_is_unknown_not_today() {
        assert_eq!(days_since_rfc3339(""), None);
        assert_eq!(days_since_rfc3339("not-a-timestamp"), None);
        // A date without a time/offset is not RFC 3339.
        assert_eq!(days_since_rfc3339("2026-09-15"), None);
        assert_eq!(days_since_rfc3339(&Utc::now().to_rfc3339()), Some(0));
        assert!(days_since_rfc3339("2020-01-01T00:00:00Z").unwrap() > 100);

        use crate::freshness::git::{DriftBasis, GitDrift};
        let measured = GitDrift {
            commits_since_baseline: 0,
            changed_files: vec![],
            basis: DriftBasis::Measured,
        };
        // Unknown age costs the saturated penalty; it must not outscore a genuinely today-fresh sync.
        assert_eq!(score_layer(&measured, None, 24, false), 95);
        assert_eq!(score_layer(&measured, Some(0), 24, false), 100);
    }

    #[test]
    fn unmeasurable_drift_outranks_the_score_in_stale_reason() {
        use crate::freshness::git::DriftBasis;

        // Reported even when the score itself looks fresh, so the reason cannot be swallowed.
        assert_eq!(
            stale_reason_for(100, 0, false, true, DriftBasis::BaselineUnreachable).as_deref(),
            Some("baseline_unreachable")
        );
        // A missing asset still describes the asset itself, so it keeps priority.
        assert_eq!(
            stale_reason_for(0, 0, false, false, DriftBasis::BaselineUnreachable).as_deref(),
            Some("asset_not_ready")
        );
    }
}
