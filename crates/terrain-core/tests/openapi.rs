mod common;

use common::TestKnowledgeSetup;
use terrain_core::ingest::OpenApiImporter;
use terrain_core::{read_doc, DocType};

fn write_spec(repo: &std::path::Path, rel: &str, body: &str) {
    let p = repo.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, body).unwrap();
}

/// A leftover doc from a previous scan on an older version, e.g. generated
/// from a dependency spec under `.venv/` that is no longer discovered.
fn write_stale_doc(setup: &TestKnowledgeSetup, rel: &str) -> std::path::PathBuf {
    let dir = setup.paths.project_dir(&setup.slug).join("interfaces");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(rel);
    std::fs::write(&path, "---\ntype: interface\nproject: stale\n---\n\nstale doc").unwrap();
    path
}

#[test]
fn rescan_without_specs_removes_stale_docs() {
    let setup = TestKnowledgeSetup::new("openapi-stale-only");
    let repo = setup.repo.clone();
    write_spec(
        &repo,
        ".venv/Lib/site-packages/litellm/proxy/openapi.json",
        r#"{"openapi":"3.0.0","paths":{"/models":{"get":{"operationId":"list_models"}}}}"#,
    );
    let stale = write_stale_doc(&setup, "get-models.md");

    let written = OpenApiImporter::new(&setup.paths, &setup.slug)
        .import_repo(repo.to_str().unwrap())
        .unwrap();

    assert!(written.is_none(), "venv-only repo must import nothing");
    assert!(!stale.exists(), "stale doc must be cleared on re-scan");
}

#[test]
fn rescan_replaces_docs_with_current_spec_set() {
    let setup = TestKnowledgeSetup::new("openapi-replace");
    let repo = setup.repo.clone();
    write_spec(&repo, "api/openapi.json", r#"{"openapi":"3.0.0","paths":{"/ping":{"get":{"summary":"Ping"}}}}"#);
    let stale = write_stale_doc(&setup, "get-models.md");

    let written = OpenApiImporter::new(&setup.paths, &setup.slug)
        .import_repo(repo.to_str().unwrap())
        .unwrap();

    assert_eq!(written, Some(2), "one route → one interface + one route doc");
    assert!(!stale.exists(), "stale doc must be cleared on re-scan");
    assert!(setup
        .paths
        .doc_path(&setup.slug, DocType::Interface, "get-ping")
        .exists());
    assert!(setup
        .paths
        .doc_path(&setup.slug, DocType::Route, "get-ping")
        .exists());
}

#[test]
fn handler_falls_back_to_method_and_path_without_operation_id() {
    let setup = TestKnowledgeSetup::new("openapi-handler-fallback");
    let repo = setup.repo.clone();
    write_spec(&repo, "api/openapi.json", r#"{"openapi":"3.0.0","paths":{"/ping":{"get":{"summary":"Ping"}}}}"#);

    OpenApiImporter::new(&setup.paths, &setup.slug)
        .import_repo(repo.to_str().unwrap())
        .unwrap();

    let doc = read_doc(setup.paths.doc_path(&setup.slug, DocType::Route, "get-ping")).unwrap();
    let handler = doc
        .frontmatter
        .extra
        .get("handler")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert_ne!(handler, "unknown", "literal 'unknown' must not be written");
    assert_eq!(handler, "GET /ping");
    assert_eq!(doc.frontmatter.source.as_deref(), Some("api/openapi.json"));
}
