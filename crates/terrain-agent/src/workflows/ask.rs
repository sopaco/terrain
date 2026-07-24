use std::sync::Mutex;

use terrain_core::{
    extract_source_citations, merge_citations, AskStreamEvent, ChatReply, ChatTokenUsage,
    CitationKind, KnowledgePaths, KnowledgeSearch, SearchOptions, SourceCitation,
};

use crate::model::{llm_status, ModelConfig};
use crate::runtime::Runtime;

pub async fn ask_knowledge(
    runtime: &Runtime,
    session_id: &str,
    query: &str,
    project: Option<&str>,
    repo_path: Option<&str>,
    on_event: impl FnMut(AskStreamEvent) + Send,
) -> anyhow::Result<ChatReply> {
    let on_event = Mutex::new(on_event);

    let engine = match runtime.chat_engine() {
        Ok(engine) => engine,
        Err(err) => {
            let reply =
                fallback_search_reply(&runtime.paths, query, project, repo_path, &err.to_string());
            if let Ok(mut guard) = on_event.lock() {
                guard(AskStreamEvent::Done { reply: reply.clone() });
            }
            return Ok(reply);
        }
    };

    let reply = engine
        .ask(
            session_id,
            query,
            project,
            repo_path,
            |chunk| {
                if let Ok(mut guard) = on_event.lock() {
                    guard(AskStreamEvent::Chunk { text: chunk.to_string() });
                }
            },
            |chunk| {
                if let Ok(mut guard) = on_event.lock() {
                    guard(AskStreamEvent::ThinkingChunk {
                        text: chunk.to_string(),
                    });
                }
            },
            |calls| {
                if let Ok(mut guard) = on_event.lock() {
                    guard(AskStreamEvent::ToolCalls {
                        tool_calls: calls.to_vec(),
                    });
                }
            },
            |phase| {
                if let Ok(mut guard) = on_event.lock() {
                    guard(AskStreamEvent::Phase { phase });
                }
            },
            |usage| {
                if let Ok(mut guard) = on_event.lock() {
                    guard(AskStreamEvent::Usage {
                        usage: usage.clone(),
                    });
                }
            },
        )
        .await?;

    if let Ok(mut guard) = on_event.lock() {
        guard(AskStreamEvent::Done { reply: reply.clone() });
    }
    Ok(reply)
}

pub fn fallback_search_reply(
    paths: &KnowledgePaths,
    query: &str,
    project: Option<&str>,
    repo_path: Option<&str>,
    llm_error: &str,
) -> ChatReply {
    let hits = KnowledgeSearch::new(paths)
        .search(
            query,
            SearchOptions {
                project: project.map(str::to_string),
                doc_type: None,
                limit: 5,
            },
        )
        .unwrap_or_default();

    let citations: Vec<SourceCitation> = hits
        .iter()
        .map(|h| SourceCitation {
            kind: if h.path.contains("/human/") {
                CitationKind::HumanDoc
            } else {
                CitationKind::StructuredDoc
            },
            title: h.title.clone().unwrap_or_else(|| h.path.clone()),
            path: h.path.clone(),
            repo_path: repo_path.map(str::to_string),
            start_line: None,
            end_line: None,
            excerpt: Some(h.snippet.clone()),
        })
        .collect();

    let citations = merge_citations(citations, extract_source_citations(query, repo_path));

    let answer = if citations.is_empty() {
        format!("LLM 不可用（{llm_error}）。未找到匹配文档。")
    } else {
        format!(
            "LLM 不可用（{llm_error}）。通过搜索找到 {} 条文档引用。",
            citations.len()
        )
    };

    ChatReply {
        answer,
        citations,
        tool_calls: Vec::new(),
        usage: ChatTokenUsage::default(),
        completed_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
    }
}

pub fn llm_ready(config: &ModelConfig) -> terrain_core::LlmStatus {
    llm_status(config)
}
