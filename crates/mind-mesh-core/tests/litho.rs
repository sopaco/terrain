use std::fs;
use std::path::Path;

use mind_mesh_core::{
    count_human_docs, litho_human_complete, litho_human_complete_with_research, litho_research_ready,
    KnowledgePaths, LITHO_CORE_RESEARCH_FILES,
};

fn write_human_fixture(base: &Path, module_count: usize) {
    fs::create_dir_all(base.join("4.Deep-Exploration")).unwrap();
    for name in [
        "1.概述.md",
        "2.架构.md",
        "3.工作流.md",
        "5.边界接口.md",
        "6.数据库概览.md",
    ] {
        fs::write(base.join(name), format!("# {name}\n")).unwrap();
    }
    for i in 0..module_count {
        fs::write(
            base.join(format!("4.Deep-Exploration/mod{i}.md")),
            format!("# mod{i}\n"),
        )
        .unwrap();
    }
}

fn write_research_fixture(base: &Path, module_count: usize) {
    fs::create_dir_all(base).unwrap();
    for name in LITHO_CORE_RESEARCH_FILES {
        fs::write(base.join(name), format!("# {name}\n")).unwrap();
    }
    fs::create_dir_all(base.join("modules")).unwrap();
    for i in 0..module_count {
        fs::write(
            base.join(format!("modules/mod{i}.md")),
            format!("# mod{i}\n"),
        )
        .unwrap();
    }
}

#[test]
fn litho_human_complete_requires_core_files_and_deep_exploration() {
    let tmp = tempfile::tempdir().unwrap();
    let human = tmp.path().join("human");
    fs::create_dir_all(&human).unwrap();
    assert!(!litho_human_complete(&human));

    write_human_fixture(&human, 0);
    assert!(!litho_human_complete(&human));

    fs::write(human.join("4.Deep-Exploration/core.md"), "# core\n").unwrap();
    assert!(litho_human_complete(&human));
}

#[test]
fn litho_human_complete_with_research_requires_all_module_docs() {
    let tmp = tempfile::tempdir().unwrap();
    let human = tmp.path().join("human");
    let research = tmp.path().join("research");
    fs::create_dir_all(&human).unwrap();
    fs::create_dir_all(&research).unwrap();
    write_research_fixture(&research, 3);
    write_human_fixture(&human, 1);

    assert!(!litho_human_complete_with_research(&human, Some(&research)));
    write_human_fixture(&human, 3);
    assert!(litho_human_complete_with_research(&human, Some(&research)));
}

#[test]
fn litho_research_ready_requires_core_reports_and_modules() {
    let tmp = tempfile::tempdir().unwrap();
    let research = tmp.path().join("research");
    fs::create_dir_all(&research).unwrap();
    assert!(!litho_research_ready(&research));

    fs::write(research.join("preprocessing.md"), "# p\n").unwrap();
    assert!(!litho_research_ready(&research));

    write_research_fixture(&research, 2);
    assert!(litho_research_ready(&research));
}

#[test]
fn count_human_docs_only_counts_human_section() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("repo");
    let mind_mesh = root.join(".mind-mesh");
    fs::create_dir_all(mind_mesh.join("human")).unwrap();
    fs::create_dir_all(mind_mesh.join("agent")).unwrap();
    fs::create_dir_all(mind_mesh.join("interfaces")).unwrap();
    fs::write(mind_mesh.join("human/1.概述.md"), "# overview\n").unwrap();
    fs::write(mind_mesh.join("agent/context.md"), "# ctx\n").unwrap();
    fs::write(mind_mesh.join("interfaces/ping.md"), "# ping\n").unwrap();

    let _lock = mind_mesh_core::registry::registry_test_lock();
    let registry_dir = tempfile::tempdir().unwrap();
    let registry_file = registry_dir.path().join("registry.json");
    unsafe {
        std::env::set_var("MIND_MESH_REGISTRY_FILE", &registry_file);
    }
    mind_mesh_core::register_project("demo", &root.display().to_string()).unwrap();

    let paths = KnowledgePaths::new();
    assert_eq!(count_human_docs(&paths, "demo"), 1);
}
