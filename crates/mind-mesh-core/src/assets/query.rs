use std::path::Path;

use regex::Regex;

use crate::error::{CoreError, Result};

#[derive(Debug, Clone, serde::Serialize)]
pub struct GrepMatch {
    /// 1-based line number in the searched file (repomix.md for pack grep).
    pub line_number: usize,
    pub line: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
    /// Repomix `### path` section containing the hit, when searching a pack.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    /// 1-based line within the source file (from repomix `N:` prefixes), when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_line: Option<u32>,
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
            file_path: None,
            file_line: None,
        });
        if hits.len() >= limit {
            break;
        }
    }

    Ok(hits)
}

/// Grep a Repomix markdown pack with per-hit source file paths and file line numbers.
pub fn grep_repomix_pack(
    content: &str,
    pattern: &str,
    context: usize,
    limit: usize,
) -> Result<Vec<GrepMatch>> {
    let re = Regex::new(pattern)
        .map_err(|e| CoreError::InvalidDoc(format!("invalid grep pattern: {e}")))?;
    let header_re = Regex::new(r"^### (.+?) \(\d+ lines")
        .map_err(|e| CoreError::InvalidDoc(format!("invalid pack header regex: {e}")))?;

    let lines: Vec<&str> = content.lines().collect();
    let mut current_file: Option<String> = None;
    let mut in_fence = false;
    let mut hits = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        if let Some(cap) = header_re.captures(line) {
            current_file = cap
                .get(1)
                .map(|m| m.as_str().replace("\\|", "|"));
            in_fence = false;
            continue;
        }

        if current_file.is_some() && !in_fence {
            if line.trim().is_empty() {
                continue;
            }
            if line.starts_with("```") {
                in_fence = true;
            }
            continue;
        }

        if in_fence && line.starts_with("```") {
            in_fence = false;
            continue;
        }

        if !in_fence || !re.is_match(line) {
            continue;
        }

        let file_line = parse_repomix_source_line(line);
        let start = idx.saturating_sub(context);
        let end = (idx + context + 1).min(lines.len());
        hits.push(GrepMatch {
            line_number: idx + 1,
            line: (*line).to_string(),
            context_before: lines[start..idx].iter().map(|s| (*s).to_string()).collect(),
            context_after: lines[idx + 1..end].iter().map(|s| (*s).to_string()).collect(),
            file_path: current_file.clone(),
            file_line,
        });
        if hits.len() >= limit {
            break;
        }
    }

    Ok(hits)
}

fn parse_repomix_source_line(line: &str) -> Option<u32> {
    let trimmed = line.trim_start();
    let (prefix, _) = trimmed.split_once(':')?;
    prefix.parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PACK_SAMPLE: &str = "\
# Repomix
### crates/foo.rs (2 lines)
```rust
10: fn alpha() {}
11: fn beta() {}
```
";

    #[test]
    fn grep_repomix_includes_file_path_and_line() {
        let hits = grep_repomix_pack(PACK_SAMPLE, "alpha", 0, 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].file_path.as_deref(), Some("crates/foo.rs"));
        assert_eq!(hits[0].file_line, Some(10));
        assert!(hits[0].line.contains("alpha"));
    }
}
