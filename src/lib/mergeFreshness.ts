import type { FreshnessSummary, ProjectOverview } from "./types";
import { tr } from "./i18n";

/** Matches `terrain_core::freshness::FRESH_THRESHOLD`. */
export const FRESH_THRESHOLD = 80;

export type FreshnessLayerStale = {
  agentPackStale: boolean;
  agentContextStale: boolean;
  humanStale: boolean;
  agentStale: boolean;
};

export function getFreshnessLayerStale(
  freshness: FreshnessSummary,
): FreshnessLayerStale {
  const agentPackStale = freshness.agent_pack_score < FRESH_THRESHOLD;
  const agentContextStale = freshness.agent_context_score < FRESH_THRESHOLD;
  const humanStale = freshness.human_docs_score < FRESH_THRESHOLD;
  return {
    agentPackStale,
    agentContextStale,
    humanStale,
    agentStale: agentPackStale || agentContextStale,
  };
}

/** Min of pack + context — what Ask / Agent macro preload actually rely on. */
export function agentLayersScore(freshness: FreshnessSummary): number {
  return Math.min(
    freshness.agent_pack_score,
    freshness.agent_context_score,
  );
}

export function formatFreshnessDriftParts(
  freshness: FreshnessSummary,
  translate: typeof tr,
): string[] {
  const parts: string[] = [
    translate("freshness.score", { score: freshness.overall_score }),
  ];
  if (freshness.commits_since_baseline > 0) {
    parts.push(
      translate("freshness.behind", {
        count: freshness.commits_since_baseline,
      }),
    );
  }
  if (freshness.changed_files_count > 0) {
    parts.push(
      translate("freshness.changedFiles", {
        count: freshness.changed_files_count,
      }),
    );
  }
  if (freshness.working_tree_dirty) {
    parts.push(translate("freshness.dirtyTree"));
  }
  return parts;
}

export function mergeFreshnessIntoOverview(
  overview: ProjectOverview,
  freshness: FreshnessSummary,
): ProjectOverview {
  const staleReason = freshness.stale_reason;
  const asset_health = overview.asset_health.map((asset) => {
    switch (asset.track) {
      case "agent_pack": {
        const stale = freshness.agent_pack_score < FRESH_THRESHOLD;
        return {
          ...asset,
          freshness_score: freshness.agent_pack_score,
          stale,
          stale_reason: stale ? staleReason : null,
        };
      }
      case "agent_context": {
        const stale = freshness.agent_context_score < FRESH_THRESHOLD;
        return {
          ...asset,
          freshness_score: freshness.agent_context_score,
          stale,
          stale_reason: stale ? staleReason : null,
        };
      }
      case "human": {
        const stale = freshness.human_docs_score < FRESH_THRESHOLD;
        return {
          ...asset,
          freshness_score: freshness.human_docs_score,
          stale,
          stale_reason: stale ? staleReason : null,
        };
      }
      default:
        return asset;
    }
  });

  return { ...overview, freshness, asset_health };
}
