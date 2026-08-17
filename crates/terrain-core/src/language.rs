//! Language / i18n support.
//!
//! The user picks a [`LanguageSetting`] in the GUI settings panel or via the CLI
//! (`terrain settings`); `system` (the default) resolves to the OS locale.
//! The resolved [`ResolvedLanguage`] drives three things:
//!
//! 1. GUI / CLI / tray copy (`LanguageSetting` is exported to TypeScript).
//! 2. The language of generated knowledge assets (agent context, Litho human
//!    docs, SDD documents) via [`ResolvedLanguage::asset_language_directive`].
//! 3. The language of agent replies via [`ResolvedLanguage::reply_language_directive`].

use std::sync::RwLock;

use serde::{Deserialize, Serialize};

static LANGUAGE_CACHE: RwLock<Option<ResolvedLanguage>> = RwLock::new(None);

/// Persisted language preference (`~/.terrain/settings.json`, `language` field).
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LanguageSetting {
    /// Follow the operating-system locale (default).
    #[default]
    #[serde(rename = "system")]
    #[cfg_attr(feature = "ts-export", ts(rename = "system"))]
    System,
    #[serde(rename = "zh-CN", alias = "zh", alias = "zh_cn", alias = "zh-hans")]
    #[cfg_attr(feature = "ts-export", ts(rename = "zh-CN"))]
    ZhCn,
    #[serde(rename = "en", alias = "en-US", alias = "en_us", alias = "en-GB")]
    #[cfg_attr(feature = "ts-export", ts(rename = "en"))]
    En,
}

impl LanguageSetting {
    pub fn as_str(self) -> &'static str {
        match self {
            LanguageSetting::System => "system",
            LanguageSetting::ZhCn => "zh-CN",
            LanguageSetting::En => "en",
        }
    }

    /// Parse user/CLI input such as `system`, `zh-CN`, `zh`, `en`, `en-US`.
    pub fn parse(value: &str) -> Option<Self> {
        let v = value.trim().to_lowercase().replace('_', "-");
        match v.as_str() {
            "system" | "auto" | "default" => Some(LanguageSetting::System),
            "zh" | "zh-cn" | "zh-hans" | "zh-sg" | "zh-tw" | "zh-hk" | "zh-hant" => {
                Some(LanguageSetting::ZhCn)
            }
            "en" | "en-us" | "en-gb" => Some(LanguageSetting::En),
            _ => None,
        }
    }
}

/// Concrete language used at runtime, after resolving `system` against the OS locale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedLanguage {
    ZhCn,
    En,
}

impl ResolvedLanguage {
    /// BCP-47-ish code used in prompts and metadata.
    pub fn code(self) -> &'static str {
        match self {
            ResolvedLanguage::ZhCn => "zh-CN",
            ResolvedLanguage::En => "en",
        }
    }

    /// Human-readable language name, in the language itself (for prompts).
    pub fn native_name(self) -> &'static str {
        match self {
            ResolvedLanguage::ZhCn => "Simplified Chinese (简体中文)",
            ResolvedLanguage::En => "English",
        }
    }

    pub fn setting(self) -> LanguageSetting {
        match self {
            ResolvedLanguage::ZhCn => LanguageSetting::ZhCn,
            ResolvedLanguage::En => LanguageSetting::En,
        }
    }

    /// `target_language` value understood by the Litho documents skill
    /// (`zh` → `1.概述.md`, `en` → `1.Overview.md`, …).
    pub fn litho_target_language(self) -> &'static str {
        match self {
            ResolvedLanguage::ZhCn => "zh",
            ResolvedLanguage::En => "en",
        }
    }

    /// File name of the first Litho human doc (`human/1.… .md`).
    pub fn litho_overview_filename(self) -> &'static str {
        match self {
            ResolvedLanguage::ZhCn => "1.概述.md",
            ResolvedLanguage::En => "1.Overview.md",
        }
    }

    /// Core Litho deliverables under `human/` for this language
    /// (mirrors the `target_language` table in the litho-documents skill).
    pub fn litho_required_files(self) -> [&'static str; 5] {
        match self {
            ResolvedLanguage::ZhCn => [
                "1.概述.md",
                "2.架构.md",
                "3.工作流.md",
                "5.边界接口.md",
                "6.数据库概览.md",
            ],
            ResolvedLanguage::En => [
                "1.Overview.md",
                "2.Architecture.md",
                "3.Workflows.md",
                "5.Boundaries-Interfaces.md",
                "6.Database-Overview.md",
            ],
        }
    }

    /// Bullet list of the full Litho doc set, for generation prompts.
    pub fn litho_file_listing(self) -> String {
        let [one, two, three, five, six] = self.litho_required_files();
        format!(
            "- {one}\n- {two}\n- {three}\n- 4.Deep-Exploration/{{module}}.md (one per module under workspace/modules/)\n- {five}\n- {six}"
        )
    }

    /// Pick a user-facing string by language (`tr("中文", "English")`).
    pub fn tr<'a>(self, zh: &'a str, en: &'a str) -> &'a str {
        match self {
            ResolvedLanguage::ZhCn => zh,
            ResolvedLanguage::En => en,
        }
    }

    /// Instruction injected into knowledge-asset generation prompts
    /// (agent context, Litho human docs, SDD documents).
    pub fn asset_language_directive(self) -> String {
        format!(
            "LANGUAGE: Write ALL generated documents entirely in {} ({}). \
             Headings, prose, tables and diagram labels must all use this language; \
             keep code identifiers, commands, file paths and proper nouns as-is.",
            self.native_name(),
            self.code()
        )
    }

    /// Instruction injected into chat/Ask system prompts so the agent replies
    /// in the user's language.
    pub fn reply_language_directive(self) -> String {
        format!(
            "Always reply in {} ({}), regardless of the language of the question \
             or of the knowledge documents you quote. Code, commands and file paths \
             stay in their original form.",
            self.native_name(),
            self.code()
        )
    }

    /// The seven mandatory top-level section headings of `agent/context.md`,
    /// in this language. `context_layers` matches headings bilingually, so
    /// existing Chinese assets keep working after a switch.
    /// First three sections of `agent/context.md` (macro / overview layer).
    pub fn agent_context_macro_sections(self) -> [&'static str; 3] {
        let all = self.agent_context_sections();
        [all[0], all[1], all[2]]
    }

    pub fn agent_context_sections(self) -> [&'static str; 7] {
        match self {
            ResolvedLanguage::ZhCn => [
                "项目概览",
                "架构设计",
                "模块地图",
                "核心流程",
                "技术选型",
                "系统边界",
                "代码映射索引",
            ],
            ResolvedLanguage::En => [
                "Project Overview",
                "Architecture",
                "Module Map",
                "Core Flows",
                "Tech Stack",
                "System Boundaries",
                "Code Map Index",
            ],
        }
    }
}

/// Best-effort OS locale detection (CoreFoundation on macOS, WinAPI on
/// Windows, `LC_ALL`/`LC_MESSAGES`/`LANG` elsewhere).
pub fn detect_system_language() -> ResolvedLanguage {
    let raw = sys_locale::get_locale()
        .or_else(|| std::env::var("LC_ALL").ok())
        .or_else(|| std::env::var("LC_MESSAGES").ok())
        .or_else(|| std::env::var("LANG").ok())
        .unwrap_or_default();
    language_from_locale(&raw)
}

/// Map a locale string (`zh-CN`, `zh_CN.UTF-8`, `en_US`, …) to a supported language.
pub fn language_from_locale(locale: &str) -> ResolvedLanguage {
    let l = locale.trim().to_lowercase().replace('_', "-");
    if l.starts_with("zh") {
        ResolvedLanguage::ZhCn
    } else {
        ResolvedLanguage::En
    }
}

/// Resolve a persisted setting to a concrete language.
pub fn resolve_language(setting: LanguageSetting) -> ResolvedLanguage {
    match setting {
        LanguageSetting::System => detect_system_language(),
        LanguageSetting::ZhCn => ResolvedLanguage::ZhCn,
        LanguageSetting::En => ResolvedLanguage::En,
    }
}

/// Clear the in-process language cache (call after saving settings).
pub fn invalidate_language_cache() {
    if let Ok(mut guard) = LANGUAGE_CACHE.write() {
        *guard = None;
    }
}

/// All `agent/context.md` `##` section titles across supported languages.
pub fn all_agent_context_section_titles() -> Vec<&'static str> {
    let mut out = Vec::with_capacity(14);
    for lang in [ResolvedLanguage::ZhCn, ResolvedLanguage::En] {
        out.extend_from_slice(lang.agent_context_sections());
    }
    out
}

/// Whether a parsed `##` heading belongs to the macro (overview) layer.
pub fn is_macro_context_section(title: &str) -> bool {
    let t = title.to_lowercase();
    for lang in [ResolvedLanguage::ZhCn, ResolvedLanguage::En] {
        for section in lang.agent_context_macro_sections() {
            if t.contains(&section.to_lowercase()) {
                return true;
            }
        }
    }
    ["modules", "module"]
        .iter()
        .any(|k| t.contains(k))
}

/// Resolve the *current* effective language: settings file → system locale.
pub fn current_language() -> ResolvedLanguage {
    if let Ok(guard) = LANGUAGE_CACHE.read() {
        if let Some(lang) = *guard {
            return lang;
        }
    }
    let resolved = resolve_current_language_from_disk();
    if let Ok(mut guard) = LANGUAGE_CACHE.write() {
        *guard = Some(resolved);
    }
    resolved
}

fn resolve_current_language_from_disk() -> ResolvedLanguage {
    let setting = crate::settings::load_model_settings()
        .map(|s| s.language)
        .unwrap_or_default();
    resolve_language(setting)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_user_input() {
        assert_eq!(LanguageSetting::parse("system"), Some(LanguageSetting::System));
        assert_eq!(LanguageSetting::parse("zh-CN"), Some(LanguageSetting::ZhCn));
        assert_eq!(LanguageSetting::parse("zh"), Some(LanguageSetting::ZhCn));
        assert_eq!(LanguageSetting::parse("EN_us"), Some(LanguageSetting::En));
        assert_eq!(LanguageSetting::parse("fr"), None);
    }

    #[test]
    fn locale_mapping() {
        assert_eq!(language_from_locale("zh-CN"), ResolvedLanguage::ZhCn);
        assert_eq!(language_from_locale("zh_CN.UTF-8"), ResolvedLanguage::ZhCn);
        assert_eq!(language_from_locale("en_US"), ResolvedLanguage::En);
        assert_eq!(language_from_locale("ja-JP"), ResolvedLanguage::En);
        assert_eq!(language_from_locale(""), ResolvedLanguage::En);
    }

    #[test]
    fn setting_round_trips_json() {
        let json = serde_json::to_string(&LanguageSetting::ZhCn).unwrap();
        assert_eq!(json, "\"zh-CN\"");
        let back: LanguageSetting = serde_json::from_str("\"zh-CN\"").unwrap();
        assert_eq!(back, LanguageSetting::ZhCn);
        let legacy: LanguageSetting = serde_json::from_str("\"zh\"").unwrap();
        assert_eq!(legacy, LanguageSetting::ZhCn);
    }
}
