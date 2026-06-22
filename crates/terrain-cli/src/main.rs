use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use terrain_agent::{
    agent_execution_ready, execution_uses_native_llm, resolve_acp_settings, resolve_model_config,
    run_agent_context_generation, run_litho_generation, ChatEngine,
};
use terrain_core::{
    KnowledgePaths, KnowledgeSearch, ProjectScanner, SearchOptions, apply_env_integration,
    build_context_overview, build_generation_plan, extract_context_section, get_env_status,
    grep_file, list_human_docs, pack_agent_assets, plan_env_integration, plan_litho_generation,
    read_agent_pack_file, read_doc_at, read_doc_at_in_project, register_project,
    resolve_project_repo_path, split_context_sections, write_agent_context, AgentPackMeta,
    AGENT_CONTEXT_TOOL_SECTION_MAX_CHARS,
};

#[derive(Parser)]
#[command(name = "terrain", about = "Terrain knowledge base CLI")]
struct Cli {
    /// Repository path (default: current Git workspace or TERRAIN_REPO_PATH)
    #[arg(long, global = true)]
    repo_path: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List indexed projects
    List,
    /// Scan a local Git repository into Markdown knowledge docs
    Scan {
        /// Repository path (default: current Git workspace)
        repo_path: Option<PathBuf>,
        #[arg(long)]
        slug: Option<String>,
    },
    /// Full-text search the knowledge base
    Search {
        query: String,
        #[arg(long)]
        project: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Read a document by path
    Read { path: String },
    /// CLI tools for ACP-mode Ask agents (JSON output)
    Tools {
        #[command(subcommand)]
        command: ToolsCommands,
    },
    /// Knowledge asset generation
    Assets {
        #[command(subcommand)]
        command: AssetCommands,
    },
    /// AI engineering environment integration (Skills, tools, AGENTS.md)
    Env {
        #[command(subcommand)]
        command: EnvCommands,
    },
}

#[derive(Subcommand)]
enum ToolsCommands {
    /// List indexed projects
    ListProjects,
    /// Repomix pack metadata for a project
    PackMeta {
        #[arg(long)]
        project: String,
    },
    /// Grep agent/repomix.md
    GrepPack {
        #[arg(long)]
        project: String,
        #[arg(long)]
        pattern: String,
        #[arg(long, default_value_t = 2)]
        context: usize,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Read a file section from the repomix pack
    ReadPackFile {
        #[arg(long)]
        project: String,
        #[arg(long)]
        file: String,
        #[arg(long)]
        start_line: Option<u32>,
        #[arg(long)]
        end_line: Option<u32>,
    },
    /// Read agent/context.md (optional section)
    ReadContext {
        #[arg(long)]
        project: String,
        #[arg(long)]
        section: Option<String>,
    },
    /// Full-text search knowledge docs
    Search {
        #[arg(long)]
        query: String,
        #[arg(long)]
        project: Option<String>,
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    /// Read a human or agent doc by project-relative path
    ReadDoc {
        #[arg(long)]
        project: String,
        #[arg(long)]
        path: String,
    },
}

#[derive(Subcommand)]
enum AssetCommands {
    /// Pack agent context with repomix-core → {repo}/.terrain/agent/repomix.md
    PackAgent {
        repo_path: PathBuf,
        #[arg(long)]
        slug: Option<String>,
    },
    /// Print Litho skill paths and output dirs for Agent-driven human doc generation
    PlanLitho {
        repo_path: PathBuf,
        #[arg(long)]
        slug: Option<String>,
    },
    /// Show full asset generation plan (Litho + Repomix)
    Plan {
        repo_path: PathBuf,
        #[arg(long)]
        slug: Option<String>,
    },
    /// Run Litho human doc generation via OpenCode ACP
    RunLitho {
        repo_path: PathBuf,
        #[arg(long)]
        slug: Option<String>,
    },
    /// List human-facing Litho docs for a project
    ListHuman {
        #[arg(long)]
        project: String,
    },
    /// Grep the Repomix agent pack for a project
    GrepPack {
        #[arg(long)]
        project: String,
        pattern: String,
        #[arg(long, default_value_t = 2)]
        context: usize,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Generate agent/context.md (architecture context for Ask mode)
    AgentContext {
        repo_path: PathBuf,
        #[arg(long)]
        slug: Option<String>,
        /// Regenerate even when context.md already exists
        #[arg(long)]
        force: bool,
    },
    /// Register a repository and point knowledge storage to {repo}/.terrain/
    Register {
        repo_path: PathBuf,
        #[arg(long)]
        slug: Option<String>,
    },
    /// Re-sanitize last-agent-context-raw.md from debug dir into context.md
    RepairContext {
        #[arg(long)]
        slug: String,
        #[arg(long)]
        repo_path: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum EnvCommands {
    /// Show integration status for a repository
    Status {
        /// Repository path (default: current Git workspace)
        repo_path: Option<PathBuf>,
    },
    /// Preview integration plan
    Plan {
        /// Repository path (default: current Git workspace)
        repo_path: Option<PathBuf>,
        /// Integration IDs (default: all non-optional + unintegrated)
        #[arg(long, value_delimiter = ',')]
        ids: Option<Vec<String>>,
    },
    /// Apply selected integrations
    Apply {
        /// Repository path (default: current Git workspace)
        repo_path: Option<PathBuf>,
        /// Integration IDs to apply (default: all)
        #[arg(long, value_delimiter = ',')]
        ids: Option<Vec<String>>,
    },
}

fn paths(cli: &Cli) -> KnowledgePaths {
    if let Some(repo) = cli.repo_path.clone() {
        return KnowledgePaths::with_workspace_repo(repo);
    }
    KnowledgePaths::from_workspace()
}

fn workspace_project_slug(paths: &KnowledgePaths) -> Option<String> {
    let repo = paths.workspace_repo()?;
    Some(slug_from(
        &repo.to_path_buf(),
        None,
    ))
}

fn require_repo_path(global: Option<PathBuf>, explicit: Option<PathBuf>) -> Result<PathBuf> {
    explicit
        .or(global)
        .or_else(KnowledgePaths::resolve_workspace_repo)
        .context("repository path is required; pass a path, --repo-path, or run inside a Git workspace")
}

fn slug_from(repo_path: &PathBuf, slug: Option<String>) -> String {
    slug.unwrap_or_else(|| {
        slug::slugify(
            repo_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("project"),
        )
    })
}

fn default_env_ids(repo_path: &std::path::Path) -> anyhow::Result<Vec<String>> {
    let status = get_env_status(repo_path)?;
    Ok(status
        .items
        .iter()
        .filter(|i| i.locked || !i.integrated)
        .map(|i| i.id.clone())
        .collect())
}

#[tokio::main]
async fn main() -> Result<()> {
    terrain_agent::load_dotenv();
    terrain_core::ensure_bundled_tools_initialized();
    terrain_core::ensure_preset_skills_initialized();
    let cli = Cli::parse();
    let paths = paths(&cli);
    let cli_repo = cli.repo_path.clone();
    paths.ensure_layout().context("create knowledge layout")?;

    match cli.command {
        Commands::List => {
            let projects = KnowledgeSearch::new(&paths).list_projects()?;
            println!("{}", serde_json::to_string_pretty(&projects)?);
        }
        Commands::Scan { repo_path, slug } => {
            let repo_path = require_repo_path(cli_repo.clone(), repo_path)?;
            let report = ProjectScanner::new(paths)
                .scan_repo(&repo_path.display().to_string(), slug.as_deref())
                .await?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Commands::Search {
            query,
            project,
            limit,
        } => {
            let project = project.or_else(|| workspace_project_slug(&paths));
            let hits = KnowledgeSearch::new(&paths).search(
                &query,
                SearchOptions {
                    project,
                    doc_type: None,
                    limit,
                },
            )?;
            println!("{}", serde_json::to_string_pretty(&hits)?);
        }
        Commands::Read { path } => {
            let doc = read_doc_at(&paths, &path)?;
            println!("{}", serde_json::to_string_pretty(&doc)?);
        }
        Commands::Tools { command } => match command {
            ToolsCommands::ListProjects => {
                let projects = KnowledgeSearch::new(&paths).list_projects()?;
                println!("{}", serde_json::to_string_pretty(&projects)?);
            }
            ToolsCommands::PackMeta { project } => {
                let meta_path = paths.agent_pack_meta(&project);
                let meta: AgentPackMeta = terrain_core::read_json(&meta_path)
                    .with_context(|| format!("read {}", meta_path.display()))?;
                println!("{}", serde_json::to_string_pretty(&meta)?);
            }
            ToolsCommands::GrepPack {
                project,
                pattern,
                context,
                limit,
            } => {
                let pack = paths.agent_pack_main(&project);
                let hits = grep_file(&pack, &pattern, context, limit)?;
                println!("{}", serde_json::to_string_pretty(&hits)?);
            }
            ToolsCommands::ReadPackFile {
                project,
                file,
                start_line,
                end_line,
            } => {
                let pack = paths.agent_pack_main(&project);
                let content = read_agent_pack_file(&pack, &file, start_line, end_line)?;
                println!("{}", serde_json::to_string_pretty(&content)?);
            }
            ToolsCommands::ReadContext { project, section } => {
                let doc = read_doc_at_in_project(&paths, "agent/context.md", Some(&project))?;
                let sections = split_context_sections(&doc.body);
                let (body, mode, title) = if let Some(ref query) = section.filter(|s| !s.trim().is_empty())
                {
                    let sec = extract_context_section(&sections, query)
                        .with_context(|| format!("section not found: {query}"))?;
                    (sec.body.clone(), "section", sec.title.clone())
                } else {
                    let overview = build_context_overview(&doc.body, AGENT_CONTEXT_TOOL_SECTION_MAX_CHARS);
                    (overview.macro_markdown, "overview", "macro".into())
                };
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "project": project,
                        "mode": mode,
                        "section": title,
                        "body": body,
                    }))?
                );
            }
            ToolsCommands::Search {
                query,
                project,
                limit,
            } => {
                let hits = KnowledgeSearch::new(&paths).search(
                    &query,
                    SearchOptions {
                        project,
                        doc_type: None,
                        limit,
                    },
                )?;
                println!("{}", serde_json::to_string_pretty(&hits)?);
            }
            ToolsCommands::ReadDoc { project, path } => {
                let doc = read_doc_at_in_project(&paths, &path, Some(&project))?;
                println!("{}", serde_json::to_string_pretty(&doc)?);
            }
        },
        Commands::Assets { command } => match command {
            AssetCommands::PackAgent { repo_path, slug } => {
                let slug = slug_from(&repo_path, slug);
                let report = pack_agent_assets(
                    &paths,
                    &slug,
                    &repo_path.display().to_string(),
                )
                .await?;
                println!("{}", serde_json::to_string_pretty(&report)?);
            }
            AssetCommands::PlanLitho { repo_path, slug } => {
                let slug = slug_from(&repo_path, slug);
                let plan = plan_litho_generation(&paths, &slug, &repo_path);
                println!("{}", serde_json::to_string_pretty(&plan)?);
            }
            AssetCommands::Plan { repo_path, slug } => {
                let slug = slug_from(&repo_path, slug);
                let plan = build_generation_plan(
                    &paths,
                    &slug,
                    &repo_path.display().to_string(),
                );
                println!("{}", serde_json::to_string_pretty(&plan)?);
            }
            AssetCommands::RunLitho { repo_path, slug } => {
                let slug = slug_from(&repo_path, slug);
                let repo = repo_path.display().to_string();
                let acp = resolve_acp_settings();
                let result = run_litho_generation(
                    &paths,
                    &slug,
                    &repo,
                    &acp,
                    |p| eprintln!("[{}] {}", p.stage, p.message),
                )
                .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            AssetCommands::ListHuman { project } => {
                let docs = list_human_docs(&paths, &project)?;
                println!("{}", serde_json::to_string_pretty(&docs)?);
            }
            AssetCommands::GrepPack {
                project,
                pattern,
                context,
                limit,
            } => {
                let pack = paths.agent_pack_main(&project);
                let hits = grep_file(&pack, &pattern, context, limit)?;
                println!("{}", serde_json::to_string_pretty(&hits)?);
            }
            AssetCommands::AgentContext {
                repo_path,
                slug,
                force,
            } => {
                let slug = slug_from(&repo_path, slug);
                let repo = repo_path.display().to_string();
                register_project(&slug, &repo)?;
                paths.ensure_project_layout(&slug)?;

                let context_path = paths.agent_context_main(&slug);
                if force && context_path.is_file() {
                    std::fs::remove_file(&context_path)?;
                    let meta_path = paths.agent_context_meta(&slug);
                    if meta_path.is_file() {
                        std::fs::remove_file(meta_path)?;
                    }
                }

                let model_config = resolve_model_config();
                let acp = resolve_acp_settings();
                agent_execution_ready(&acp, &model_config)
                    .map_err(|e| anyhow::anyhow!(e))?;
                let engine = if execution_uses_native_llm(&acp) {
                    Some(Arc::new(ChatEngine::new_native(paths.clone(), model_config)?))
                } else {
                    None
                };
                let result =
                    run_agent_context_generation(&paths, engine, &acp, &slug, &repo).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            AssetCommands::Register { repo_path, slug } => {
                let slug = slug_from(&repo_path, slug);
                let repo = repo_path.display().to_string();
                register_project(&slug, &repo)?;
                paths.ensure_project_layout(&slug)?;
                let knowledge_root = terrain_core::knowledge_root_for_repo(&repo_path);
                println!(
                    "{{\"slug\":\"{slug}\",\"repo_path\":\"{repo}\",\"knowledge_root\":\"{}\"}}",
                    knowledge_root.display()
                );
            }
            AssetCommands::RepairContext { slug, repo_path } => {
                let repo_hint = repo_path.as_ref().map(|p| p.display().to_string());
                let repo = resolve_project_repo_path(&paths, &slug, repo_hint.as_deref())?;
                let raw_path = KnowledgePaths::debug_dir().join("last-agent-context-raw.md");
                let raw = std::fs::read_to_string(&raw_path)
                    .with_context(|| format!("read {}", raw_path.display()))?;
                let meta = write_agent_context(&paths, &slug, &repo, &raw)?;
                println!("{}", serde_json::to_string_pretty(&meta)?);
            }
        },
        Commands::Env { command } => match command {
            EnvCommands::Status { repo_path } => {
                let repo_path = require_repo_path(cli_repo.clone(), repo_path)?;
                let status = get_env_status(&repo_path)?;
                println!("{}", serde_json::to_string_pretty(&status)?);
            }
            EnvCommands::Plan { repo_path, ids } => {
                let repo_path = require_repo_path(cli_repo.clone(), repo_path)?;
                let selected = match ids {
                    Some(v) if !v.is_empty() => v,
                    _ => default_env_ids(&repo_path)?,
                };
                let plan = plan_env_integration(&repo_path, &selected)?;
                println!("{}", serde_json::to_string_pretty(&plan)?);
            }
            EnvCommands::Apply { repo_path, ids } => {
                let repo_path = require_repo_path(cli_repo.clone(), repo_path)?;
                let selected = match ids {
                    Some(v) if !v.is_empty() => v,
                    _ => default_env_ids(&repo_path)?,
                };
                let result = apply_env_integration(
                    &repo_path,
                    &selected,
                    &[],
                    |p| eprintln!("[{}] {}", p.stage, p.message),
                )
                .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        },
    }

    Ok(())
}
