use std::collections::HashMap;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use adk_core::Part;

use super::types::{ChatToolCallRecord, ChatToolCallStatus};

pub(crate) struct ToolCallTracker {
    records: Vec<ChatToolCallRecord>,
    by_id: HashMap<String, usize>,
    clocks: HashMap<String, Instant>,
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl ToolCallTracker {
    pub(crate) fn new() -> Self {
        Self {
            records: Vec::new(),
            by_id: HashMap::new(),
            clocks: HashMap::new(),
        }
    }

    pub(crate) fn ingest_event(&mut self, event: &adk_core::Event) -> bool {
        let Some(content) = &event.llm_response.content else {
            return false;
        };

        let mut changed = false;
        for part in &content.parts {
            match part {
                Part::FunctionCall { name, args, id, .. } => {
                    // Streaming providers may emit the same call across partial chunks.
                    if event.llm_response.partial
                        && let Some(idx) = self.records.iter().rposition(|r| {
                            r.name == *name && matches!(r.status, ChatToolCallStatus::Running)
                        }) {
                            self.records[idx].arguments = args.clone();
                            if let Some(call_id) = id {
                                self.records[idx].id = call_id.clone();
                                self.by_id.insert(call_id.clone(), idx);
                            }
                            changed = true;
                            continue;
                        }

                    let call_id = id.clone().unwrap_or_else(|| {
                        if let Some(idx) = self.records.iter().rposition(|r| {
                            r.name == *name && matches!(r.status, ChatToolCallStatus::Running)
                        }) {
                            return self.records[idx].id.clone();
                        }
                        format!("{name}-{}", self.records.len())
                    });

                    if let Some(&idx) = self.by_id.get(&call_id) {
                        if self.records[idx].arguments != *args {
                            self.records[idx].arguments = args.clone();
                            changed = true;
                        }
                        continue;
                    }

                    let idx = self.records.len();
                    self.by_id.insert(call_id.clone(), idx);
                    self.clocks.insert(call_id.clone(), Instant::now());
                    self.records.push(ChatToolCallRecord {
                        id: call_id,
                        name: name.clone(),
                        arguments: args.clone(),
                        result: None,
                        error: None,
                        status: ChatToolCallStatus::Running,
                        started_at: now_ms(),
                        completed_at: None,
                        duration_ms: None,
                    });
                    changed = true;
                }
                Part::FunctionResponse {
                    function_response,
                    id,
                } => {
                    let mut matched = false;
                    if let Some(call_id) = id
                        && let Some(&idx) = self.by_id.get(call_id) {
                            apply_tool_result(self, idx, &function_response.response);
                            matched = true;
                            changed = true;
                        }
                    if !matched {
                        let call_id = function_response.name.clone();
                        if let Some(&idx) = self.by_id.get(&call_id)
                            && matches!(self.records[idx].status, ChatToolCallStatus::Running) {
                                apply_tool_result(self, idx, &function_response.response);
                                matched = true;
                                changed = true;
                            }
                    }
                    if !matched {
                        if let Some(idx) = self.records.iter().position(|r| {
                            r.name == function_response.name
                                && matches!(r.status, ChatToolCallStatus::Running)
                        }) {
                            apply_tool_result(self, idx, &function_response.response);
                            changed = true;
                        } else {
                            let call_id =
                                id.clone().unwrap_or_else(|| function_response.name.clone());
                            let idx = self.records.len();
                            self.by_id.insert(call_id.clone(), idx);
                            self.clocks.insert(call_id.clone(), Instant::now());
                            self.records.push(ChatToolCallRecord {
                                id: call_id,
                                name: function_response.name.clone(),
                                arguments: serde_json::json!({}),
                                result: Some(function_response.response.clone()),
                                error: None,
                                status: ChatToolCallStatus::Ok,
                                started_at: now_ms(),
                                completed_at: Some(now_ms()),
                                duration_ms: Some(0),
                            });
                            changed = true;
                        }
                    }
                }
                _ => {}
            }
        }
        changed
    }

    pub(crate) fn records(&self) -> &[ChatToolCallRecord] {
        &self.records
    }

    pub(crate) fn has_running(&self) -> bool {
        self.records
            .iter()
            .any(|r| matches!(r.status, ChatToolCallStatus::Running))
    }

    pub(crate) fn has_any(&self) -> bool {
        !self.records.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn all_done(&self) -> bool {
        self.has_any() && !self.has_running()
    }
}

fn apply_tool_result(tracker: &mut ToolCallTracker, idx: usize, response: &serde_json::Value) {
    let call_id = tracker.records[idx].id.clone();
    let completed = now_ms();
    if let Some(exec_ms) = crate::throttle::take_tool_execution_ms(&call_id) {
        tracker.records[idx].duration_ms = Some(exec_ms);
    } else if let Some(started) = tracker.clocks.remove(&call_id) {
        tracker.records[idx].duration_ms = Some(started.elapsed().as_millis() as u64);
    }
    tracker.records[idx].completed_at = Some(completed);
    if let Some(err) = response.get("error").and_then(|v| v.as_str()) {
        tracker.records[idx].error = Some(err.to_string());
        tracker.records[idx].status = ChatToolCallStatus::Error;
    } else {
        tracker.records[idx].result = Some(response.clone());
        tracker.records[idx].status = ChatToolCallStatus::Ok;
    }
}

#[cfg(test)]
mod tests {
    use adk_core::{Content, Event, Part};

    use super::*;

    fn function_call_event(partial: bool, name: &str, args: serde_json::Value) -> Event {
        let mut event = Event::new("inv-1");
        event.llm_response.partial = partial;
        event.llm_response.content = Some(Content {
            role: "model".into(),
            parts: vec![Part::FunctionCall {
                name: name.into(),
                args,
                id: None,
                thought_signature: None,
            }],
        });
        event
    }

    fn function_response_event(name: &str, response: serde_json::Value) -> Event {
        let mut event = Event::new("inv-1");
        event.llm_response.content = Some(Content {
            role: "function".into(),
            parts: vec![Part::FunctionResponse {
                function_response: adk_core::FunctionResponseData::new(name, response),
                id: None,
            }],
        });
        event
    }

    #[test]
    fn dedupes_partial_function_calls() {
        let mut tracker = ToolCallTracker::new();
        assert!(tracker.ingest_event(&function_call_event(
            true,
            "grep_agent_pack",
            serde_json::json!({ "pattern": "fo" }),
        )));
        assert!(tracker.ingest_event(&function_call_event(
            true,
            "grep_agent_pack",
            serde_json::json!({ "pattern": "foo" }),
        )));
        assert_eq!(tracker.records().len(), 1);
        assert_eq!(
            tracker.records()[0].arguments,
            serde_json::json!({ "pattern": "foo" })
        );
    }

    #[test]
    fn matches_response_to_running_call_by_name() {
        let mut tracker = ToolCallTracker::new();
        tracker.ingest_event(&function_call_event(
            false,
            "read_agent_pack_meta",
            serde_json::json!({ "project": "demo" }),
        ));
        tracker.ingest_event(&function_response_event(
            "read_agent_pack_meta",
            serde_json::json!({ "meta": { "total_tokens": 1 } }),
        ));
        assert!(matches!(
            tracker.records()[0].status,
            ChatToolCallStatus::Ok
        ));
        assert!(tracker.records()[0].result.is_some());
    }

    #[test]
    fn all_done_after_each_tool_batch() {
        let mut tracker = ToolCallTracker::new();
        tracker.ingest_event(&function_call_event(
            false,
            "grep_agent_pack",
            serde_json::json!({ "pattern": "a" }),
        ));
        assert!(tracker.has_running());
        assert!(!tracker.all_done());

        tracker.ingest_event(&function_response_event(
            "grep_agent_pack",
            serde_json::json!({ "matches": [] }),
        ));
        assert!(!tracker.has_running());
        assert!(tracker.all_done());

        // Second tool round
        tracker.ingest_event(&function_call_event(
            false,
            "read_agent_pack_file",
            serde_json::json!({ "file_path": "src/main.rs" }),
        ));
        assert!(tracker.has_running());
        assert!(!tracker.all_done());

        tracker.ingest_event(&function_response_event(
            "read_agent_pack_file",
            serde_json::json!({ "content": "fn main() {}" }),
        ));
        assert!(tracker.all_done());
        assert_eq!(tracker.records().len(), 2);
    }
}
