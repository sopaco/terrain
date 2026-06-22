use crate::schema::{DocFrontmatter, DocType, InterfaceMeta, ProjectMeta, RouteMeta};

pub fn project_index_body(meta: &ProjectMeta, tree_summary: &str) -> String {
    format!(
        "# {}\n\n{}\n\n## Repository\n\n`{}`\n\n## Tech stack\n\n{}\n\n## Structure\n\n{}\n",
        meta.name,
        meta.owner
            .as_ref()
            .map(|o| format!("Owner: **{o}**"))
            .unwrap_or_default(),
        meta.repo_path,
        if meta.tech_stack.is_empty() {
            "_Not detected_".into()
        } else {
            meta.tech_stack
                .iter()
                .map(|t| format!("- {t}"))
                .collect::<Vec<_>>()
                .join("\n")
        },
        tree_summary
    )
}

pub fn interface_body(meta: &InterfaceMeta, description: Option<&str>) -> String {
    let desc = description.unwrap_or("Auto-imported from OpenAPI.");
    format!(
        "# {} {}\n\n{desc}\n\n## Request\n\n_See schema in frontmatter / source spec._\n\n## Response\n\n_See schema in frontmatter / source spec._\n",
        meta.method.to_uppercase(),
        meta.path
    )
}

pub fn module_body(name: &str, repo_path: &str) -> String {
    format!(
        "# {name}\n\nAuto-detected module from repository layout.\n\n## Path\n\n`{repo_path}`\n\n## Overview\n\n_Scan-generated stub. Regenerate by re-indexing the project._\n"
    )
}

pub fn module_frontmatter(
    project_slug: &str,
    name: &str,
    repo_path: &str,
) -> DocFrontmatter {
    DocFrontmatter {
        doc_type: DocType::Module,
        project: project_slug.to_string(),
        module: Some(name.to_string()),
        title: Some(name.to_string()),
        source: Some(repo_path.to_string()),
        refs: Vec::new(),
        deps: Vec::new(),
        extra: serde_json::Map::new(),
    }
}

pub fn route_body(meta: &RouteMeta) -> String {
    let mw = if meta.middleware.is_empty() {
        "_None_".into()
    } else {
        meta.middleware
            .iter()
            .map(|m| format!("- {m}"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "# {}\n\n## Handler\n\n`{}`\n\n## Middleware\n\n{mw}\n",
        meta.uri, meta.handler
    )
}

pub fn project_frontmatter(project_slug: &str, meta: &ProjectMeta) -> DocFrontmatter {
    DocFrontmatter {
        doc_type: DocType::Project,
        project: project_slug.to_string(),
        module: None,
        title: Some(meta.name.clone()),
        source: Some(meta.repo_path.clone()),
        refs: Vec::new(),
        deps: Vec::new(),
        extra: serde_json::Map::new(),
    }
}

pub fn interface_frontmatter(
    project_slug: &str,
    module: Option<&str>,
    meta: &InterfaceMeta,
    source: &str,
) -> DocFrontmatter {
    let mut extra = serde_json::Map::new();
    extra.insert(
        "method".into(),
        serde_json::Value::String(meta.method.clone()),
    );
    extra.insert("path".into(), serde_json::Value::String(meta.path.clone()));
    if let Some(v) = &meta.version {
        extra.insert("version".into(), serde_json::Value::String(v.clone()));
    }

    DocFrontmatter {
        doc_type: DocType::Interface,
        project: project_slug.to_string(),
        module: module.map(str::to_string),
        title: Some(format!("{} {}", meta.method.to_uppercase(), meta.path)),
        source: Some(source.to_string()),
        refs: Vec::new(),
        deps: Vec::new(),
        extra,
    }
}

pub fn route_frontmatter(project_slug: &str, meta: &RouteMeta, source: &str) -> DocFrontmatter {
    let mut extra = serde_json::Map::new();
    extra.insert("uri".into(), serde_json::Value::String(meta.uri.clone()));
    extra.insert(
        "handler".into(),
        serde_json::Value::String(meta.handler.clone()),
    );
    if !meta.middleware.is_empty() {
        extra.insert(
            "middleware".into(),
            serde_json::Value::Array(
                meta.middleware
                    .iter()
                    .cloned()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        );
    }

    DocFrontmatter {
        doc_type: DocType::Route,
        project: project_slug.to_string(),
        module: None,
        title: Some(meta.uri.clone()),
        source: Some(source.to_string()),
        refs: Vec::new(),
        deps: Vec::new(),
        extra,
    }
}
