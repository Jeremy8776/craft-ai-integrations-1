//! `AppTools`: every app reads the same catalogue and the same shared choice.
//!
//! Scripted stand-in only: a temporary store, an in-memory key store and a fake adapter. No
//! network, no real settings and no model.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use craft_ai_client::{
    AiClient, AppTools, EngineRegistry, LoadedModel, ModelRunner, ProviderFactory, RoleStatus,
    RunContext, ToolDesk,
};
use craft_ai_core::mock::{MockProvider, Outcome, Script};
use craft_ai_core::{
    AiError, AppId, CapabilityId, CapabilitySpec, ConsentLedger, InputRole, ModelId, ModelInfo,
    Output, Payload, Provider, Request, Role, RoleId, StoreFile,
};
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

/// A tool someone declared in the shared store is listed after the catalogue, greyed with a reason
/// until a model is chosen, and has a role to run.
#[test]
fn a_custom_tool_is_listed_after_the_catalogue() -> TestResult {
    let dir = tempfile::tempdir()?;
    let shared = SharedStore::new(dir.path().join("store.json"));
    let role = Role {
        id: RoleId::new("my-tidy-tool")?,
        label: "My tidy tool".to_owned(),
        capability: CapabilityId::parse("text.generate")?,
        inputs: vec![InputRole::Prompt],
    };
    let launcher = AiClient::new(
        AppId::new("launcher")?,
        shared.clone(),
        Arc::new(MemoryStore::new()),
    );
    ToolDesk::new(launcher, shared.clone(), None).register_tool(role.clone())?;

    let tools = tools_for("soundcraft", &shared)?;
    let report = tools.report();
    let last = report.last().expect("the report has tools");
    assert_eq!(
        (last.id.as_str(), last.title.as_str()),
        ("my-tidy-tool", "My tidy tool")
    );
    assert!(matches!(&last.status, RoleStatus::Unavailable { reason } if !reason.is_empty()));
    assert_eq!(tools.role("my-tidy-tool"), Some(role));
    assert!(tools.role("select-subject").is_some());
    Ok(())
}

/// A stand-in engine for `image.segment`: answers one text output, or waits to be cancelled.
#[derive(Clone)]
struct Fake {
    hang: bool,
}

struct FakeLoaded(Fake);

impl ModelRunner for Fake {
    fn engine(&self) -> &str {
        "rten"
    }
    fn model(&self) -> ModelInfo {
        let capability = CapabilityId::parse("image.segment").unwrap();
        ModelInfo::new(ModelId::new("sam21").unwrap(), "fake").with(
            CapabilitySpec::new(capability)
                .accepting(InputRole::Source, 1)
                .accepting(InputRole::Prompt, 1)
                .requiring(InputRole::Source),
        )
    }
    fn load(&self, _: &Path) -> Result<Box<dyn LoadedModel>, AiError> {
        Ok(Box::new(FakeLoaded(self.clone())))
    }
}

impl LoadedModel for FakeLoaded {
    fn run(&mut self, _: &Request, ctx: &RunContext) -> Result<Vec<Output>, AiError> {
        if self.0.hang {
            loop {
                ctx.check_cancelled()?;
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        Ok(vec![Output {
            payload: Payload::text("done"),
            label: None,
        }])
    }
}

/// An `AppTools` for `app` over a store where Select subject is bound, through the launcher's own
/// action, to the stand-in (or to nothing when `engine` is `None`).
fn bound_tools(
    app: &str,
    engine: Option<Fake>,
) -> Result<(AppTools, tempfile::TempDir), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let shared = SharedStore::new(dir.path().join("connections.json"));
    shared.replace(&StoreFile::default())?;
    let client = |name: &str| -> Result<AiClient, Box<dyn std::error::Error>> {
        Ok(AiClient::new(
            AppId::new(name)?,
            shared.clone(),
            Arc::new(MemoryStore::new()),
        ))
    };
    let mut registry = EngineRegistry::new();
    if let Some(engine) = engine {
        let weights = dir.path().join("encoder.onnx");
        std::fs::write(&weights, b"x")?;
        let feature = craft_ai_features::builtin()
            .into_iter()
            .find(|f| f.id == "select-subject")
            .ok_or("no select-subject")?;
        ToolDesk::new(client("launcher")?, shared.clone(), None)
            .enable_native_with(&feature, "rten", &weights)?;
        registry.register(engine);
    }
    let mut factory = ProviderFactory::standard();
    factory.register_engines(registry);
    Ok((
        AppTools::with_client(client(app)?.with_factory(factory)),
        dir,
    ))
}

fn picture_request(tools: &AppTools) -> Request {
    tools.request("select-subject").unwrap().with_input(
        InputRole::Source,
        Payload::inline("image/png", vec![1, 2, 3]).unwrap(),
    )
}

/// Plan, then run: the outputs of the chosen model come back, and the same call works for every app id.
#[test]
fn plan_then_run_returns_what_the_chosen_model_produced() -> TestResult {
    for app in ["lightcraft", "pdfcraft"] {
        let (tools, _dir) = bound_tools(app, Some(Fake { hang: false }))?;
        let plan = tools.plan("select-subject", picture_request(&tools))?;
        let outputs = tools.run(&plan, &ConsentLedger::default())?;
        assert_eq!(outputs.len(), 1, "{app}");
    }
    Ok(())
}

/// A request is for the tool's own capability, and an unknown tool has none.
#[test]
fn a_request_carries_the_tools_capability() -> TestResult {
    let (tools, _dir) = bound_tools("wordcraft", None)?;
    assert_eq!(
        tools
            .request("select-subject")
            .map(|r| r.capability.to_string()),
        Some("image.segment".to_owned())
    );
    assert!(tools.request("no-such-tool").is_none());
    Ok(())
}

/// With nothing chosen, or an unknown tool, planning fails with a sentence and nothing runs.
#[test]
fn planning_without_a_choice_or_an_unknown_tool_is_a_sentence() -> TestResult {
    let (tools, _dir) = bound_tools("wordcraft", None)?;
    let nothing = tools
        .plan("select-subject", picture_request(&tools))
        .unwrap_err();
    assert!(!nothing.is_empty());
    let unknown = tools
        .plan(
            "no-such-tool",
            Request::new(CapabilityId::parse("image.segment")?),
        )
        .unwrap_err();
    assert!(unknown.contains("not part of the suite"), "{unknown}");
    Ok(())
}

/// Cancel from another thread reaches the running job, and a cancel with nothing running is harmless.
#[test]
fn cancel_reaches_the_running_job_and_is_harmless_otherwise() -> TestResult {
    let (tools, _dir) = bound_tools("effectcraft", Some(Fake { hang: true }))?;
    let canceller = tools.canceller();
    canceller.cancel();
    let plan = tools.plan("select-subject", picture_request(&tools))?;
    let stopper = std::thread::spawn(move || {
        for _ in 0..200 {
            canceller.cancel();
            std::thread::sleep(Duration::from_millis(10));
        }
    });
    let error = tools.run(&plan, &ConsentLedger::default()).unwrap_err();
    stopper.join().unwrap();
    assert!(error.to_lowercase().contains("cancel"), "{error}");
    tools.canceller().cancel();
    Ok(())
}
