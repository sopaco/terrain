use mind_mesh_core::{
    KnowledgePaths, KnowledgeSearch, SearchOptions, parse_markdown, parse_markdown_at, read_doc,
    write_doc,
};
use mind_mesh_core::schema::{DocFrontmatter, DocType};

#[test]
fn parse_bare_human_markdown_without_frontmatter() {
    let path = std::path::Path::new("/data/knowledge/projects/repomix-rs/human/1.概述.md");
    let content = "# 项目概述\n\nRepomix is a tool.";
    let (fm, body) = parse_markdown_at(content, Some(path)).unwrap();
    assert_eq!(fm.doc_type, DocType::Human);
    assert_eq!(fm.project, "repomix-rs");
    assert_eq!(fm.title.as_deref(), Some("项目概述"));
    assert!(body.contains("Repomix is a tool."));
}

#[test]
fn parse_and_roundtrip_frontmatter() {
    let content = r#"---
type: interface
project: demo
title: GET /ping
method: get
path: /ping
refs:
  - routes/get-ping.md
---

# GET /ping

Pong.
"#;

    let (fm, body) = parse_markdown(content).unwrap();
    assert_eq!(fm.doc_type, DocType::Interface);
    assert_eq!(fm.project, "demo");
    assert_eq!(body.trim(), "# GET /ping\n\nPong.");

    let rendered = mind_mesh_core::render_markdown(&fm, &body).unwrap();
    let (fm2, body2) = parse_markdown(&rendered).unwrap();
    assert_eq!(fm2.doc_type, fm.doc_type);
    assert_eq!(fm2.project, fm.project);
    assert_eq!(body2, body);
}

#[test]
fn search_finds_demo_document() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/knowledge");
    let paths = KnowledgePaths::new(&root);

    let hits = KnowledgeSearch::new(&paths)
        .search(
            "health",
            SearchOptions {
                project: Some("demo-api".into()),
                doc_type: None,
                limit: 10,
            },
        )
        .unwrap();

    assert!(!hits.is_empty());
    assert!(hits.iter().any(|h| h.path.contains("health")));
}

#[test]
fn write_and_read_doc() {
    let dir = tempfile::tempdir().unwrap();
    let paths = KnowledgePaths::new(dir.path());
    paths.ensure_project_layout("test-proj").unwrap();

    let fm = DocFrontmatter {
        doc_type: DocType::Module,
        project: "test-proj".into(),
        module: Some("core".into()),
        title: Some("core".into()),
        source: None,
        refs: vec![],
        deps: vec![],
        extra: Default::default(),
    };
    let body = "# core\n\nCore module.";
    let path = paths.doc_path("test-proj", DocType::Module, "core");
    write_doc(&path, &fm, body).unwrap();

    let doc = read_doc(&path).unwrap();
    assert_eq!(doc.frontmatter.doc_type, DocType::Module);
    assert!(doc.body.contains("Core module"));
}
