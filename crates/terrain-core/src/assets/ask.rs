use std::path::{Path, PathBuf};

use crate::paths::KnowledgePaths;
use crate::schema::AskSessionInfo;

const MAX_ASK_SESSIONS: usize = 50;
const TITLE_CHAR_LIMIT: usize = 10;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ActiveSessionFile {
    session_id: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct SessionMetaFile {
    id: String,
    title: String,
    last_replied_at: String,
}

fn session_meta_path(sessions_dir: &Path, session_id: &str) -> PathBuf {
    sessions_dir.join(session_id).join("meta.json")
}

fn session_messages_path(sessions_dir: &Path, session_id: &str) -> PathBuf {
    sessions_dir.join(session_id).join("messages.json")
}

fn session_has_content(sessions_dir: &Path, session_id: &str) -> bool {
    let path = session_messages_path(sessions_dir, session_id);
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return false;
    };
    value
        .as_array()
        .is_some_and(|arr| !arr.is_empty())
}

/// First `max_chars` Unicode scalar values (safe for CJK titles).
pub fn title_from_question(question: &str, max_chars: usize) -> String {
    question.trim().chars().take(max_chars).collect()
}

pub fn today_date_string() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

pub fn new_ask_session_id(title: &str) -> String {
    let base = slug::slugify(title.trim());
    let base = if base.is_empty() {
        "ask".to_string()
    } else {
        base
    };
    let ts = chrono::Utc::now().format("%Y%m%d%H%M%S");
    format!("{base}-{ts}")
}

fn meta_to_info(meta: SessionMetaFile) -> AskSessionInfo {
    AskSessionInfo {
        id: meta.id,
        title: meta.title,
        last_replied_at: meta.last_replied_at,
    }
}

pub fn list_ask_sessions(paths: &KnowledgePaths, project_slug: &str) -> Vec<AskSessionInfo> {
    let sessions_dir = paths.ask_sessions_dir(project_slug);
    if !sessions_dir.is_dir() {
        return Vec::new();
    }
    let mut sessions = Vec::new();
    let Ok(entries) = std::fs::read_dir(&sessions_dir) else {
        return sessions;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().into_owned();
        if !session_has_content(&sessions_dir, &id) {
            continue;
        }
        let meta_path = session_meta_path(&sessions_dir, &id);
        if let Ok(raw) = std::fs::read_to_string(&meta_path) {
            if let Ok(meta) = serde_json::from_str::<SessionMetaFile>(&raw) {
                sessions.push(meta_to_info(meta));
                continue;
            }
        }
        sessions.push(AskSessionInfo {
            id: id.clone(),
            title: id,
            last_replied_at: String::new(),
        });
    }
    sessions.sort_by(|a, b| b.last_replied_at.cmp(&a.last_replied_at));
    sessions
}

pub fn get_active_ask_session(paths: &KnowledgePaths, project_slug: &str) -> Option<String> {
    let path = paths.ask_active_session_path(project_slug);
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<ActiveSessionFile>(&raw)
        .ok()
        .map(|f| f.session_id)
}

pub fn set_active_ask_session(
    paths: &KnowledgePaths,
    project_slug: &str,
    session_id: &str,
) -> crate::error::Result<()> {
    let session_dir = paths.ask_workspace_dir(project_slug, session_id);
    if !session_dir.is_dir() {
        return Err(crate::error::CoreError::InvalidDoc(format!(
            "Ask session not found: {session_id}"
        )));
    }
    let root = paths.ask_local_root(project_slug);
    std::fs::create_dir_all(&root)?;
    let active = ActiveSessionFile {
        session_id: session_id.to_string(),
    };
    let raw = serde_json::to_string_pretty(&active)?;
    std::fs::write(paths.ask_active_session_path(project_slug), raw)?;
    Ok(())
}

fn prune_old_sessions(paths: &KnowledgePaths, project_slug: &str) -> crate::error::Result<()> {
    let mut sessions = list_ask_sessions(paths, project_slug);
    if sessions.len() <= MAX_ASK_SESSIONS {
        return Ok(());
    }
    sessions.sort_by(|a, b| a.last_replied_at.cmp(&b.last_replied_at));
    let to_remove = sessions.len() - MAX_ASK_SESSIONS;
    for session in sessions.into_iter().take(to_remove) {
        let _ = delete_ask_session(paths, project_slug, &session.id);
    }
    Ok(())
}

pub fn create_ask_session(
    paths: &KnowledgePaths,
    project_slug: &str,
    question: &str,
) -> crate::error::Result<AskSessionInfo> {
    let title = title_from_question(question, TITLE_CHAR_LIMIT);
    let id = new_ask_session_id(&title);
    let sessions_dir = paths.ask_sessions_dir(project_slug);
    std::fs::create_dir_all(&sessions_dir)?;
    let session_dir = sessions_dir.join(&id);
    std::fs::create_dir_all(&session_dir)?;

    let meta = SessionMetaFile {
        id: id.clone(),
        title: if title.is_empty() {
            "新对话".to_string()
        } else {
            title
        },
        last_replied_at: today_date_string(),
    };
    let meta_path = session_meta_path(&sessions_dir, &id);
    std::fs::write(meta_path, serde_json::to_string_pretty(&meta)?)?;
    std::fs::write(
        session_messages_path(&sessions_dir, &id),
        "[]",
    )?;
    set_active_ask_session(paths, project_slug, &id)?;
    prune_old_sessions(paths, project_slug)?;
    Ok(meta_to_info(meta))
}

pub fn clear_active_ask_session(
    paths: &KnowledgePaths,
    project_slug: &str,
) -> crate::error::Result<()> {
    let path = paths.ask_active_session_path(project_slug);
    if path.is_file() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

/// Remove a session directory without activating another session.
pub fn discard_ask_session(
    paths: &KnowledgePaths,
    project_slug: &str,
    session_id: &str,
) -> crate::error::Result<()> {
    let session_dir = paths.ask_workspace_dir(project_slug, session_id);
    if session_dir.is_dir() {
        std::fs::remove_dir_all(session_dir)?;
    }
    if get_active_ask_session(paths, project_slug).as_deref() == Some(session_id) {
        clear_active_ask_session(paths, project_slug)?;
    }
    Ok(())
}

pub fn delete_ask_session(
    paths: &KnowledgePaths,
    project_slug: &str,
    session_id: &str,
) -> crate::error::Result<()> {
    let session_dir = paths.ask_workspace_dir(project_slug, session_id);
    if session_dir.is_dir() {
        std::fs::remove_dir_all(session_dir)?;
    }
    if get_active_ask_session(paths, project_slug).as_deref() == Some(session_id) {
        let active_path = paths.ask_active_session_path(project_slug);
        let _ = std::fs::remove_file(active_path);
        let remaining = list_ask_sessions(paths, project_slug);
        if let Some(next) = remaining.first() {
            set_active_ask_session(paths, project_slug, &next.id)?;
        }
    }
    Ok(())
}

pub fn load_ask_messages(
    paths: &KnowledgePaths,
    project_slug: &str,
    session_id: &str,
) -> crate::error::Result<serde_json::Value> {
    let path = session_messages_path(&paths.ask_sessions_dir(project_slug), session_id);
    if !path.is_file() {
        return Ok(serde_json::json!([]));
    }
    let raw = std::fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::json!([]));
    Ok(value)
}

fn last_replied_date_from_messages(messages: &serde_json::Value) -> String {
    if let Some(arr) = messages.as_array() {
        for msg in arr.iter().rev() {
            if msg.get("role").and_then(|r| r.as_str()) == Some("assistant") {
                if let Some(ts) = msg.get("timestamp").and_then(|t| t.as_u64()) {
                    if let Some(dt) = chrono::DateTime::from_timestamp_millis(ts as i64) {
                        return dt.format("%Y-%m-%d").to_string();
                    }
                }
                return today_date_string();
            }
        }
    }
    today_date_string()
}

pub fn save_ask_messages(
    paths: &KnowledgePaths,
    project_slug: &str,
    session_id: &str,
    messages: &serde_json::Value,
    first_question: Option<&str>,
) -> crate::error::Result<AskSessionInfo> {
    let sessions_dir = paths.ask_sessions_dir(project_slug);
    let session_dir = sessions_dir.join(session_id);
    if !session_dir.is_dir() {
        return Err(crate::error::CoreError::InvalidDoc(format!(
            "Ask session not found: {session_id}"
        )));
    }

    let meta_path = session_meta_path(&sessions_dir, session_id);
    let mut meta: SessionMetaFile = std::fs::read_to_string(&meta_path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or(SessionMetaFile {
            id: session_id.to_string(),
            title: session_id.to_string(),
            last_replied_at: today_date_string(),
        });

    if let Some(q) = first_question {
        let title = title_from_question(q, TITLE_CHAR_LIMIT);
        if !title.is_empty() && (meta.title.is_empty() || meta.title == "新对话") {
            meta.title = title;
        }
    }

    meta.last_replied_at = last_replied_date_from_messages(messages);
    std::fs::write(&meta_path, serde_json::to_string_pretty(&meta)?)?;
    std::fs::write(
        session_messages_path(&sessions_dir, session_id),
        serde_json::to_string_pretty(messages)?,
    )?;
    Ok(meta_to_info(meta))
}

pub fn resolve_ask_session_id(
    paths: &KnowledgePaths,
    project_slug: &str,
    question: &str,
) -> crate::error::Result<String> {
    if let Some(id) = get_active_ask_session(paths, project_slug) {
        let dir = paths.ask_workspace_dir(project_slug, &id);
        if dir.is_dir() {
            return Ok(id);
        }
    }
    let sessions = list_ask_sessions(paths, project_slug);
    if let Some(first) = sessions.first() {
        set_active_ask_session(paths, project_slug, &first.id)?;
        return Ok(first.id.clone());
    }
    let created = create_ask_session(paths, project_slug, question)?;
    Ok(created.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_truncates_by_char_not_byte() {
        assert_eq!(title_from_question("你好世界欢迎使用", 10), "你好世界欢迎使用");
        assert_eq!(title_from_question("你好世界欢迎使用", 4), "你好世界");
        assert_eq!(title_from_question("  hello world  ", 10), "hello worl");
    }
}
