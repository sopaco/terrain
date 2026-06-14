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

pub fn duplicate_call_response(tool: &str, fingerprint: &str) -> Value {
    json!({
        "duplicate_call": true,
        "message": format!(
            "Duplicate {tool} for `{fingerprint}` in this session. \
             The result is already in the conversation above — reuse it; do not call again."
        ),
    })
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
}
