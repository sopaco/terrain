//! Git-following policy for the generated `.terrain/` knowledge directory.
//!
//! `.terrain/` mixes three kinds of files with very different Git needs:
//!
//! - **Hand-maintained knowledge** (`knowledge/`) — must merge line by line.
//! - **LLM-generated documents** (`agent/context.md`, `human/`, `index.md`) —
//!   non-deterministic: two runs over the same commit differ in wording and
//!   structure, so a line-level three-way merge yields a document that is
//!   neither side's output. Keep one side, then regenerate.
//! - **Per-machine derivatives** (`agent/repomix.*`, `agent/meta*.json`,
//!   `.meta/`, `env/`) — large, timestamped, and dependent on the local scan
//!   environment. Never commit them.
//!
//! Encoding that split as a nested `.gitignore` / `.gitattributes` *inside*
//! `.terrain/` keeps the policy self-contained: it is written when the
//! directory is scaffolded (so every project gets it without running env
//! integration), it is committed and therefore travels on clone, it needs no
//! per-developer `git config`, and it leaves the host project's root
//! `.gitignore` untouched.

use std::path::{Path, PathBuf};

/// Bumped whenever the managed bodies below change; older managed copies get rewritten.
pub const GIT_POLICY_VERSION: u32 = 1;

/// Marker that identifies a Terrain-managed file. Removing it opts the file out of updates.
const MARKER_PREFIX: &str = "terrain-git-policy: v";

const GITIGNORE_BODY: &str = r#"
# ── 不入库：本机衍生物 ─────────────────────────────────────────
# 源码索引包及其派生物
agent/repomix.md
agent/repomix.index.json
agent/meta.json
agent/meta-inputs.md
agent/meta-inputs-manifest.json
agent/context-meta.json

# 保鲜 / 同步快照（含时间戳与 baseline git HEAD，逐机器不同）
.meta/

# 本机环境状态（工具路径、集成清单）
env/

# 生成过程的中间态工作区
.litho-agent/
.sdd-agent/

# OS 噪音
.DS_Store

# ── 入库（未被上面规则排除）────────────────────────────────────
# knowledge/   人为维护的私域知识 —— 真正需要逐行合并的部分
# agent/context.md, human/, index.md, project-note.md
#              生成的知识文档；合并策略见同目录 .gitattributes
"#;

const GITATTRIBUTES_BODY: &str = r#"
# ── 生成的知识文档：禁用自动合并 ───────────────────────────────
# 冲突处理：
#   git checkout --ours  .terrain/agent/context.md   # 或 --theirs
#   git add              .terrain/agent/context.md
#   # 合并完成后重新运行 Terrain scan，让资产对齐合并后的代码

agent/context.md    -merge linguist-generated=true
index.md            -merge linguist-generated=true
human/**            -merge linguist-generated=true

# ── 人为维护的知识：走正常三方合并 ─────────────────────────────

knowledge/**        merge
"#;

/// One managed file and what happened to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyOutcome {
    /// File did not exist and was written.
    Created,
    /// A Terrain-managed file from an older policy version was rewritten.
    Upgraded,
    /// Already current; left untouched (mtime preserved).
    UpToDate,
    /// Exists without the Terrain marker — hand-authored, so left alone.
    UserOwned,
}

#[derive(Debug, Clone)]
pub struct PolicyFileStatus {
    /// Path relative to the knowledge root, e.g. `.gitignore`.
    pub name: &'static str,
    pub outcome: PolicyOutcome,
}

/// The two files this module manages, paired with their desired contents.
fn managed_files() -> [(&'static str, String); 2] {
    [
        (
            ".gitignore",
            render(
                ".gitignore",
                "决定 .terrain/ 下哪些文件进版本库（策略说明见下）",
                GITIGNORE_BODY,
            ),
        ),
        (
            ".gitattributes",
            render(
                ".gitattributes",
                "决定 .terrain/ 下生成文档的合并方式（策略说明见下）",
                GITATTRIBUTES_BODY,
            ),
        ),
    ]
}

fn render(name: &str, purpose: &str, body: &str) -> String {
    format!(
        "# {name} — {MARKER_PREFIX}{GIT_POLICY_VERSION} · 由 Terrain 生成并维护\n\
         # {purpose}\n\
         # 需要自定义时删除本文件顶部的 `{MARKER_PREFIX}<n>` 标记行，\n\
         # Terrain 就不再覆盖它（Terrain 也不会再为你升级策略）。\n\
         {body}"
    )
}

/// Parse the managed policy version out of an existing file, if it carries the marker.
fn marker_version(content: &str) -> Option<u32> {
    content.lines().take(10).find_map(|line| {
        let rest = line.split_once(MARKER_PREFIX)?.1;
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        digits.parse().ok()
    })
}

/// Write `.terrain/.gitignore` and `.terrain/.gitattributes`, upgrading older
/// managed copies and never clobbering hand-authored ones.
///
/// `knowledge_root` is the repository's `.terrain/` directory.
pub fn ensure_git_policy(knowledge_root: &Path) -> std::io::Result<Vec<PolicyFileStatus>> {
    std::fs::create_dir_all(knowledge_root)?;
    let mut report = Vec::new();

    for (name, desired) in managed_files() {
        let path = knowledge_root.join(name);
        let outcome = match std::fs::read_to_string(&path) {
            Err(_) => {
                std::fs::write(&path, &desired)?;
                PolicyOutcome::Created
            }
            Ok(existing) => match marker_version(&existing) {
                None => PolicyOutcome::UserOwned,
                Some(v) if v >= GIT_POLICY_VERSION && existing == desired => {
                    PolicyOutcome::UpToDate
                }
                Some(v) if v > GIT_POLICY_VERSION => PolicyOutcome::UpToDate,
                Some(_) => {
                    std::fs::write(&path, &desired)?;
                    PolicyOutcome::Upgraded
                }
            },
        };
        report.push(PolicyFileStatus { name, outcome });
    }

    Ok(report)
}

/// True when both managed files exist at the current policy version, or have
/// been deliberately taken over by the user.
pub fn git_policy_ready(knowledge_root: &Path) -> bool {
    managed_files().iter().all(|(name, _)| {
        match std::fs::read_to_string(knowledge_root.join(name)) {
            Err(_) => false,
            // No marker means the user owns the file; that counts as configured.
            Ok(existing) => marker_version(&existing).is_none_or(|v| v >= GIT_POLICY_VERSION),
        }
    })
}

/// Paths whose mtime should invalidate env-status caches.
pub fn git_policy_paths(knowledge_root: &Path) -> Vec<PathBuf> {
    managed_files()
        .iter()
        .map(|(name, _)| knowledge_root.join(name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "terrain-git-policy-{tag}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn creates_both_files_and_reports_ready() {
        let root = tmp("create");
        assert!(!git_policy_ready(&root));

        let report = ensure_git_policy(&root).unwrap();
        assert_eq!(report.len(), 2);
        assert!(report.iter().all(|r| r.outcome == PolicyOutcome::Created));
        assert!(git_policy_ready(&root));

        let ignore = std::fs::read_to_string(root.join(".gitignore")).unwrap();
        assert!(ignore.contains("agent/repomix.index.json"));
        assert!(ignore.contains("agent/meta.json"));
        assert!(ignore.contains(".meta/"));
        // knowledge/ must stay committable.
        assert!(!ignore.lines().any(|l| l.trim() == "knowledge/"));

        let attrs = std::fs::read_to_string(root.join(".gitattributes")).unwrap();
        assert!(attrs.contains("agent/context.md    -merge"));
        assert!(attrs.contains("knowledge/**        merge"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn second_run_is_a_no_op() {
        let root = tmp("noop");
        ensure_git_policy(&root).unwrap();
        let report = ensure_git_policy(&root).unwrap();
        assert!(report.iter().all(|r| r.outcome == PolicyOutcome::UpToDate));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn upgrades_older_managed_version() {
        let root = tmp("upgrade");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join(".gitignore"),
            "# .gitignore — terrain-git-policy: v0 · 由 Terrain 生成并维护\nagent/repomix.md\n",
        )
        .unwrap();

        assert!(!git_policy_ready(&root));
        let report = ensure_git_policy(&root).unwrap();
        let ignore_outcome = report
            .iter()
            .find(|r| r.name == ".gitignore")
            .map(|r| r.outcome);
        assert_eq!(ignore_outcome, Some(PolicyOutcome::Upgraded));

        let ignore = std::fs::read_to_string(root.join(".gitignore")).unwrap();
        assert!(ignore.contains("agent/meta.json"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn never_clobbers_a_hand_authored_file() {
        let root = tmp("userowned");
        std::fs::create_dir_all(&root).unwrap();
        let mine = "# my own rules\nhuman/\n";
        std::fs::write(root.join(".gitignore"), mine).unwrap();

        let report = ensure_git_policy(&root).unwrap();
        assert_eq!(
            report
                .iter()
                .find(|r| r.name == ".gitignore")
                .map(|r| r.outcome),
            Some(PolicyOutcome::UserOwned)
        );
        assert_eq!(
            std::fs::read_to_string(root.join(".gitignore")).unwrap(),
            mine
        );
        // A user-owned file still counts as configured, so status stays green.
        assert!(git_policy_ready(&root));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn marker_version_parses_and_rejects() {
        assert_eq!(marker_version("# x terrain-git-policy: v3 · y"), Some(3));
        assert_eq!(marker_version("# nothing here\nagent/repomix.md"), None);
    }
}
