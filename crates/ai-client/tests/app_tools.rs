//! `AppTools`: every app reads the same catalogue and the same shared choice.
//!
//! Scripted stand-in only: a temporary store, an in-memory key store and a fake adapter. No
//! network, no real settings and no model.

use std::sync::Arc;

use craft_ai_client::{AiClient, AppTools, ProviderFactory, RoleStatus};
use craft_ai_core::mock::{MockProvider, Outcome, Script};
use craft_ai_core::{AppId, Provider};
use craft_ai_native::secrets::MemoryStore;
use craft_ai_native::shared::SharedStore;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// An `AppTools` for `app` over a scripted client: the given store, in-memory keys and a fake
/// adapter for the `tidy` service, so nothing real is read or sent.
fn tools_for(app: &str, shared: &SharedStore) -> Result<AppTools, Box<dyn std::error::Error>> {
    let mut factory = ProviderFactory::standard();
    factory.register("tidy", |connection, _key| {
        let script = Script {
            running_polls: 0,
            text_chunks: vec![],
            outcome: Outcome::Succeed(vec![]),
        };
        Ok(Arc::new(MockProvider::new(connection.models.clone(), script)) as Arc<dyn Provider>)
    });
    let client = AiClient::new(
        AppId::new(app)?,
        shared.clone(),
        Arc::new(MemoryStore::new()),
    )
    .with_factory(factory);
    Ok(AppTools::with_client(client))
}

/// With no choice made, every catalogue tool is listed and each is `Unavailable` with a reason.
#[test]
fn nothing_chosen_greys_every_tool_with_a_reason() -> TestResult {
    let dir = tempfile::tempdir()?;
    let shared = SharedStore::new(dir.path().join("store.json"));
    let report = tools_for("lightcraft", &shared)?.report();

    assert!(
        report.len() >= 8,
        "the whole catalogue is listed, got {}",
        report.len()
    );
    for tool in &report {
        match &tool.status {
            RoleStatus::Unavailable { reason } => {
                assert!(!reason.is_empty(), "{} has no reason", tool.id)
            }
            RoleStatus::Ready(_) => panic!("{} is ready with nothing chosen", tool.id),
        }
    }
    assert!(
        report
            .iter()
            .any(|t| t.id == "select-subject" && t.title == "Select subject")
    );
    Ok(())
}

/// A tool id the catalogue lacks greys out with a reason and has no role to run.
#[test]
fn an_unknown_tool_is_unavailable_not_an_error() -> TestResult {
    let dir = tempfile::tempdir()?;
    let shared = SharedStore::new(dir.path().join("store.json"));
    let tools = tools_for("pdfcraft", &shared)?;

    match tools.status("no-such-tool") {
        RoleStatus::Unavailable { reason } => {
            assert!(!reason.is_empty(), "a greyed tool needs a reason")
        }
        RoleStatus::Ready(_) => panic!("an unknown tool was ready"),
    }
    assert!(tools.role("no-such-tool").is_none());
    assert!(tools.role("select-subject").is_some());
    Ok(())
}

/// Listing and asking for a status never create or change the shared store file.
#[test]
fn reading_writes_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("store.json");
    let shared = SharedStore::new(&path);
    let tools = tools_for("wordcraft", &shared)?;
    let _ = tools.report();
    let _ = tools.status("select-subject");

    assert!(!path.exists(), "reading the choices created the store file");
    Ok(())
}

/// `standard` refuses an invalid app id with a readable sentence, before touching any store.
#[test]
fn a_bad_app_id_is_refused_with_a_sentence() {
    match AppTools::standard("Not A Valid Id!") {
        Ok(_) => panic!("an invalid app id was accepted"),
        Err(why) => assert!(!why.is_empty()),
    }
}
