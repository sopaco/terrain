use anyhow::Result;
use terrain_core::{load_usage_snapshot, probe_usage_sources, UsageDetailLevel};

use crate::cli::UsageCommands;
use crate::util::print_json;

pub fn run(command: UsageCommands) -> Result<()> {
    match command {
        UsageCommands::Probe => {
            let probe = probe_usage_sources();
            print_json(&probe)
        }
        UsageCommands::Snapshot { detail, force } => {
            let level = match detail.as_str() {
                "full" => UsageDetailLevel::Full,
                _ => UsageDetailLevel::Summary,
            };
            let snapshot = load_usage_snapshot(level, force);
            print_json(&snapshot)
        }
    }
}
