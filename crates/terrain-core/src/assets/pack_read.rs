use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::SystemTime;

use regex::Regex;

use crate::error::{CoreError, Result};

/// `### path (N lines)` section header.
///
/// Compiled once: this sits on the Agent tool hot path and used to be rebuilt
/// on every single read.
static HEADER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^### (.+?) \(\d+ lines").expect("valid pack header regex"));

/// One file section inside a repomix pack, located by **byte** offset.
///
/// Byte offsets rather than line numbers are what make a seek possible: reaching
/// line N of a `&str` still costs a walk over every preceding byte, whereas a
/// byte offset slices in O(1). This index lives only in the process cache below,
/// derived from exactly the bytes being served — so it cannot go stale against
/// the pack the way a sidecar file on disk could.
#[derive(Debug, Clone)]
struct Section {
    /// Header path as stored in the pack, with `\|` unescaped.
    path: String,
    /// Byte offset of the header line.
    start: usize,
    /// Byte offset one past this section (next header, or end of pack).
    end: usize,
}

/// Locate every `### path (N lines)` section and its byte range.
fn index_sections(pack: &str) -> Vec<Section> {
    let mut headers: Vec<(usize, String)> = Vec::new();
    let mut offset = 0usize;
    for line in pack.split_inclusive('\n') {
        // Match against the line without its terminator, mirroring `str::lines`.
        if let Some(cap) = HEADER_RE.captures(line.trim_end_matches(['\n', '\r'])) {
            let path = cap
                .get(1)
                .map(|m| m.as_str().replace("\\|", "|"))
                .unwrap_or_default();
            headers.push((offset, path));
        }
        offset += line.len();
    }

    let total = pack.len();
    headers
        .iter()
        .enumerate()
        .map(|(i, (start, path))| Section {
            path: path.clone(),
            start: *start,
            end: headers.get(i + 1).map(|(next, _)| *next).unwrap_or(total),
        })
        .collect()
}

struct PackCacheEntry {
    mtime: SystemTime,
    len: u64,
    /// `Arc` so handing the text to a caller does not copy ~1MB per call.
    text: Arc<str>,
    sections: Arc<[Section]>,
}

static PACK_TEXT_CACHE: LazyLock<Mutex<HashMap<String, PackCacheEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Pack text plus its section index, cached per path + mtime + len.
fn cached_pack(pack_path: &Path) -> Result<(Arc<str>, Arc<[Section]>)> {
    let key = pack_path.display().to_string();
    let meta = std::fs::metadata(pack_path).map_err(|e| {
        CoreError::InvalidDoc(format!("cannot stat pack {}: {e}", pack_path.display()))
    })?;
    let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let len = meta.len();

    if let Ok(guard) = PACK_TEXT_CACHE.lock()
        && let Some(entry) = guard.get(&key)
        && entry.mtime == mtime
        && entry.len == len
    {
        return Ok((entry.text.clone(), entry.sections.clone()));
    }

    let text = std::fs::read_to_string(pack_path).map_err(|e| {
        CoreError::InvalidDoc(format!("cannot read pack {}: {e}", pack_path.display()))
    })?;
    // Indexed outside the lock: one pass per pack version, not per read.
    let text: Arc<str> = Arc::from(text);
    let sections: Arc<[Section]> = Arc::from(index_sections(&text));

    if let Ok(mut guard) = PACK_TEXT_CACHE.lock() {
        guard.insert(
            key,
            PackCacheEntry {
                mtime,
                len,
                text: text.clone(),
                sections: sections.clone(),
            },
        );
    }

    Ok((text, sections))
}

/// Read repomix pack text with an in-process cache keyed by path + mtime.
pub fn read_pack_text_cached(pack_path: &Path) -> Result<Arc<str>> {
    Ok(cached_pack(pack_path)?.0)
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

/// Split a `N: body` pack line into its source line number and body.
///
/// Borrows from `line` rather than allocating. Note this is deliberately *not*
/// the same as `strip_line_number_prefix`: for a blank source line repomix emits
/// bare `42:`, which must yield an empty body — `strip_line_number_prefix`
/// requires a space after the colon and would return the whole `42:` verbatim.
fn parse_numbered_line(line: &str) -> Option<(u32, &str)> {
    if let Some((prefix, rest)) = line.split_once(':')
        && let Ok(num) = prefix.parse::<u32>() {
            return Some((num, rest.strip_prefix(' ').unwrap_or(rest)));
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
    let (text, sections) = cached_pack(pack_path)?;
    let section = find_section(&sections, file_path)?;
    slice_section(
        &text[section.start..section.end],
        file_path,
        &section.path,
        start_line,
        end_line,
    )
}

/// Text-based entry point, for exercising the section parser without touching
/// the filesystem or the cache. Production reads go through
/// [`read_agent_pack_file`], which uses the cached byte-offset index.
#[cfg(test)]
fn read_agent_pack_file_from_text(
    pack_content: &str,
    file_path: &str,
    start_line: Option<u32>,
    end_line: Option<u32>,
) -> Result<AgentPackFileContent> {
    let sections = index_sections(pack_content);
    let section = find_section(&sections, file_path)?;
    slice_section(
        &pack_content[section.start..section.end],
        file_path,
        &section.path,
        start_line,
        end_line,
    )
}

/// First section whose header path matches the request, as the linear scan did.
fn find_section<'a>(sections: &'a [Section], file_path: &str) -> Result<&'a Section> {
    sections
        .iter()
        .find(|s| path_matches(file_path, &s.path))
        .ok_or_else(|| {
            CoreError::InvalidDoc(format!("file not found in agent pack: {file_path}"))
        })
}

/// Extract the fenced body of a single pack section and apply the line range.
fn slice_section(
    section: &str,
    file_path: &str,
    matched_path: &str,
    start_line: Option<u32>,
    end_line: Option<u32>,
) -> Result<AgentPackFileContent> {
    let mut body_lines: Vec<&str> = Vec::new();
    let mut in_fence = false;
    // `skip(1)` drops the `### path (N lines)` header itself.
    for line in section.lines().skip(1) {
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
        body_lines.push(line);
    }

    let numbered: Vec<(u32, &str)> = body_lines
        .iter()
        .enumerate()
        .map(|(i, line)| match parse_numbered_line(line) {
            Some((n, body)) => (n, body),
            None => (i as u32 + 1, strip_line_number_prefix(line)),
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
            matched_path: matched_path.to_string(),
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

    let slice: Vec<&str> = numbered
        .iter()
        .filter(|(n, _)| *n >= start && *n <= end)
        .map(|(_, body)| *body)
        .collect();

    Ok(AgentPackFileContent {
        file_path: file_path.to_string(),
        matched_path: matched_path.to_string(),
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

    #[test]
    fn blank_source_lines_stay_blank() {
        // repomix emits a bare `N:` for an empty source line — it must decode to
        // an empty body, not to the literal "2:".
        const WITH_BLANK: &str = "\
### src/blank.rs (3 lines)

```rust
1: first
2:
3: third
```
";
        let got = read_agent_pack_file_from_text(WITH_BLANK, "src/blank.rs", None, None).unwrap();
        assert_eq!(got.total_lines, 3);
        assert_eq!(got.content, "first\n\nthird");
        assert!(!got.content.contains("2:"));
    }

    #[test]
    fn keeps_colon_lines_that_are_not_line_numbers() {
        const TRICKY: &str = "\
### src/map.yaml (2 lines)

```yaml
1: key: value
2: other:thing
```
";
        let got = read_agent_pack_file_from_text(TRICKY, "src/map.yaml", None, None).unwrap();
        assert_eq!(got.content, "key: value\nother:thing");
    }

    #[test]
    fn missing_file_reports_error() {
        let err = read_agent_pack_file_from_text(SAMPLE, "src/nope.rs", None, None);
        assert!(err.is_err());
    }

    #[test]
    fn reads_last_section_up_to_end_of_pack() {
        // The final section has no following header, so its range must run to EOF.
        let got = read_agent_pack_file_from_text(SAMPLE, "src/other.rs", None, None).unwrap();
        assert_eq!(got.matched_path, "src/other.rs");
        assert_eq!(got.total_lines, 1);
        assert!(got.content.contains("pub fn other()"));
    }

    #[test]
    fn does_not_bleed_into_the_next_section() {
        let got = read_agent_pack_file_from_text(SAMPLE, "src/lib.rs", None, None).unwrap();
        assert!(!got.content.contains("pub fn other()"));
    }

    #[test]
    fn section_index_covers_every_file_with_contiguous_ranges() {
        let sections = index_sections(SAMPLE);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].path, "src/lib.rs");
        assert_eq!(sections[1].path, "src/other.rs");
        // Ranges must be contiguous and the last must reach the end of the pack.
        assert_eq!(sections[0].end, sections[1].start);
        assert_eq!(sections[1].end, SAMPLE.len());
        // Each range must actually start at its own header.
        for s in &sections {
            assert!(SAMPLE[s.start..s.end].starts_with("### "));
        }
    }

    #[test]
    fn cache_serves_new_content_after_pack_is_rewritten() {
        let dir = std::env::temp_dir().join(format!(
            "terrain-pack-cache-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let pack = dir.join("repomix.md");

        std::fs::write(&pack, SAMPLE).unwrap();
        let first = read_agent_pack_file(&pack, "src/lib.rs", None, None).unwrap();
        assert!(first.content.contains("fn main()"));

        // Rewrite in place at the same path with different content and length.
        let rewritten = SAMPLE.replace("fn main()", "fn renamed_main()");
        assert_ne!(rewritten.len(), SAMPLE.len());
        std::fs::write(&pack, &rewritten).unwrap();

        let second = read_agent_pack_file(&pack, "src/lib.rs", None, None).unwrap();
        assert!(
            second.content.contains("fn renamed_main()"),
            "stale cache served old content: {}",
            second.content
        );

        // An explicit invalidation must also work (what scan calls).
        invalidate_pack_text_cache(&pack);
        let third = read_agent_pack_file(&pack, "src/lib.rs", None, None).unwrap();
        assert!(third.content.contains("fn renamed_main()"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
