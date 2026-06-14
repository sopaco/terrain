use regex::Regex;
use std::sync::LazyLock;

const THINK_OPEN: &str = concat!("<", "think", ">");
const THINK_CLOSE: &str = concat!("<", "/", "think", ">");

static MARKDOWN_FENCE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)^```(?:markdown|md)\s*\n(.*)\n```\s*$")
        .expect("markdown fence regex")
});

static HEADING_BREAK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([。；;：:])(### )").expect("heading break regex")
});

static BOLD_SECTION_BREAK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([。`])(\*\*\d+\.\d+)").expect("bold section break regex")
});

static BOLD_LABEL_BREAK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([。])(\*\*[^*]+\*\*：)").expect("bold label break regex")
});

static NUMBERED_LIST_BREAK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\*\*)(\d+\.\s+\*\*)").expect("numbered list break regex")
});

static NUMBERED_LIST_AFTER_LABEL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([：:])(\d+\.\s+\*\*)").expect("numbered list after label regex")
});

static PAREN_CJK_BREAK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(`\))([\x{4e00}-\x{9fff}])").expect("paren cjk break regex")
});

static PAREN_BOLD_CJK_BREAK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\))(\*\*[^*]+\*\*)([\x{4e00}-\x{9fff}])").expect("paren bold cjk break regex")
});

static BOLD_AFTER_PAREN_BREAK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\))(\*\*[^*]+:\*\*)").expect("bold after paren break regex")
});

static STEPS_LABEL_BREAK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\*\*[^*]+:\*\*)(\d+\.)").expect("steps label break regex")
});

static H3_TITLE_BOLD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(### [^*]+?)(\*\*[^*]+：[^*]*\*\*)").expect("h3 title bold regex")
});

static BOLD_BEFORE_CJK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\*\*[^*]+\*\*)([\x{4e00}-\x{9fff}])").expect("bold before cjk regex")
});

static SENTENCE_BREAK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([。！？])([\x{4e00}-\x{9fff}看当从不这无每])").expect("sentence break regex")
});

static EMPHASIS_CJK_BREAK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\*)([从当不无这每])").expect("emphasis cjk break regex")
});

static CARET_EMPHASIS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\*\^([^^*]+)\^\*").expect("caret emphasis regex")
});

static LONE_CARET_EMPHASIS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\^([^^*]+)\^").expect("lone caret emphasis regex")
});

/// Break inline `##` headings glued to prior sentences (common with local models).
static INLINE_H2_BREAK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([。；;：:.!?])\s*(## )").expect("inline h2 break regex")
});

static INLINE_H2_AFTER_WORD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([\x{4e00}-\x{9fff}a-zA-Z0-9`\)])\s*(## )").expect("inline h2 after word regex")
});

/// Break inline `###`–`######` headings glued to prior text on the same line.
static INLINE_SUBHEADING_BREAK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([^\n\r])(#{3,6} )").expect("inline subheading break regex")
});

/// Break any `##`–`######` heading not already at line start (LLM flattened output).
static INLINE_HEADING_BREAK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([^\n\r#])(#{2,6} )").expect("inline heading break regex")
});

/// Heading title glued to body text (e.g. `## 标题MindMesh **bold**`).
static HEADING_GLUE_BODY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(#{1,6} [^\n]+[\x{4e00}-\x{9fff}])([A-Za-z])").expect("heading glue body regex")
});

/// Fenced code block glued to prior prose.
static INLINE_FENCE_BREAK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([^\n\r`])(```[\w]+)").expect("inline fence break regex")
});

static INLINE_BARE_FENCE_BREAK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([^\n\r`])(```)([^\w\n\r])").expect("inline bare fence break regex")
});

/// Fenced code block immediately followed by a heading.
static FENCE_BEFORE_HEADING_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"```(#{1,6} )").expect("fence before heading regex")
});

/// Numbered list item glued to prior sentence (`…标准2. **Item**`).
static GLUED_NUMBERED_LIST_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([^\n\d])(\d+\.\s+\*\*)").expect("glued numbered list regex")
});

/// Table rows glued together (`| a | b || c | d |`).
static GLUED_TABLE_ROW_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\|\|").expect("glued table row regex")
});

/// Newline after fenced-code language tag (` ```toml# ...`).
static FENCE_LANG_NEWLINE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(```[\w]+)([^\n\r\w])").expect("fence lang newline regex")
});

/// Body text glued after a heading that ends with `)`.
static HEADING_PAREN_BODY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(#{1,6} [^\n]+?\))([^\n#\s\d\-])").expect("heading paren body regex")
});

/// Markdown table starting on the same line as prose.
static TABLE_AFTER_COLON_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([：:])(\| )").expect("table after colon regex")
});

/// Unordered list glued to prior heading text on the same line.
static HEADING_LIST_BREAK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(#{1,6} [^\n]+?)(-\s*`)").expect("heading list break regex")
});

/// Unordered list item glued to prior inline text (`Provider- \`code\``).
static GLUED_UNORDERED_LIST_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([^\n\r])(-\s*`)").expect("glued unordered list regex")
});

static PROMOTE_REQUIRED_H3_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^### (项目概览|架构设计|模块地图|核心流程|技术选型|系统边界|代码映射索引)\s*$")
        .expect("promote required h3 regex")
});

const REQUIRED_CONTEXT_SECTIONS: &[&str] = &[
    "项目概览",
    "架构设计",
    "模块地图",
    "核心流程",
    "技术选型",
    "系统边界",
    "代码映射索引",
];

fn break_before_known_sections(text: &str) -> String {
    let mut out = text.to_string();
    for title in REQUIRED_CONTEXT_SECTIONS {
        let before = format!(r"([^\n])(## {title})");
        if let Ok(re) = Regex::new(&before) {
            out = re.replace_all(&out, "$1\n\n$2").into_owned();
        }
        // `## 标题` glued to first content char on the same line
        let glued = format!(r"## {title}([^\n#])");
        if let Ok(re) = Regex::new(&glued) {
            out = re.replace_all(&out, &format!("## {title}\n\n$1")).into_owned();
        }
    }
    out
}

/// Strip model reasoning / thinking blocks from text.
pub fn strip_model_reasoning(text: &str) -> String {
    let mut out = text.to_string();

    loop {
        let Some(start) = find_ignore_case(&out, THINK_OPEN) else {
            break;
        };
        if let Some(rel_end) = find_ignore_case(&out[start..], THINK_CLOSE) {
            let remove_end = start + rel_end + THINK_CLOSE.len();
            out.replace_range(start..remove_end, "");
        } else {
            out.replace_range(start..start + THINK_OPEN.len(), "");
            break;
        }
    }

    strip_qwen_reasoning_prefix(&out)
}

fn strip_qwen_reasoning_prefix(text: &str) -> String {
    let trimmed = text.trim_start();
    let leading = text.len() - trimmed.len();
    if !trimmed.starts_with("`\n") {
        return text.trim().to_string();
    }

    let bytes = trimmed.as_bytes();
    let mut i = 2;
    while i + 1 < bytes.len() {
        if bytes[i] == b'`' && bytes[i + 1] == b'`' {
            let end = i + 2;
            if end < bytes.len() && bytes[end] == b'`' {
                i += 1;
                continue;
            }
            let suffix = trimmed[end..].trim_start();
            return format!("{}{suffix}", &text[..leading]);
        }
        i += 1;
    }

    text.trim().to_string()
}

/// Unwrap a single outer ```markdown / ```md fenced block if present.
pub fn unwrap_markdown_fence(text: &str) -> String {
    let trimmed = text.trim();
    if let Some(caps) = MARKDOWN_FENCE_RE.captures(trimmed) {
        return caps.get(1).map(|m| m.as_str().trim()).unwrap_or(trimmed).to_string();
    }
    trimmed.to_string()
}

/// Drop preamble before the first markdown heading.
pub fn extract_markdown_body(text: &str) -> String {
    let text = text.trim();
    // Agent context: start at the first required section, not a later `\n##`.
    for title in REQUIRED_CONTEXT_SECTIONS {
        let marker = format!("## {title}");
        if let Some(idx) = text.find(&marker) {
            return text[idx..].to_string();
        }
    }
    if let Some(idx) = text.find("\n## ") {
        return text[idx + 1..].trim_start().to_string();
    }
    if text.starts_with("## ") {
        return text.to_string();
    }
    // Skip a lone document title (`# …`) before real `##` sections.
    if text.starts_with("# ") {
        if let Some(idx) = text.find("\n## ") {
            return text[idx + 1..].trim_start().to_string();
        }
    }
    // Inline preamble before the first `##` on the same line (local LLM output).
    if let Some(idx) = text.find("## ") {
        return text[idx..].to_string();
    }
    text.to_string()
}

/// Fix headings, lists, fences, and tables run together on one line.
/// Always applied — local LLM providers often return zero newlines.
pub fn repair_inline_section_headings(text: &str) -> String {
    let mut out = INLINE_H2_BREAK_RE.replace_all(text, "$1\n\n$2").into_owned();
    out = INLINE_H2_AFTER_WORD_RE
        .replace_all(&out, "$1\n\n$2")
        .into_owned();
    out = INLINE_HEADING_BREAK_RE
        .replace_all(&out, "$1\n\n$2")
        .into_owned();
    out = INLINE_SUBHEADING_BREAK_RE
        .replace_all(&out, "$1\n\n$2")
        .into_owned();
    out = HEADING_GLUE_BODY_RE
        .replace_all(&out, "$1\n\n$2")
        .into_owned();
    out = INLINE_FENCE_BREAK_RE
        .replace_all(&out, "$1\n\n$2")
        .into_owned();
    out = INLINE_BARE_FENCE_BREAK_RE
        .replace_all(&out, "$1\n\n$2$3")
        .into_owned();
    out = FENCE_BEFORE_HEADING_RE
        .replace_all(&out, "```\n\n$1")
        .into_owned();
    out = GLUED_NUMBERED_LIST_RE
        .replace_all(&out, "$1\n\n$2")
        .into_owned();
    out = GLUED_TABLE_ROW_RE.replace_all(&out, "|\n|").into_owned();
    out = FENCE_LANG_NEWLINE_RE.replace_all(&out, "$1\n$2").into_owned();
    out = HEADING_PAREN_BODY_RE
        .replace_all(&out, "$1\n\n$2")
        .into_owned();
    out = TABLE_AFTER_COLON_RE
        .replace_all(&out, "$1\n\n$2")
        .into_owned();
    out = HEADING_LIST_BREAK_RE
        .replace_all(&out, "$1\n\n$2")
        .into_owned();
    out = GLUED_UNORDERED_LIST_RE
        .replace_all(&out, "$1\n\n$2")
        .into_owned();
    out = break_before_known_sections(&out);
    PROMOTE_REQUIRED_H3_RE
        .replace_all(&out, "## $1")
        .into_owned()
}

/// Re-insert markdown structure when providers stream a single flattened line.
pub fn repair_flattened_markdown(text: &str) -> String {
    let layout = repair_inline_section_headings(text);
    if layout.chars().filter(|c| *c == '\n').count() >= 3 {
        return layout;
    }

    let mut out = layout;
    out = CARET_EMPHASIS_RE.replace_all(&out, "*$1*").into_owned();
    out = LONE_CARET_EMPHASIS_RE.replace_all(&out, "*$1*").into_owned();
    out = HEADING_BREAK_RE.replace_all(&out, "$1\n\n$2").into_owned();
    out = H3_TITLE_BOLD_RE.replace_all(&out, "$1\n\n$2").into_owned();
    out = BOLD_BEFORE_CJK_RE.replace_all(&out, "$1\n\n$2").into_owned();
    out = SENTENCE_BREAK_RE.replace_all(&out, "$1\n\n$2").into_owned();
    out = EMPHASIS_CJK_BREAK_RE.replace_all(&out, "$1\n\n$2").into_owned();
    out = BOLD_SECTION_BREAK_RE
        .replace_all(&out, "$1\n\n$2")
        .into_owned();
    out = BOLD_LABEL_BREAK_RE.replace_all(&out, "$1\n\n$2").into_owned();
    out = NUMBERED_LIST_BREAK_RE.replace_all(&out, "$1\n$2").into_owned();
    out = NUMBERED_LIST_AFTER_LABEL_RE
        .replace_all(&out, "$1\n$2")
        .into_owned();
    out = out.replace("：```rustpub ", "：\n\n```rust\npub ");
    out = out.replace("```rustpub struct", "```rust\npub struct");
    out = out.replace("{ pub ", "{\n pub ");
    out = out.replace(", pub ", ",\n pub ");
    out = out.replace("}```", "}\n```\n\n");
    out = PAREN_CJK_BREAK_RE.replace_all(&out, "$1\n\n$2").into_owned();
    out = PAREN_BOLD_CJK_BREAK_RE
        .replace_all(&out, "$1\n\n$2\n\n$3")
        .into_owned();
    out = BOLD_AFTER_PAREN_BREAK_RE
        .replace_all(&out, "$1\n\n$2")
        .into_owned();
    out = STEPS_LABEL_BREAK_RE.replace_all(&out, "$1\n$2").into_owned();
    out
}

/// Cleanup for Ask chat replies (no heading-only extraction).
pub fn prepare_chat_markdown(text: &str) -> String {
    let stripped = strip_model_reasoning(text);
    let unwrapped = unwrap_markdown_fence(&stripped);
    repair_flattened_markdown(&unwrapped)
}

/// Full cleanup for persisted or displayed model markdown output.
pub fn prepare_model_markdown(text: &str) -> String {
    let repaired = prepare_chat_markdown(text);
    extract_markdown_body(&repaired)
}

fn find_ignore_case(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }
    let lower_hay = haystack.to_ascii_lowercase();
    let lower_needle = needle.to_ascii_lowercase();
    lower_hay.find(&lower_needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_think_blocks() {
        let open = concat!("<", "think", ">");
        let close = concat!("<", "/", "think", ">");
        let input = format!("Hello{open}secret reasoning{close}World");
        assert_eq!(strip_model_reasoning(&input), "HelloWorld");
    }

    #[test]
    fn strips_qwen_reasoning_prefix() {
        let input = "`\nLet me reason here\n``\n\n## Answer\n\nDone.";
        assert_eq!(strip_model_reasoning(input), "## Answer\n\nDone.");
    }

    #[test]
    fn preserves_inline_code_and_fences() {
        let input = "## Answer\n\nUse `read_file`.\n\n```rust\nfn main() {}\n```";
        assert_eq!(strip_model_reasoning(input), input);
    }

    #[test]
    fn unwraps_markdown_fence() {
        let input = "```markdown\n## Title\n\nBody\n```";
        assert_eq!(unwrap_markdown_fence(input), "## Title\n\nBody");
    }

    #[test]
    fn extracts_from_preamble() {
        let input = "Here is the document.\n\n## 项目概览\n\nSummary";
        assert_eq!(extract_markdown_body(input), "## 项目概览\n\nSummary");
    }

    #[test]
    fn preserves_markdown_fence_prefix() {
        let input = "```markdown\n## Answer\n\nDone.\n```";
        assert_eq!(strip_model_reasoning(input), input);
    }

    #[test]
    fn prepare_model_markdown_end_to_end() {
        let open = concat!("<", "think", ">");
        let close = concat!("<", "/", "think", ">");
        let input = format!(
            "{open}planning{close}\n\n```markdown\n## 项目概览\n\nHello\n```"
        );
        assert_eq!(prepare_model_markdown(&input), "## 项目概览\n\nHello");
    }

    #[test]
    fn repairs_real_flattened_ask_sample() {
        let path = std::path::Path::new(env!("HOME"))
            .join(".mind-mesh/debug/last-ask-raw.md");
        if !path.exists() {
            return;
        }
        let raw = std::fs::read_to_string(&path).expect("read debug ask sample");
        if raw.chars().filter(|c| *c == '\n').count() > 2 {
            return;
        }
        let out = prepare_chat_markdown(&raw);
        assert!(
            out.chars().filter(|c| *c == '\n').count() >= 20,
            "expected repaired newlines, got {} chars in:\n{}",
            out.chars().filter(|c| *c == '\n').count(),
            &out[..out.len().min(400)]
        );
        assert!(out.contains("\n\n## 核心依赖与协议"));
        assert!(out.contains("\n\n1. **ACP"));
        assert!(out.contains("```toml\n"));
    }

    #[test]
    fn repairs_flattened_llm_provider_ask_markdown() {
        let input = "说明如下：## 核心依赖与协议MindMesh **不直接实现** 客户端：1. **One**2. **Two**特性```toml\nkey = 1\n```## 总结Done。";
        let out = prepare_chat_markdown(input);
        assert!(out.contains("\n\n## 核心依赖与协议"), "missing h2 break: {out}");
        assert!(out.contains("\n\nMindMesh"), "missing heading/body break: {out}");
        assert!(out.contains("\n\n1. **One**"), "missing list break: {out}");
        assert!(out.contains("\n\n2. **Two**"), "missing list item break: {out}");
        assert!(out.contains("\n\n```toml"), "missing fence break: {out}");
        assert!(out.contains("\n\n## 总结"), "missing trailing heading break: {out}");
    }

    #[test]
    fn repairs_flattened_ask_markdown() {
        let input = "说明：### 1. 标题模块 (`path`)内容是列表：1.  **One**2.  **Two**";
        let out = repair_flattened_markdown(input);
        assert!(out.contains("\n### 1."));
        assert!(out.contains("\n1.  **One**"));
        assert!(out.contains("\n2.  **Two**"));
    }

    #[test]
    fn repairs_lmstudio_inline_headings() {
        let input = "Based on context.## 项目概览\n\nHello。## 架构设计\n\nWorld";
        let out = prepare_model_markdown(input);
        assert!(out.starts_with("## 项目概览"));
        assert!(out.contains("\n\n## 架构设计"));
        assert!(!out.contains("Based on"));
    }

    #[test]
    fn repairs_glued_section_title() {
        let input = "## 项目概览`repomix-rs` 是一个工具。## 架构设计系统采用分层架构。";
        let out = prepare_model_markdown(input);
        assert!(
            out.contains("## 项目概览"),
            "missing 项目概览: {out}"
        );
        assert!(
            out.contains("## 架构设计"),
            "missing 架构设计: {out}"
        );
        assert!(!out.contains("## 项目概览`"));
    }

    #[test]
    fn promotes_required_h3_sections() {
        let input = "## 架构设计\n\n### 模块地图\n\nTable";
        let out = repair_inline_section_headings(input);
        assert!(out.contains("## 模块地图"));
    }
}
