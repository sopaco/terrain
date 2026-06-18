import type { FreshnessSummary, ProjectOverview } from "./types";

/** Matches `mind_mesh_core::freshness::FRESH_THRESHOLD`. */
const FRESH_THRESHOLD = 80;

export function mergeFreshnessIntoOverview(
  overview: ProjectOverview,
  freshness: FreshnessSummary,
): ProjectOverview {
  const staleReason = freshness.stale_reason ?? undefined;
  const asset_health = overview.asset_health.map((asset) => {
    switch (asset.track) {
      case "agent_pack": {
        const stale = freshness.agent_pack_score < FRESH_THRESHOLD;
        return {
          ...asset,
          freshness_score: freshness.agent_pack_score,
          stale,
          stale_reason: stale ? staleReason : undefined,
        };
      }
      case "agent_context": {
        const stale = freshness.agent_context_score < FRESH_THRESHOLD;
        return {
          ...asset,
          freshness_score: freshness.agent_context_score,
          stale,
          stale_reason: stale ? staleReason : undefined,
        };
      }
      case "human": {
        const stale = freshness.human_docs_score < FRESH_THRESHOLD;
        return {
          ...asset,
          freshness_score: freshness.human_docs_score,
          stale,
          stale_reason: stale ? staleReason : undefined,
        };
      }
      default:
        return asset;
    }
  });

  return { ...overview, freshness, asset_health };
}
