//! Freshness score computation helpers.

use chrono::{DateTime, Utc};

use super::FRESH_THRESHOLD;

/// Compute asset freshness score (0–100).
pub fn score_asset(
    commits_since: u32,
    changed_files_count: u32,
    total_tracked_estimate: u32,
    days_since_sync: u32,
    working_tree_dirty: bool,
) -> u8 {
    let mut score: i32 = 100;
    score -= (commits_since as i32 * 2).min(40);
    if total_tracked_estimate > 0 {
        let ratio = (changed_files_count as f64 / total_tracked_estimate as f64).min(1.0);
        score -= (ratio * 30.0).round() as i32;
    } else if changed_files_count > 0 {
        score -= (changed_files_count as i32).min(30);
    }
    score -= (days_since_sync as i32 * 2).min(20);
    if working_tree_dirty {
        score -= 5;
    }
    score.clamp(0, 100) as u8
}

pub(crate) fn days_since_rfc3339(value: &str) -> u32 {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| {
            let now = Utc::now();
            let synced = dt.with_timezone(&Utc);
            now.signed_duration_since(synced).num_days().max(0) as u32
        })
        .unwrap_or(0)
}

pub(crate) fn stale_reason_for(score: u8, commits: u32, dirty: bool, ready: bool) -> Option<String> {
    if !ready {
        return Some("asset_not_ready".into());
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
    fn score_decreases_with_commits_and_age() {
        let fresh = score_asset(0, 0, 100, 0, false);
        assert!(fresh >= 95);

        let stale = score_asset(25, 50, 100, 10, true);
        assert!(stale < FRESH_THRESHOLD);
        assert!(stale < MACRO_PRELOAD_THRESHOLD);
    }
}
