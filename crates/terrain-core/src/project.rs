use std::path::Path;

use crate::doc::{read_doc, read_json};
use crate::error::{CoreError, Result};
use crate::assets::{
    count_markdown_in_dir, has_litho_research_artifacts, litho_human_complete_with_research,
};
use crate::human::count_human_docs;
use crate::freshness::read_freshness_ledger;
use crate::paths::KnowledgePaths;
use crate::schema::{AgentPackMeta, DocCounts, FreshnessSummary, LithoStatus, ProjectOverview, SyncMeta};

/// Merge freshness scores into overview asset tracks (for deferred overview loading).
pub fn merge_overview_freshness(
    asset_health: Vec<crate::schema::AssetTrackHealth>,
    freshness: &FreshnessSummary,
) -> Vec<crate::schema::AssetTrackHealth> {
    apply_freshness_to_asset_health(asset_health, freshness)
}

pub fn get_project_overview(paths: &KnowledgePaths, project_slug: &str) -> Result<ProjectOverview> {
    let index_path = paths.project_index(project_slug);
    if !index_path.is_file() {
        return Err(CoreError::ProjectNotFound(format!(
            "project index not found: {}",
            index_path.display()
        )));
    }

    let doc = read_doc(&index_path)?;
    let name = doc
        .frontmatter
        .title
        .clone()
        .unwrap_or_else(|| project_slug.to_string());
    let repo_path = doc
        .frontmatter
        .source
        .clone()
        .unwrap_or_default();

    let tech_stack = parse_tech_stack(&doc.body);
    let structure_preview = parse_section(&doc.body, "Structure");
    let overview_excerpt = read_overview_excerpt(paths, project_slug);

    let sync_meta = read_json::<SyncMeta>(paths.sync_meta_path(project_slug)).ok();
    let synced_at = sync_meta.as_ref().map(|m| m.synced_at.clone());
    let collectors = sync_meta
        .map(|m| m.collectors)
        .unwrap_or_default();

    let agent_pack = read_json::<AgentPackMeta>(paths.agent_pack_meta(project_slug)).ok();
    let doc_counts = count_docs(paths, project_slug)?;
    let human_dir = paths.human_docs_dir(project_slug);
    let litho_workspace = paths.litho_workspace_dir(project_slug);
    let human_doc_count = count_human_docs(paths, project_slug);
    let litho = LithoStatus {
        human_doc_count,
        has_human_docs: human_doc_count > 0,
        human_docs_complete: litho_human_complete_with_research(&human_dir, Some(&litho_workspace)),
        has_research_artifacts: has_litho_research_artifacts(&litho_workspace),
    };

    let agent_context = crate::assets::read_agent_context_status(paths, project_slug);
    let (meta_ready, meta_summary) = crate::assets::meta_inputs_status(paths, project_slug);
    let architecture_excerpt = agent_context.excerpt.clone().or_else(|| {
        read_doc(paths.agent_context_main(project_slug))
            .ok()
            .map(|d| d.body.chars().take(800).collect())
    });

    let mut asset_health = build_asset_health(
        paths,
        project_slug,
        &doc_counts,
        &litho,
        &agent_context,
        agent_pack.as_ref(),
        meta_ready,
        &meta_summary,
        &repo_path,
    );

    let freshness = read_freshness_ledger(paths, project_slug).map(|ledger| ledger.summary);
    if let Some(ref summary) = freshness {
        asset_health = apply_freshness_to_asset_health(asset_health, summary);
    }

    let agent_env = build_agent_env_status(&repo_path);
    let project_remark = read_project_remark(paths, project_slug);

    Ok(ProjectOverview {
        slug: project_slug.to_string(),
        name,
        repo_path,
        tech_stack,
        synced_at,
        collectors,
        doc_counts,
        agent_pack,
        litho,
        agent_context,
        asset_health,
        agent_env,
        structure_preview,
        overview_excerpt,
        architecture_excerpt,
        freshness,
        project_remark,
    })
}

/// Read human-editable project remark from `.terrain/project-note.md`.
pub fn read_project_remark(paths: &KnowledgePaths, project_slug: &str) -> Option<String> {
    let path = paths.project_note_path(project_slug);
    read_remark_file(&path)
}

/// Persist project remark to `.terrain/project-note.md` (empty clears the file).
pub fn write_project_remark(
    paths: &KnowledgePaths,
    project_slug: &str,
    remark: &str,
) -> Result<()> {
    let path = paths.project_note_path(project_slug);
    write_remark_file(&path, remark)
}

fn read_remark_file(path: &Path) -> Option<String> {
    if !path.is_file() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn write_remark_file(path: &Path, remark: &str) -> Result<()> {
    let trimmed = remark.trim();
    if trimmed.is_empty() {
        if path.is_file() {
            std::fs::remove_file(path)?;
        }
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, format!("{trimmed}\n"))?;
    Ok(())
}

/// Resolve the repository path for a project slug (registry, pack meta, or index frontmatter).
pub fn resolve_project_repo_path(
    paths: &KnowledgePaths,
    project_slug: &str,
    hint: Option<&str>,
) -> Result<String> {
    if let Some(repo) = hint.filter(|r| !r.is_empty()) {
        return Ok(repo.to_string());
    }
    if let Ok(meta) = read_json::<AgentPackMeta>(paths.agent_pack_meta(project_slug)) {
        if !meta.repo_path.is_empty() {
            return Ok(meta.repo_path);
        }
    }
    if let Some(repo) = crate::registry::repo_path_for_slug(project_slug) {
        return Ok(repo);
    }
    let index_path = paths.project_index(project_slug);
    if index_path.is_file() {
        let doc = read_doc(&index_path)?;
        if let Some(source) = doc.frontmatter.source.filter(|s| !s.is_empty()) {
            return Ok(source);
        }
    }
    Err(CoreError::ProjectNotFound(format!(
        "cannot resolve repository path for project '{project_slug}'"
    )))
}

fn build_asset_health(
    paths: &KnowledgePaths,
    project_slug: &str,
    counts: &DocCounts,
    litho: &LithoStatus,
    agent_ctx: &crate::schema::AgentContextStatus,
    pack: Option<&AgentPackMeta>,
    meta_ready: bool,
    meta_summary: &str,
    repo_path: &str,
) -> Vec<crate::schema::AssetTrackHealth> {
    use crate::assets::has_repo_meta_configured;
    use crate::schema::AssetTrackHealth;

    let repo_meta_configured = if !repo_path.is_empty() {
        has_repo_meta_configured(Path::new(repo_path))
    } else {
        resolve_project_repo_path(paths, project_slug, None)
            .ok()
            .map(|repo| has_repo_meta_configured(Path::new(&repo)))
            .unwrap_or(false)
    };

    let openapi_count = counts.interfaces + counts.routes;
    vec![
        AssetTrackHealth {
            track: "human".into(),
            label: "人类友好的知识库".into(),
            ready: litho.human_docs_complete,
            summary: if litho.human_docs_complete {
                format!("{} 篇 Litho 文档", litho.human_doc_count)
            } else if litho.has_human_docs {
                format!("{} 篇（未完成）", litho.human_doc_count)
            } else {
                "未生成".into()
            },
            detail: if litho.human_docs_complete {
                "适合新人阅读与 DeepWiki".into()
            } else if litho.has_research_artifacts {
                "有 Litho 研究中间产物，编排未完成".into()
            } else if litho.has_human_docs {
                "Litho 文档不完整，请重新生成".into()
            } else {
                "适合新人阅读与 DeepWiki".into()
            },
            freshness_score: None,
            stale: None,
            stale_reason: None,
        },
        AssetTrackHealth {
            track: "agent_context".into(),
            label: "Agent 友好的知识资产".into(),
            ready: agent_ctx.ready,
            summary: if agent_ctx.ready {
                format!("{} 个章节", agent_ctx.section_count)
            } else {
                "未生成".into()
            },
            detail: "架构/模块/流程，无代码细节".into(),
            freshness_score: None,
            stale: None,
            stale_reason: None,
        },
        AssetTrackHealth {
            track: "agent_pack".into(),
            label: "Agent 源码索引".into(),
            ready: pack.is_some(),
            summary: pack
                .map(|p| format!("{} tokens · {} 文件", p.total_tokens, p.total_files))
                .unwrap_or_else(|| "未打包".into()),
            detail: "repomix 压缩签名，按需 grep".into(),
            freshness_score: None,
            stale: None,
            stale_reason: None,
        },
        AssetTrackHealth {
            track: "structured".into(),
            label: "结构化条目".into(),
            ready: meta_ready || openapi_count > 0 || repo_meta_configured,
            summary: if meta_ready {
                meta_summary.to_string()
            } else if repo_meta_configured {
                "已配置 terrain-meta.json（待生成上下文）".into()
            } else if openapi_count > 0 {
                format!(
                    "{} 接口 · {} 路由",
                    counts.interfaces, counts.routes
                )
            } else {
                "未配置".into()
            },
            detail: "开发者 terrain-meta.json + OpenAPI 采集".into(),
            freshness_score: None,
            stale: None,
            stale_reason: None,
        },
    ]
}

fn apply_freshness_to_asset_health(
    mut health: Vec<crate::schema::AssetTrackHealth>,
    fresh: &crate::schema::FreshnessSummary,
) -> Vec<crate::schema::AssetTrackHealth> {
    for item in &mut health {
        let (score, stale, reason) = match item.track.as_str() {
            "agent_pack" => (
                fresh.agent_pack_score,
                fresh.agent_pack_score < crate::freshness::FRESH_THRESHOLD,
                fresh.stale_reason.clone(),
            ),
            "agent_context" => (
                fresh.agent_context_score,
                fresh.agent_context_score < crate::freshness::FRESH_THRESHOLD,
                fresh.stale_reason.clone(),
            ),
            "human" => (
                fresh.human_docs_score,
                fresh.human_docs_score < crate::freshness::FRESH_THRESHOLD,
                fresh.stale_reason.clone(),
            ),
            _ => continue,
        };
        item.freshness_score = Some(score);
        item.stale = Some(stale);
        if stale {
            item.stale_reason = reason;
        }
    }
    health
}

fn build_agent_env_status(repo_path: &str) -> crate::schema::AgentEnvStatus {
    use crate::assets::summarize_agent_env_light;
    use crate::schema::AgentEnvStatus;

    if repo_path.is_empty() {
        return AgentEnvStatus {
            ready: false,
            integrated_count: 0,
            total_count: 0,
            summary: "未检测".into(),
            detail: "打开工程环境页以配置 Skills、工具链与 AGENTS.md".into(),
        };
    }

    let repo = Path::new(repo_path);
    let knowledge_count = crate::assets::collect_knowledge_dir_inputs(repo).len();
    summarize_agent_env_light(repo, knowledge_count)
}

fn read_overview_excerpt(paths: &KnowledgePaths, project_slug: &str) -> Option<String> {
    let candidates = [
        paths.human_docs_dir(project_slug).join("1.概述.md"),
        paths.human_docs_dir(project_slug).join("1.overview.md"),
    ];
    for path in candidates {
        if let Ok(doc) = read_doc(&path) {
            let excerpt: String = doc.body.chars().take(800).collect();
            if !excerpt.trim().is_empty() {
                return Some(excerpt);
            }
        }
    }
    None
}

fn parse_section(body: &str, heading: &str) -> Option<String> {
    let marker = format!("## {heading}");
    let start = body.find(&marker)?;
    let rest = &body[start + marker.len()..];
    let end = rest.find("\n## ").unwrap_or(rest.len());
    let section = rest[..end].trim();
    if section.is_empty() {
        None
    } else {
        Some(section.to_string())
    }
}

fn parse_tech_stack(body: &str) -> Vec<String> {
    let section = match parse_section(body, "Tech stack") {
        Some(s) => s,
        None => return Vec::new(),
    };
    if section == "_Not detected_" {
        return Vec::new();
    }
    section
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            line.strip_prefix("- ").map(str::trim).map(str::to_string)
        })
        .collect()
}

fn count_docs(paths: &KnowledgePaths, project_slug: &str) -> Result<DocCounts> {
    Ok(DocCounts {
        human: count_human_docs(paths, project_slug),
        interfaces: count_markdown_in_dir(&paths.project_dir(project_slug).join("interfaces")),
        routes: count_markdown_in_dir(&paths.project_dir(project_slug).join("routes")),
        modules: count_markdown_in_dir(&paths.project_dir(project_slug).join("modules")),
        events: count_markdown_in_dir(&paths.project_dir(project_slug).join("events")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn project_remark_roundtrip() {
        let dir = std::env::temp_dir().join(format!("mm-remark-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("project-note.md");

        assert_eq!(read_remark_file(&path), None);
        write_remark_file(&path, "  团队内部备注  ").unwrap();
        assert_eq!(
            read_remark_file(&path).as_deref(),
            Some("团队内部备注")
        );
        write_remark_file(&path, "").unwrap();
        assert!(!path.exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
