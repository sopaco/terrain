use regex::Regex;
use std::sync::LazyLock;

use crate::schema::{CitationKind, SourceCitation};

static SOURCE_REF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"`?([a-zA-Z0-9_./-]+\.(?:rs|ts|tsx|js|jsx|py|go|java|kt|swift|cs|cpp|c|h|md|yaml|yml|toml|json))(?::(\d+)(?:-(\d+))?)?`?",
    )
    .expect("valid source ref regex")
});

/// Extract `source_code` citations from free text (e.g. Litho docs or LLM replies).
pub fn extract_source_citations(text: &str, repo_path: Option<&str>) -> Vec<SourceCitation> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for cap in SOURCE_REF.captures_iter(text) {
        let path = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
        if path.is_empty() {
            continue;
        }
        if path == "agent/repomix.md" || path.contains("/agent/repomix.md") {
            continue;
        }
        let start = cap.get(2).and_then(|m| m.as_str().parse().ok());
        let end = cap
            .get(3)
            .and_then(|m| m.as_str().parse().ok())
            .or(start);

        let key = format!("{path}:{start:?}:{end:?}");
        if !seen.insert(key) {
            continue;
        }

        let title = if let (Some(s), Some(e)) = (start, end) {
            if s == e {
                format!("{path}:{s}")
            } else {
                format!("{path}:{s}-{e}")
            }
        } else {
            path.to_string()
        };

        out.push(SourceCitation {
            kind: CitationKind::SourceCode,
            title,
            path: path.to_string(),
            repo_path: repo_path.map(str::to_string),
            start_line: start,
            end_line: end,
            excerpt: None,
        });
    }

    out
}

pub fn merge_citations(mut base: Vec<SourceCitation>, extra: Vec<SourceCitation>) -> Vec<SourceCitation> {
    for c in extra {
        if !base.iter().any(|b| b.kind == c.kind && b.path == c.path && b.start_line == c.start_line) {
            base.push(c);
        }
    }
    base
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_path_with_line() {
        let refs = extract_source_citations("See `src/main.rs:42` for entry.", Some("/repo"));
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].path, "src/main.rs");
        assert_eq!(refs[0].start_line, Some(42));
    }

    #[test]
    fn parses_path_with_range() {
        let refs = extract_source_citations("Handler in src/api.rs:10-25", None);
        assert_eq!(refs[0].start_line, Some(10));
        assert_eq!(refs[0].end_line, Some(25));
    }

    #[test]
    fn skips_agent_pack_index_path() {
        let refs = extract_source_citations("See agent/repomix.md for context.", Some("/repo"));
        assert!(refs.is_empty());
    }
}
