use std::collections::HashSet;
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
///
/// In a compressed/folded section (legacy packs — repomix `⋮` markers) every `N:`
/// prefix is a compressed-view line number, not a source-file line number; hits
/// there report `file_line: None` so no citation can be built on them.
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

    // Pass 1: mark sections whose body contains a fold marker (`⋮`). A folded
    // section is a renumbered view — every line in it is unreliable, not just
    // the marker lines — so the verdict must be per-section and decided up front.
    // Packs that declare the current strategy are uncompressed by construction,
    // so they skip scanning: that keeps a source file containing a literal `⋮`
    // (icons, fixtures) from being mislabelled.
    let folded_headers: HashSet<usize> =
        if crate::assets::repomix::pack_declares_source_line_numbers(content) {
            HashSet::new()
        } else {
            let mut folded = HashSet::new();
            let mut header_idx: Option<usize> = None;
            let mut fence = false;
            for (idx, line) in lines.iter().enumerate() {
                if header_re.captures(line).is_some() {
                    header_idx = Some(idx);
                    fence = false;
                    continue;
                }
                let Some(h) = header_idx else { continue };
                if !fence {
                    if line.trim().is_empty() {
                        continue;
                    }
                    if line.starts_with("```") {
                        fence = true;
                    }
                    continue;
                }
                if line.starts_with("```") {
                    fence = false;
                    continue;
                }
                if line.contains('⋮') {
                    folded.insert(h);
                }
            }
            folded
        };

    let mut current_file: Option<String> = None;
    let mut current_folded = false;
    let mut in_fence = false;
    let mut hits = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        if let Some(cap) = header_re.captures(line) {
            current_file = cap.get(1).map(|m| m.as_str().replace("\\|", "|"));
            current_folded = folded_headers.contains(&idx);
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

        let file_line = if current_folded {
            // Folded section: `N:` prefixes are compressed-view numbers, not
            // source-file lines — suppress so no citation is built on them.
            None
        } else {
            parse_repomix_source_line(line)
        };
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

    const FOLDED_SAMPLE: &str = "\
# Repomix
### crates/big.rs (5 lines)
```rust
1: Section
2: ⋮----
3: {
4:     fn alpha() {}
5: }
```

### crates/plain.rs (2 lines)
```rust
10: fn alpha() {}
11: fn beta() {}
```
";

    #[test]
    fn declared_pack_keeps_file_lines_despite_ellipsis() {
        // A v2 pack declares itself trustworthy; a `⋮` in real source content
        // must not be mistaken for a fold marker.
        let pack = format!(
            "# Repomix\nTerrain Agent Source Pack (repomix-core / {})\n\n\
             ### src/menu.svelte (2 lines)\n```svelte\n1: <button>⋮</button>\n2: fn alpha() {{}}\n```\n",
            crate::assets::repomix::AGENT_PACK_STRATEGY
        );
        let hits = grep_repomix_pack(&pack, "alpha", 0, 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].file_line, Some(2));
    }

    #[test]
    fn folded_section_hits_report_no_file_line() {
        // Every hit inside a folded section must suppress file_line — its `N:`
        // prefixes are compressed-view numbers, not source-file line numbers.
        let hits = grep_repomix_pack(FOLDED_SAMPLE, "alpha", 0, 5).unwrap();
        assert_eq!(hits.len(), 2);
        let folded = hits
            .iter()
            .find(|h| h.file_path.as_deref() == Some("crates/big.rs"))
            .unwrap();
        assert_eq!(folded.file_line, None);
        let plain = hits
            .iter()
            .find(|h| h.file_path.as_deref() == Some("crates/plain.rs"))
            .unwrap();
        assert_eq!(plain.file_line, Some(10));
    }
}
