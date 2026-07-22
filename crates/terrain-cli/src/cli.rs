use std::path::PathBuf;

use clap::{Parser, Subcommand};
use terrain_core::SddPhase;

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
        repo_path: Option<PathBuf>,
        #[arg(long)]
        slug: Option<String>,
    },
    /// Full project initialization (scan + Litho + agent context)
    Init {
        repo_path: Option<PathBuf>,
        #[arg(long)]
        slug: Option<String>,
    },
    /// Quick refresh (scan + repack + optional agent context; skips Litho)
    Refresh {
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
    /// Project registry and overview
    Project {
        #[command(subcommand)]
        command: ProjectCommands,
    },
    /// LLM and ACP settings
    Settings {
        #[command(subcommand)]
        command: SettingsCommands,
    },
    /// DeepWiki knowledge Q&A
    Ask {
        #[command(subcommand)]
        command: AskCommands,
    },
    /// SDD standardized development workflow
    Sdd {
        #[command(subcommand)]
        command: SddCommands,
    },
    /// Token usage monitoring
    Usage {
        #[command(subcommand)]
        command: UsageCommands,
    },
    /// Read live repository source slices
    Source {
        #[command(subcommand)]
        command: SourceCommands,
    },
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
pub enum ProjectCommands {
    /// Project overview (freshness, doc counts, paths)
    Overview {
        #[arg(long)]
        project: String,
    },
    /// Save a project remark
    Remark {
        #[arg(long)]
        project: String,
        remark: String,
    },
    /// Remove a project from the registry (does not delete .terrain/)
    Remove {
        #[arg(long)]
        project: String,
    },
    /// List registry projects whose repos are missing or moved
    ListStale,
    /// Read cached freshness ledger without recomputing
    FreshnessCached {
        #[arg(long)]
        project: String,
    },
}

#[derive(Subcommand)]
pub enum SettingsCommands {
    /// Show effective model settings
    Get,
    /// Save model settings from a JSON file
    Set {
        /// Path to JSON file matching ModelSettings schema
        file: PathBuf,
    },
    /// Check LLM connectivity
    CheckLlm,
    /// Check ACP agent availability
    CheckAcp,
}

#[derive(Subcommand)]
pub enum AskCommands {
    /// Ask a question against project knowledge
    Query {
        query: String,
        #[arg(long)]
        project: Option<String>,
        /// Emit NDJSON stream events (chunk, tool_calls, phase, usage, done)
        #[arg(long)]
        stream: bool,
    },
    /// List Ask sessions for a project
    SessionsList {
        #[arg(long)]
        project: String,
    },
}

#[derive(Subcommand)]
pub enum SddCommands {
    /// SDD workflow status for a project
    Status {
        #[arg(long)]
        project: String,
    },
    /// Run an SDD phase
    Run {
        #[arg(long)]
        project: String,
        #[arg(long)]
        repo_path: Option<PathBuf>,
        #[arg(long, value_enum)]
        phase: SddPhaseArg,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long)]
        input: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum SddPhaseArg {
    Requirements,
    TechDesign,
    CodeGen,
    CodeReview,
}

impl From<SddPhaseArg> for SddPhase {
    fn from(value: SddPhaseArg) -> Self {
        match value {
            SddPhaseArg::Requirements => SddPhase::Requirements,
            SddPhaseArg::TechDesign => SddPhase::TechDesign,
            SddPhaseArg::CodeGen => SddPhase::CodeGen,
            SddPhaseArg::CodeReview => SddPhase::CodeReview,
        }
    }
}

#[derive(Subcommand)]
pub enum UsageCommands {
    /// Probe configured usage data sources
    Probe,
    /// Load usage snapshot
    Snapshot {
        #[arg(long, default_value = "summary")]
        detail: String,
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
pub enum SourceCommands {
    /// Read a slice from the live repository
    Read {
        #[arg(long)]
        repo_path: Option<PathBuf>,
        #[arg(long)]
        file: String,
        #[arg(long)]
        start_line: u32,
        #[arg(long)]
        end_line: u32,
    },
}

#[derive(Subcommand)]
pub enum ToolsCommands {
    ListProjects,
    PackMeta {
        #[arg(long)]
        project: String,
    },
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
    ReadContext {
        #[arg(long)]
        project: String,
        #[arg(long)]
        section: Option<String>,
    },
    Search {
        #[arg(long)]
        query: String,
        #[arg(long)]
        project: Option<String>,
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    ReadDoc {
        #[arg(long)]
        project: String,
        #[arg(long)]
        path: String,
    },
    Freshness {
        #[arg(long)]
        project: String,
    },
    CodegraphDrift {
        #[arg(long)]
        project: String,
    },
}

#[derive(Subcommand)]
pub enum AssetCommands {
    PackAgent {
        repo_path: PathBuf,
        #[arg(long)]
        slug: Option<String>,
    },
    PlanLitho {
        repo_path: PathBuf,
        #[arg(long)]
        slug: Option<String>,
    },
    PrepareLitho {
        repo_path: PathBuf,
        #[arg(long)]
        slug: Option<String>,
    },
    Plan {
        repo_path: PathBuf,
        #[arg(long)]
        slug: Option<String>,
    },
    RunLitho {
        repo_path: PathBuf,
        #[arg(long)]
        slug: Option<String>,
        #[arg(long)]
        force: bool,
    },
    ListHuman {
        #[arg(long)]
        project: String,
    },
    GrepPack {
        #[arg(long)]
        project: String,
        pattern: String,
        #[arg(long, default_value_t = 2)]
        context: usize,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    AgentContext {
        repo_path: PathBuf,
        #[arg(long)]
        slug: Option<String>,
        #[arg(long)]
        force: bool,
    },
    Register {
        repo_path: PathBuf,
        #[arg(long)]
        slug: Option<String>,
    },
    RepairContext {
        #[arg(long)]
        slug: String,
        #[arg(long)]
        repo_path: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum EnvCommands {
    Status {
        repo_path: Option<PathBuf>,
    },
    Plan {
        repo_path: Option<PathBuf>,
        #[arg(long, value_delimiter = ',')]
        ids: Option<Vec<String>>,
        #[arg(long, value_delimiter = ',')]
        reinstall: Option<Vec<String>>,
    },
    Apply {
        repo_path: Option<PathBuf>,
        #[arg(long, value_delimiter = ',')]
        ids: Option<Vec<String>>,
        #[arg(long, value_delimiter = ',')]
        reinstall: Option<Vec<String>>,
    },
}
