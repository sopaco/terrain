use std::collections::HashMap;

use anyhow::Result;

use crate::acp::{acp_spawn_command, build_acp_config, default_ask_acp_skill_dir};
use terrain_core::{
    extract_source_citations, merge_citations, normalize_repo_hint, resolve_project_repo_path,
};

use super::{finalize_usage, sanitize_answer_text};
use super::prompt::build_ask_acp_prompt;
use super::tracker::now_ms;
use super::types::{
    ChatPhase, ChatReply, ChatTokenUsage, ChatToolCallRecord, ChatToolCallStatus,
};

impl super::ChatEngine {
    pub(crate) async fn run_turn_acp(
        &self,
        session_id: &str,
        query: &str,
        project: Option<&str>,
        repo_path: Option<&str>,
        mut on_chunk: impl FnMut(&str),
        mut on_tool_calls: impl FnMut(&[ChatToolCallRecord]),
        mut on_phase: impl FnMut(ChatPhase),
        mut on_usage: impl FnMut(&ChatTokenUsage),
    ) -> Result<ChatReply> {
        use adk_acp::prompt_agent;

        let prompt = build_ask_acp_prompt(query, project, &self.paths)?;
        let prompt_chars = prompt.len();

        on_phase(ChatPhase::Thinking);
        on_phase(ChatPhase::Tools);

        let started = now_ms();
        let spawn = acp_spawn_command(&self.acp_settings);
        let acp_record = ChatToolCallRecord {
            id: format!("acp-{session_id}"),
            name: "acp_agent".into(),
            arguments: serde_json::json!({ "command": spawn, "session_id": session_id }),
            result: None,
            error: None,
            status: ChatToolCallStatus::Running,
            started_at: started,
            completed_at: None,
            duration_ms: None,
        };
        on_tool_calls(&[acp_record.clone()]);

        let repo = normalize_repo_hint(repo_path)
            .map(str::to_string)
            .or_else(|| {
                project.and_then(|slug| resolve_project_repo_path(&self.paths, slug, None).ok())
            });

        let skill_dir = default_ask_acp_skill_dir();
        let mut env = HashMap::new();
        env.insert(
            "TERRAIN_ASK_SKILL".into(),
            skill_dir.display().to_string(),
        );
        env.insert(
            "TERRAIN_KNOWLEDGE_ROOT".into(),
            self.paths
                .knowledge_root_for(project)
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
        );
        if let Some(slug) = project {
            env.insert("TERRAIN_PROJECT_SLUG".into(), slug.to_string());
        }
        if let Some(ref r) = repo {
            env.insert("TERRAIN_REPO_PATH".into(), r.clone());
        }

        let config = build_acp_config(&self.acp_settings, repo.as_deref(), env);

        let response = prompt_agent(&config, &prompt)
            .await
            .map_err(|e| anyhow::anyhow!("ACP Ask failed: {e}"))?;

        let completed = now_ms();
        let mut done_record = acp_record;
        done_record.result = Some(serde_json::json!({
            "mode": "acp",
            "response_chars": response.len(),
        }));
        done_record.status = ChatToolCallStatus::Ok;
        done_record.completed_at = Some(completed);
        done_record.duration_ms = Some(completed.saturating_sub(started));
        on_tool_calls(&[done_record.clone()]);

        on_phase(ChatPhase::Generating);
        on_phase(ChatPhase::Streaming);

        for chunk in response.chars().collect::<Vec<_>>().chunks(48) {
            let s: String = chunk.iter().collect();
            on_chunk(&s);
        }

        let raw_answer = response.clone();
        let answer = sanitize_answer_text(&response);
        self.paths.write_debug_file("last-ask-raw.md", &raw_answer);
        self.paths
            .write_debug_file("last-ask-sanitized.md", &answer);

        let mut usage = ChatTokenUsage::default();
        finalize_usage(&mut usage, prompt_chars, &answer);
        on_usage(&usage);

        let mut citations = self
            .search_citations(query, project, repo_path)
            .unwrap_or_default();
        citations = merge_citations(
            citations,
            extract_source_citations(&answer, repo_path.as_deref()),
        );

        let answer = if answer.trim().is_empty() {
            "ACP agent returned empty text.".into()
        } else {
            answer
        };

        Ok(ChatReply {
            answer,
            citations,
            tool_calls: vec![done_record],
            usage,
            completed_at: now_ms(),
        })
    }
}
