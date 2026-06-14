use std::path::Path;

use regex::Regex;

use super::catalog::{load_catalog, resolve_fragment_path, env_catalog_root};
use crate::error::{CoreError, Result};

const BEGIN_RE: &str = r"<!-- mind-mesh:begin (\S+) v(\d+) -->";

pub fn agents_md_ready(repo: &Path) -> bool {
    let path = repo.join("AGENTS.md");
    if !path.is_file() {
        return false;
    }
    let Ok(content) = std::fs::read_to_string(&path) else {
        return false;
    };
    content.contains("<!-- mind-mesh:begin knowledge-guide")
}

pub fn patch_agents_md(repo: &Path) -> Result<String> {
    let catalog = load_catalog()?;
    let catalog_root = env_catalog_root();
    let path = repo.join("AGENTS.md");

    let mut content = if path.is_file() {
        std::fs::read_to_string(&path)?
    } else {
        String::from(
            "# Agents Guide\n\n\
             This file guides AI coding agents working in this repository.\n\n",
        )
    };

    for block in &catalog.agents_md_blocks {
        let fragment_path = resolve_fragment_path(&catalog_root, &block.fragment);
        let fragment = std::fs::read_to_string(&fragment_path).map_err(|e| {
            CoreError::InvalidDoc(format!(
                "missing AGENTS.md fragment {}: {e}",
                fragment_path.display()
            ))
        })?;
        let wrapped = format!(
            "<!-- mind-mesh:begin {} v{} -->\n{}\n<!-- mind-mesh:end {} -->",
            block.id, block.version, fragment.trim(), block.id
        );
        content = replace_or_append_block(&content, &block.id, block.version, &wrapped);
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, &content)?;
    Ok(path.display().to_string())
}

fn replace_or_append_block(content: &str, id: &str, _version: u32, wrapped: &str) -> String {
    let begin_re = Regex::new(&format!(
        r"<!-- mind-mesh:begin {id} v\d+ -->[\s\S]*?<!-- mind-mesh:end {id} -->"
    ))
    .expect("valid regex");

    if begin_re.is_match(content) {
        return begin_re.replace(content, wrapped).to_string();
    }

    let header_re = Regex::new(BEGIN_RE).expect("valid regex");
    if header_re.is_match(content) {
        let mut out = content.to_string();
        out.push_str("\n\n");
        out.push_str(wrapped);
        return out;
    }

    if content.trim().is_empty() {
        return wrapped.to_string();
    }

    let mut out = content.to_string();
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push('\n');
    out.push_str(wrapped);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_existing_block() {
        let content = "intro\n\n<!-- mind-mesh:begin foo v1 -->\nold\n<!-- mind-mesh:end foo -->\n";
        let wrapped = "<!-- mind-mesh:begin foo v2 -->\nnew\n<!-- mind-mesh:end foo -->";
        let out = replace_or_append_block(content, "foo", 2, wrapped);
        assert!(out.contains("new"));
        assert!(!out.contains("old"));
    }
}
