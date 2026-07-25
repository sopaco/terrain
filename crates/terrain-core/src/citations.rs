use regex::Regex;
use std::sync::LazyLock;

use crate::paths::{is_terrain_knowledge_asset_path, normalize_knowledge_ref};
use crate::schema::{CitationKind, SourceCitation};

/// Longer extensions must precede shorter prefixes (`json` before `js`, `tsx` before `ts`, …).
const SOURCE_FILE_EXTENSIONS: &str =
    "tsx|jsx|mjs|cjs|yaml|json|toml|swift|java|cpp|ts|rs|py|go|kt|cs|md|yml|js|c|h";

static SOURCE_REF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"`?([a-zA-Z0-9_./-]+\.(?:{SOURCE_FILE_EXTENSIONS}))(?::(\d+)(?:-(\d+))?)?`?"
    ))
    .expect("valid source ref regex")
});

fn is_terrain_knowledge_markdown_path(file_path: &str) -> bool {
    let p = normalize_knowledge_ref(file_path);
    if !p.ends_with(".md") || p.contains("repomix.md") {
        return false;
    }
    p.starts_with("human/")
        || p.starts_with("knowledge/")
        || p.starts_with("modules/")
        || p.starts_with("interfaces/")
        || p.starts_with("routes/")
        || p == "agent/context.md"
        || p.ends_with("/context.md")
        || file_path.contains("/.terrain/")
        || file_path.trim_start_matches("./").starts_with(".terrain/")
}

fn citation_kind_for_path(path: &str) -> CitationKind {
    let p = normalize_knowledge_ref(path);
    if p.contains("/human/") || p.starts_with("human/") {
        return CitationKind::HumanDoc;
    }
    if is_terrain_knowledge_asset_path(path) || is_terrain_knowledge_markdown_path(path) {
        return CitationKind::StructuredDoc;
    }
    CitationKind::SourceCode
}

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
            kind: citation_kind_for_path(path),
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

    #[test]
    fn parses_json_path_without_js_prefix_match() {
        let refs = extract_source_citations(
            "See `.meta/freshness.json` for the ledger.",
            Some("/repo"),
        );
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].path, ".meta/freshness.json");
        assert_eq!(refs[0].kind, CitationKind::StructuredDoc);
    }

    #[test]
    fn classifies_terrain_meta_json_as_structured_doc() {
        let refs = extract_source_citations(
            "Freshness lives in .terrain/.meta/freshness.json",
            None,
        );
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].path, ".terrain/.meta/freshness.json");
        assert_eq!(refs[0].kind, CitationKind::StructuredDoc);
    }
}
