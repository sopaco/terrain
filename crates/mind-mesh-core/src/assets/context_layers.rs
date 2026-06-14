//! Tiered agent/context.md: macro overview vs section reads vs repomix code.

use serde::Serialize;

/// Hard cap when persisting generated context (16 KiB characters).
pub const AGENT_CONTEXT_SAVE_MAX_CHARS: usize = 16 * 1024;

/// Macro layer injected into Ask prompt (overview + section index).
pub const AGENT_CONTEXT_ASK_OVERVIEW_MAX_CHARS: usize = 4_500;

/// Max chars returned per `read_agent_context(section=…)` call.
pub const AGENT_CONTEXT_TOOL_SECTION_MAX_CHARS: usize = 3_500;

#[derive(Debug, Clone, Serialize)]
pub struct ContextSection {
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextOverview {
    pub section_titles: Vec<String>,
    pub macro_markdown: String,
    pub total_chars: usize,
    pub overview_chars: usize,
    pub size_capped: bool,
    pub on_demand_hint: String,
}

/// Split markdown body into `##` sections.
pub fn split_context_sections(body: &str) -> Vec<ContextSection> {
    let mut out = Vec::new();
    let mut current_title = String::new();
    let mut current_lines: Vec<&str> = Vec::new();

    for line in body.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            if !current_title.is_empty() || !current_lines.is_empty() {
                out.push(ContextSection {
                    title: current_title.clone(),
                    body: current_lines.join("\n").trim().to_string(),
                });
            }
            current_title = heading.trim().to_string();
            current_lines.clear();
        } else {
            current_lines.push(line);
        }
    }

    if !current_title.is_empty() || !current_lines.is_empty() {
        out.push(ContextSection {
            title: current_title,
            body: current_lines.join("\n").trim().to_string(),
        });
    }

    if out.is_empty() && !body.trim().is_empty() {
        out.push(ContextSection {
            title: String::new(),
            body: body.trim().to_string(),
        });
    }

    out
}

fn is_macro_section(title: &str) -> bool {
    let t = title.to_lowercase();
    [
        "项目概览",
        "架构设计",
        "模块地图",
        "overview",
        "architecture",
        "module map",
        "modules",
        "module",
    ]
    .iter()
    .any(|k| t.contains(k))
}

fn truncate_chars(text: &str, max: usize) -> (String, bool) {
    if text.chars().count() <= max {
        return (text.to_string(), false);
    }
    let cut: String = text.chars().take(max).collect();
    (format!("{cut}\n\n…"), true)
}

fn render_section(title: &str, body: &str) -> String {
    if title.is_empty() {
        return body.to_string();
    }
    format!("## {title}\n{body}")
}

/// Build macro-layer overview: key sections + index of on-demand sections.
pub fn build_context_overview(body: &str, max_chars: usize) -> ContextOverview {
    let sections = split_context_sections(body);
    let section_titles: Vec<String> = sections
        .iter()
        .map(|s| s.title.clone())
        .filter(|t| !t.is_empty())
        .collect();

    let mut macro_parts: Vec<String> = Vec::new();
    let mut used = 0usize;
    let mut size_capped = false;

    for section in &sections {
        if !is_macro_section(&section.title) {
            continue;
        }
        let block = render_section(&section.title, &section.body);
        let block_len = block.chars().count();
        if used + block_len <= max_chars {
            macro_parts.push(block);
            used += block_len;
        } else {
            let remaining = max_chars.saturating_sub(used);
            if remaining > 80 {
                let (part, _) = truncate_chars(&block, remaining);
                macro_parts.push(part);
                used = max_chars;
            }
            size_capped = true;
            break;
        }
    }

    if macro_parts.is_empty() {
        for section in sections.iter().take(2) {
            let block = render_section(&section.title, &section.body);
            let (part, capped) = truncate_chars(&block, max_chars.saturating_sub(used));
            if part.trim().is_empty() {
                break;
            }
            macro_parts.push(part);
            used += macro_parts.last().map(|s| s.chars().count()).unwrap_or(0);
            size_capped |= capped;
            if used >= max_chars {
                break;
            }
        }
    } else {
        size_capped |= body.chars().count() > max_chars;
    }

    let on_demand: Vec<String> = sections
        .iter()
        .filter(|s| !s.title.is_empty() && !is_macro_section(&s.title))
        .map(|s| format!("  - \"{}\"", s.title))
        .collect();

    let on_demand_hint = if on_demand.is_empty() {
        "For deeper architecture sections, call read_agent_context(section=\"<heading>\"). \
         For implementation/code, use grep_agent_pack → read_agent_pack_file."
            .to_string()
    } else {
        format!(
            "Meso layer — call read_agent_context(section=\"<heading>\") for:\n{}\n\
Micro layer — grep_agent_pack → read_agent_pack_file for source code (≤150 lines per call).",
            on_demand.join("\n")
        )
    };

    let index_block = if section_titles.is_empty() {
        String::new()
    } else {
        format!(
            "### Section index\n{}\n\n",
            section_titles
                .iter()
                .map(|t| format!("- {t}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    let macro_markdown = format!(
        "{index_block}{}\n\n{on_demand_hint}",
        macro_parts.join("\n\n")
    );

    ContextOverview {
        section_titles,
        overview_chars: macro_markdown.chars().count(),
        total_chars: body.chars().count(),
        size_capped,
        macro_markdown,
        on_demand_hint,
    }
}

/// Find a section by case-insensitive substring match on the heading.
pub fn extract_context_section<'a>(
    sections: &'a [ContextSection],
    query: &str,
) -> Option<&'a ContextSection> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return None;
    }
    sections
        .iter()
        .find(|s| s.title.to_lowercase().contains(&q) || q.contains(&s.title.to_lowercase()))
}

/// Enforce max size at save time — trim each section proportionally.
pub fn enforce_context_max_size(body: &str, max_chars: usize) -> (String, bool) {
    if body.chars().count() <= max_chars {
        return (body.to_string(), false);
    }

    let sections = split_context_sections(body);
    if sections.is_empty() {
        return truncate_chars(body, max_chars);
    }

    let header_budget: usize = sections
        .iter()
        .map(|s| {
            if s.title.is_empty() {
                0
            } else {
                s.title.chars().count() + 4
            }
        })
        .sum();
    let body_budget = max_chars.saturating_sub(header_budget + sections.len() * 2);
    let per_section = body_budget / sections.len().max(1);

    let mut out = String::new();
    let mut any_trimmed = false;

    for section in &sections {
        let budget = per_section.max(60);
        let (body_part, trimmed) = truncate_chars(&section.body, budget);
        any_trimmed |= trimmed;
        if !section.title.is_empty() {
            out.push_str(&format!("## {}\n", section.title));
        }
        out.push_str(&body_part);
        out.push('\n');
    }

    let result = out.trim().to_string();
    let (result, hard_trim) = if result.chars().count() > max_chars {
        truncate_chars(&result, max_chars)
    } else {
        (result, false)
    };

    (result, any_trimmed || hard_trim || body.chars().count() > max_chars)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
## 项目概览
Short overview.

## 架构设计
Layer A -> B.

## 模块地图
| M | P |
| a | src/a |

## 核心流程
1. step one
2. step two

## 代码映射索引
| C | L |
| x | src/x.rs |
";

    #[test]
    fn splits_sections() {
        let s = split_context_sections(SAMPLE);
        assert_eq!(s.len(), 5);
        assert_eq!(s[0].title, "项目概览");
    }

    #[test]
    fn overview_prefers_macro_sections() {
        let ov = build_context_overview(SAMPLE, 10_000);
        assert!(ov.macro_markdown.contains("项目概览"));
        assert!(ov.macro_markdown.contains("模块地图"));
        assert!(ov.on_demand_hint.contains("核心流程"));
    }

    #[test]
    fn extracts_section_by_substring() {
        let sections = split_context_sections(SAMPLE);
        let hit = extract_context_section(&sections, "流程").unwrap();
        assert!(hit.title.contains("核心流程"));
    }

    #[test]
    fn enforces_max_size() {
        let (out, capped) = enforce_context_max_size(SAMPLE, 100);
        assert!(capped);
        assert!(out.chars().count() <= 150);
    }
}
