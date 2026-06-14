use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde_json::{json, Value};

fn store() -> &'static Mutex<HashMap<String, Value>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Value>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cache_key(session_id: &str, tool: &str, fingerprint: &str) -> String {
    format!("{session_id}:{tool}:{fingerprint}")
}

/// Return a cached tool response for this session + tool + args fingerprint.
pub fn get_cached(session_id: &str, tool: &str, fingerprint: &str) -> Option<Value> {
    let key = cache_key(session_id, tool, fingerprint);
    store().lock().ok()?.get(&key).cloned()
}

pub fn store_cached(session_id: &str, tool: &str, fingerprint: &str, response: Value) {
    if let Ok(mut map) = store().lock() {
        map.insert(cache_key(session_id, tool, fingerprint), response);
    }
}

/// Return a prior tool result for the same session + args, flagged as duplicate.
pub fn duplicate_call_response(cached: Value) -> Value {
    match cached {
        Value::Object(mut map) => {
            map.insert("duplicate_call".into(), Value::Bool(true));
            map.insert(
                "message".into(),
                Value::String(
                    "Duplicate call — returning cached result from this session. Do not repeat."
                        .into(),
                ),
            );
            Value::Object(map)
        }
        other => json!({
            "duplicate_call": true,
            "message": "Duplicate call — returning cached result from this session. Do not repeat.",
            "cached": other,
        }),
    }
}

fn normalize_section_fingerprint(section: Option<&str>) -> String {
    section
        .map(|s| {
            s.trim()
                .trim_start_matches('#')
                .trim()
                .to_lowercase()
        })
        .unwrap_or_default()
}

pub fn context_call_fingerprint(project: &str, section: Option<&str>) -> String {
    format!("{}:{}", project.trim(), normalize_section_fingerprint(section))
}

pub fn pack_file_call_fingerprint(
    project: &str,
    file_path: &str,
    start_line: u32,
    end_line: u32,
) -> String {
    format!(
        "{}:{}:{}-{}",
        project.trim(),
        file_path.trim().trim_start_matches("./"),
        start_line,
        end_line
    )
}

pub fn truncate_with_notice(text: &str, max_chars: usize) -> (String, bool) {
    if text.chars().count() <= max_chars {
        return (text.to_string(), false);
    }
    let truncated: String = text.chars().take(max_chars).collect();
    (
        format!("{truncated}\n\n...[truncated at {max_chars} chars]"),
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_long_text() {
        let (out, was) = truncate_with_notice("abcdef", 3);
        assert!(was);
        assert!(out.contains("truncated"));
    }

    #[test]
    fn normalizes_context_fingerprint() {
        assert_eq!(
            context_call_fingerprint("mind-mesh", Some("  ## 核心流程 ")),
            context_call_fingerprint("mind-mesh", Some("核心流程"))
        );
    }

    #[test]
    fn duplicate_response_preserves_cached_body() {
        let cached = json!({ "body": "hello", "mode": "section" });
        let dup = duplicate_call_response(cached);
        assert_eq!(dup["duplicate_call"], true);
        assert_eq!(dup["body"], "hello");
    }
}
