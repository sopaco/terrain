use mind_mesh_agent::{ChatEngine, load_dotenv, resolve_model_config};
use mind_mesh_core::KnowledgePaths;

#[tokio::test]
async fn ask_smoke() {
    load_dotenv();
    let paths = KnowledgePaths::default_home();
    let config = resolve_model_config();
    let engine = ChatEngine::new(paths, config).expect("engine");
    let result = engine
        .ask(
            "test-session",
            "What is repomix-rs?",
            Some("repomix-rs"),
            None,
            |_| {},
            |_| {},
            |_| {},
            |_| {},
        )
        .await;
    match &result {
        Ok(r) => println!(
            "OK: {} chars, {} citations",
            r.answer.len(),
            r.citations.len()
        ),
        Err(e) => println!("ERR: {e:?}"),
    }
    result.expect("ask should succeed");
}
