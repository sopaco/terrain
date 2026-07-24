use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackFileIndexEntry {
    pub path: String,
    pub header_line: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct PackFileIndex {
    files: Vec<PackFileIndexEntry>,
}

struct PackTextCacheEntry {
    mtime: SystemTime,
    len: u64,
    text: String,
}

use std::sync::LazyLock;

static PACK_TEXT_CACHE: LazyLock<Mutex<HashMap<String, PackTextCacheEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Read repomix pack with in-process cache keyed by path + mtime.
pub fn read_pack_text_cached(pack_path: &Path) -> Result<String> {
    let key = pack_path.display().to_string();
    let meta = std::fs::metadata(pack_path).map_err(|e| {
        CoreError::InvalidDoc(format!("cannot stat pack {}: {e}", pack_path.display()))
    })?;
    let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let len = meta.len();

    if let Ok(guard) = PACK_TEXT_CACHE.lock()
        && let Some(entry) = guard.get(&key)
            && entry.mtime == mtime && entry.len == len {
                return Ok(entry.text.clone());
            }

    let text = std::fs::read_to_string(pack_path).map_err(|e| {
        CoreError::InvalidDoc(format!("cannot read pack {}: {e}", pack_path.display()))
    })?;

    if let Ok(mut guard) = PACK_TEXT_CACHE.lock() {
        guard.insert(
            key,
            PackTextCacheEntry {
                mtime,
                len,
                text: text.clone(),
            },
        );
    }

    Ok(text)
}

pub fn pack_index_path(pack_path: &Path) -> PathBuf {
    pack_path.with_extension("index.json")
}

pub fn write_pack_file_index(pack_path: &Path, pack_content: &str) -> Result<()> {
    let index = build_pack_file_index(pack_content);
    let path = pack_index_path(pack_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    crate::doc::write_json(path, &PackFileIndex { files: index })?;
    Ok(())
}

pub fn build_pack_file_index(pack_content: &str) -> Vec<PackFileIndexEntry> {
    let header_re = match Regex::new(r"^### (.+?) \(\d+ lines") {
        Ok(re) => re,
        Err(_) => return Vec::new(),
    };
    pack_content
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            let cap = header_re.captures(line)?;
            let path = cap.get(1)?.as_str().replace("\\|", "|");
            Some(PackFileIndexEntry {
                path,
                header_line: idx + 1,
            })
        })
        .collect()
}

pub fn invalidate_pack_text_cache(pack_path: &Path) {
    let key = pack_path.display().to_string();
    if let Ok(mut guard) = PACK_TEXT_CACHE.lock() {
        guard.remove(&key);
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentPackFileContent {
    /// Path requested by the caller.
    pub file_path: String,
    /// Path as stored in the pack header (may differ slightly).
    pub matched_path: String,
    /// File body from the pack (line-number prefixes stripped when present).
    pub content: String,
    /// 1-based start line within the source file.
    pub start_line: u32,
    /// 1-based end line within the source file (inclusive).
    pub end_line: u32,
    pub total_lines: u32,
    /// True when the requested range was adjusted to fit the pack slice.
    #[serde(default, skip_serializing_if = "is_false")]
    pub range_clamped: bool,
    /// Original requested start when clamped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_start_line: Option<u32>,
    /// Original requested end when clamped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_end_line: Option<u32>,
}

fn is_false(v: &bool) -> bool {
    !*v
}

fn normalize_path(path: &str) -> String {
    path.trim()
        .trim_start_matches("./")
        .replace('\\', "/")
}

fn path_suffix_segments_match(requested: &str, header: &str) -> bool {
    let req: Vec<&str> = requested.split('/').filter(|s| !s.is_empty()).collect();
    let hdr: Vec<&str> = header.split('/').filter(|s| !s.is_empty()).collect();
    let max = req.len().min(hdr.len());
    for len in 1..=max {
        if req[req.len() - len..] == hdr[hdr.len() - len..] {
            return true;
        }
    }
    false
}

fn path_matches(requested: &str, header: &str) -> bool {
    let requested = normalize_path(requested);
    let header = normalize_path(header);
    if requested == header {
        return true;
    }
    if header.ends_with(&format!("/{requested}"))
        || requested.ends_with(&format!("/{header}"))
        || header.ends_with(&requested)
        || requested.ends_with(&header)
    {
        return true;
    }
    path_suffix_segments_match(&requested, &header)
}

fn strip_line_number_prefix(line: &str) -> &str {
    if let Some((prefix, rest)) = line.split_once(':')
        && !prefix.is_empty()
            && prefix.chars().all(|c| c.is_ascii_digit())
            && rest.starts_with(' ')
        {
            return &rest[1..];
        }
    line
}

fn parse_numbered_line(line: &str) -> Option<(u32, String)> {
    if let Some((prefix, rest)) = line.split_once(':')
        && let Ok(num) = prefix.parse::<u32>() {
            let body = rest.strip_prefix(' ').unwrap_or(rest).to_string();
            return Some((num, body));
        }
    None
}

/// Read one file section from a Repomix Markdown pack (`### path (N lines)` blocks).
pub fn read_agent_pack_file(
    pack_path: &Path,
    file_path: &str,
    start_line: Option<u32>,
    end_line: Option<u32>,
) -> Result<AgentPackFileContent> {
    let content = read_pack_text_cached(pack_path)?;
    read_agent_pack_file_from_text(&content, file_path, start_line, end_line)
}

pub fn read_agent_pack_file_from_text(
    pack_content: &str,
    file_path: &str,
    start_line: Option<u32>,
    end_line: Option<u32>,
) -> Result<AgentPackFileContent> {
    let header_re = Regex::new(r"^### (.+?) \(\d+ lines").map_err(|e| {
        CoreError::InvalidDoc(format!("invalid pack header regex: {e}"))
    })?;

    let lines: Vec<&str> = pack_content.lines().collect();
    let mut matched_path = String::new();
    let mut body_lines: Vec<String> = Vec::new();
    let mut in_target = false;
    let mut in_fence = false;

    for line in &lines {
        if let Some(cap) = header_re.captures(line) {
            let header_path = cap
                .get(1)
                .map(|m| m.as_str().replace("\\|", "|"))
                .unwrap_or_default();
            if in_target {
                break;
            }
            if path_matches(file_path, &header_path) {
                matched_path = header_path;
                in_target = true;
            }
            continue;
        }

        if !in_target {
            continue;
        }

        if !in_fence {
            if line.trim().is_empty() {
                continue;
            }
            if line.starts_with("```") {
                in_fence = true;
            }
            continue;
        }

        if line.starts_with("```") {
            break;
        }
        body_lines.push((*line).to_string());
    }

    if matched_path.is_empty() {
        return Err(CoreError::InvalidDoc(format!(
            "file not found in agent pack: {file_path}"
        )));
    }

    let numbered: Vec<(u32, String)> = body_lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            parse_numbered_line(line)
                .unwrap_or_else(|| (i as u32 + 1, strip_line_number_prefix(line).to_string()))
        })
        .collect();

    let total_lines = numbered.len() as u32;
    let requested_start = start_line.unwrap_or(1).max(1);
    let requested_end = end_line.unwrap_or(total_lines.max(1));
    let mut start = requested_start;
    let mut end = requested_end;
    let mut range_clamped = false;

    if total_lines == 0 {
        return Ok(AgentPackFileContent {
            file_path: file_path.to_string(),
            matched_path,
            content: String::new(),
            start_line: 0,
            end_line: 0,
            total_lines: 0,
            range_clamped: false,
            requested_start_line: None,
            requested_end_line: None,
        });
    }

    if start > total_lines {
        start = 1;
        end = total_lines;
        range_clamped = true;
    } else {
        end = end.min(total_lines);
        if end < start {
            end = start.min(total_lines);
            range_clamped = true;
        }
        range_clamped |= requested_start != start || requested_end != end;
    }

    if start > end {
        return Err(CoreError::InvalidDoc(format!(
            "invalid line range {requested_start}-{requested_end} for {matched_path} ({total_lines} lines in pack)"
        )));
    }

    let slice: Vec<String> = numbered
        .iter()
        .filter(|(n, _)| *n >= start && *n <= end)
        .map(|(_, body)| body.clone())
        .collect();

    Ok(AgentPackFileContent {
        file_path: file_path.to_string(),
        matched_path,
        content: slice.join("\n"),
        start_line: start,
        end_line: end,
        total_lines,
        range_clamped,
        requested_start_line: if range_clamped {
            Some(requested_start)
        } else {
            None
        },
        requested_end_line: if range_clamped {
            Some(requested_end)
        } else {
            None
        },
    })
}

pub fn agent_pack_ready(paths: &crate::paths::KnowledgePaths, project_slug: &str) -> bool {
    paths.agent_pack_meta(project_slug).is_file() && paths.agent_pack_main(project_slug).is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
# Repomix

## Files

### src/lib.rs (3 lines)

```rust
1: fn main() {
2:     println!(\"hi\");
3: }
```

### src/other.rs (1 lines)

```rust
1: pub fn other() {}
```
";

    #[test]
    fn reads_full_file_from_pack() {
        let got = read_agent_pack_file_from_text(SAMPLE, "src/lib.rs", None, None).unwrap();
        assert_eq!(got.matched_path, "src/lib.rs");
        assert_eq!(got.total_lines, 3);
        assert!(got.content.contains("fn main()"));
        assert!(!got.content.contains("1:"));
    }

    #[test]
    fn reads_line_range_from_pack() {
        let got = read_agent_pack_file_from_text(SAMPLE, "lib.rs", Some(2), Some(2)).unwrap();
        assert_eq!(got.start_line, 2);
        assert_eq!(got.end_line, 2);
        assert!(got.content.contains("println!"));
    }

    #[test]
    fn clamps_out_of_range_line_request() {
        let got = read_agent_pack_file_from_text(SAMPLE, "src/lib.rs", Some(190), Some(260)).unwrap();
        assert_eq!(got.total_lines, 3);
        assert!(got.range_clamped);
        assert_eq!(got.start_line, 1);
        assert_eq!(got.end_line, 3);
        assert!(got.content.contains("fn main()"));
    }

    #[test]
    fn matches_prefixed_pack_path() {
        let got = read_agent_pack_file_from_text(
            SAMPLE,
            "android-au/src/lib.rs",
            None,
            None,
        )
        .unwrap();
        assert_eq!(got.matched_path, "src/lib.rs");
    }
}
