use mind_mesh_agent::{ChatReply, ChatTokenUsage, ChatToolCallRecord};
use mind_mesh_core::{
    extract_source_citations, merge_citations, KnowledgePaths, KnowledgeSearch, SearchOptions,
    SourceCitation,
};
use tauri::{AppHandle, Emitter, State};

use crate::AppState;

use super::payloads::{
    ChatChunkPayload, ChatDonePayload, ChatPhasePayload, ChatToolCallsPayload, ChatUsagePayload,
};

#[derive(serde::Serialize)]
pub struct AskKnowledgeReply {
    pub answer: String,
    pub citations: Vec<SourceCitation>,
    pub tool_calls: Vec<ChatToolCallRecord>,
    pub usage: ChatTokenUsage,
    pub completed_at: u64,
}

#[tauri::command]
pub async fn ask_knowledge_cmd(
    app: AppHandle,
    state: State<'_, AppState>,
    query: String,
    project: Option<String>,
    repo_path: Option<String>,
    request_id: Option<String>,
) -> Result<AskKnowledgeReply, String> {
    let stream_id = request_id.unwrap_or_else(|| format!("ask-{}", query.len()));

    let engine = match state.chat_engine().await {
        Ok(e) => e,
        Err(e) => {
            return Ok(fallback_search_reply(&state.paths, &query, project, repo_path, &e));
        }
    };

    let app_emit = app.clone();
    let app_emit_tools = app.clone();
    let app_emit_phase = app.clone();
    let app_emit_usage = app.clone();
    let sid = stream_id.clone();
    let sid_tools = stream_id.clone();
    let sid_phase = stream_id.clone();
    let sid_usage = stream_id.clone();
    let project_ref = project.clone();
    let repo_ref = repo_path.clone();

    let ChatReply {
        answer,
        citations,
        tool_calls,
        usage,
        completed_at,
    } = engine
        .ask(
            &stream_id,
            &query,
            project_ref.as_deref(),
            repo_ref.as_deref(),
            move |chunk| {
                let _ = app_emit.emit(
                    "chat-chunk",
                    ChatChunkPayload {
                        session_id: sid.clone(),
                        text: chunk.to_string(),
                    },
                );
            },
            move |calls| {
                let _ = app_emit_tools.emit(
                    "chat-tool-calls",
                    ChatToolCallsPayload {
                        session_id: sid_tools.clone(),
                        tool_calls: calls.to_vec(),
                    },
                );
            },
            move |phase| {
                let _ = app_emit_phase.emit(
                    "chat-phase",
                    ChatPhasePayload {
                        session_id: sid_phase.clone(),
                        phase,
                    },
                );
            },
            move |usage| {
                let _ = app_emit_usage.emit(
                    "chat-usage",
                    ChatUsagePayload {
                        session_id: sid_usage.clone(),
                        usage: usage.clone(),
                    },
                );
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    app.emit(
        "chat-done",
        ChatDonePayload {
            session_id: stream_id.clone(),
            answer: answer.clone(),
            citations: citations.clone(),
            tool_calls: tool_calls.clone(),
            usage: usage.clone(),
            completed_at,
        },
    )
    .map_err(|e| e.to_string())?;

    Ok(AskKnowledgeReply {
        answer,
        citations,
        tool_calls,
        usage,
        completed_at,
    })
}

fn fallback_search_reply(
    paths: &KnowledgePaths,
    query: &str,
    project: Option<String>,
    repo_path: Option<String>,
    llm_error: &str,
) -> AskKnowledgeReply {
    let hits = KnowledgeSearch::new(paths)
        .search(
            query,
            SearchOptions {
                project: project.clone(),
                doc_type: None,
                limit: 5,
            },
        )
        .unwrap_or_default();

    let citations: Vec<SourceCitation> = hits
        .iter()
        .map(|h| SourceCitation {
            kind: if h.path.contains("/human/") {
                mind_mesh_core::CitationKind::HumanDoc
            } else {
                mind_mesh_core::CitationKind::StructuredDoc
            },
            title: h.title.clone().unwrap_or_else(|| h.path.clone()),
            path: h.path.clone(),
            repo_path: repo_path.clone(),
            start_line: None,
            end_line: None,
            excerpt: Some(h.snippet.clone()),
        })
        .collect();

    let mut all_citations = citations;
    all_citations = merge_citations(
        all_citations,
        extract_source_citations(query, repo_path.as_deref()),
    );

    let answer = if all_citations.is_empty() {
        format!("LLM 不可用（{llm_error}）。未找到匹配文档。")
    } else {
        format!(
            "LLM 不可用（{llm_error}）。通过搜索找到 {} 条文档引用。",
            all_citations.len()
        )
    };

    AskKnowledgeReply {
        answer,
        citations: all_citations,
        tool_calls: Vec::new(),
        usage: ChatTokenUsage::default(),
        completed_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
    }
}
