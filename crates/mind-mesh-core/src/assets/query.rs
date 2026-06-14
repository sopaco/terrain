use std::path::Path;

use regex::Regex;

use crate::error::{CoreError, Result};

#[derive(Debug, Clone, serde::Serialize)]
pub struct GrepMatch {
    pub line_number: usize,
    pub line: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

pub fn grep_file(path: &Path, pattern: &str, context: usize, limit: usize) -> Result<Vec<GrepMatch>> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        CoreError::InvalidDoc(format!("cannot read {}: {e}", path.display()))
    })?;
    grep_text(&content, pattern, context, limit)
}

pub fn grep_text(content: &str, pattern: &str, context: usize, limit: usize) -> Result<Vec<GrepMatch>> {
    let re = Regex::new(pattern)
        .map_err(|e| CoreError::InvalidDoc(format!("invalid grep pattern: {e}")))?;
    let lines: Vec<&str> = content.lines().collect();
    let mut hits = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        if !re.is_match(line) {
            continue;
        }
        let start = idx.saturating_sub(context);
        let end = (idx + context + 1).min(lines.len());
        hits.push(GrepMatch {
            line_number: idx + 1,
            line: (*line).to_string(),
            context_before: lines[start..idx].iter().map(|s| (*s).to_string()).collect(),
            context_after: lines[idx + 1..end].iter().map(|s| (*s).to_string()).collect(),
        });
        if hits.len() >= limit {
            break;
        }
    }

    Ok(hits)
}
