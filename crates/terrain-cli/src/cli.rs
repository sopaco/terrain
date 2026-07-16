use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "terrain",
    about = "Terrain CLI",
    version,
    long_version = concat!(
        env!("CARGO_PKG_VERSION"),
        "\n\nScan repos, manage .terrain/ knowledge assets, and ACP tools for Ask agents."
    ),
)]
pub struct Cli {
    /// Repository path (default: current Git workspace or TERRAIN_REPO_PATH)
    #[arg(long, global = true)]
    pub repo_path: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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
pub enum ToolsCommands {
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
    /// Recompute (if stale) and return the knowledge freshness ledger for a project
    Freshness {
        #[arg(long)]
        project: String,
    },
    /// Git-based staleness check for the CodeGraph index, independent of `codegraph status`
    CodegraphDrift {
        #[arg(long)]
        project: String,
    },
}

#[derive(Subcommand)]
pub enum AssetCommands {
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
pub enum EnvCommands {
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
