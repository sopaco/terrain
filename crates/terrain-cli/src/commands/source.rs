use std::path::PathBuf;

use anyhow::Result;
use terrain_core::read_source_slice;

use crate::cli::SourceCommands;
use crate::util::{print_json, require_repo_path};

pub fn run(cli_repo: Option<PathBuf>, command: SourceCommands) -> Result<()> {
    match command {
        SourceCommands::Read {
            repo_path,
            file,
            start_line,
            end_line,
        } => {
            let repo_path = require_repo_path(cli_repo, repo_path)?;
            let slice = read_source_slice(
                &repo_path.display().to_string(),
                &file,
                start_line,
                end_line,
            )?;
            print_json(&slice)
        }
    }
}
