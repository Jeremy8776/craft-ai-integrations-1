//! The same setup state is used by a host's own rendering and the neutral fallback widget.
mod native_setup_support;
use craft_ai_client::{
    AiClient, FolderDesk, NativeSetupPlan, ProviderFactory, SetupSession, SetupTask,
};
use craft_ai_core::{AiError, AppId};
use craft_ai_native::{secrets::MemoryStore, shared::SharedStore};
use native_setup_support::{feature, world};
use std::sync::Arc;

fn prepared() -> (tempfile::TempDir, FolderDesk, NativeSetupPlan) {
    let (dir, desk, _) = world();
    let mut factory = ProviderFactory::standard();
    factory.register("candle", |_, _| {
        Err(AiError::unsupported("setup must not construct a model"))
    });
    let app = AiClient::new(
        AppId::new("test-app").unwrap(),
        SharedStore::new(dir.path().join("connections.json")),
        Arc::new(MemoryStore::new()),
    )
    .with_factory(factory);
    let plan = desk
        .prepare_native_setup(&feature(), "candle", &app)
        .unwrap();
    (dir, desk, plan)
}

#[test]
fn download_needs_an_explicit_yes_and_cannot_be_started_twice() {
    let (_dir, _desk, plan) = prepared();
    let mut session = SetupSession::new(plan.clone());
    assert!(session.begin(SetupTask::Download).is_none());
    session.accept_licences(true);
    let request = session
        .begin(SetupTask::Download)
        .expect("the approved download is actionable");
    assert_eq!(request.plan(), &plan);
    assert!(request.approved());
    assert!(session.busy());
    assert!(session.begin(SetupTask::Download).is_none());
}

#[test]
fn changing_the_destination_discards_the_old_approval() {
    let (dir, desk, plan) = prepared();
    let mut session = SetupSession::new(plan.clone());
    session.accept_licences(true);
    let _ = session.review(plan);
    assert!(session.accepted(), "the same frame does not reset a choice");
    let next = dir.path().join("next");
    std::fs::create_dir(&next).unwrap();
    let folder = desk.add(&next, None).unwrap();
    desk.set_default(&folder.folder.id).unwrap();
    let mut factory = ProviderFactory::standard();
    factory.register("candle", |_, _| Err(AiError::unsupported("not inference")));
    let app = AiClient::new(
        AppId::new("app").unwrap(),
        SharedStore::new(dir.path().join("connections.json")),
        Arc::new(MemoryStore::new()),
    )
    .with_factory(factory);
    let _ = session.review(
        desk.prepare_native_setup(&feature(), "candle", &app)
            .unwrap(),
    );
    assert!(!session.accepted());
    assert!(session.begin(SetupTask::Download).is_none());
}

#[test]
fn a_completed_request_makes_checked_files_available_but_does_not_bind_them() {
    let (dir, _desk, plan) = prepared();
    let destination = plan.destination().unwrap();
    std::fs::create_dir_all(destination).unwrap();
    for (i, file) in plan.route().models.iter().enumerate() {
        std::fs::write(destination.join(&file.filename), vec![b'x'; i + 1]).unwrap();
    }
    let mut session = SetupSession::new(plan);
    session.accept_licences(true);
    let request = session.begin(SetupTask::Download).unwrap();
    let result = request.run(
        &craft_ai_comfy_env::Downloader::new(Default::default()),
        None,
        &mut |_, _| {},
    );
    assert!(
        result.is_ok(),
        "the requested files are actually checked: {result:?}"
    );
    assert!(session.complete(request.ticket(), result));
    assert!(session.checked().is_some());
    assert!(!session.busy());
    assert!(
        SharedStore::new(dir.path().join("connections.json"))
            .load()
            .unwrap()
            .defaults
            .is_empty()
    );
}

#[test]
fn a_cancelled_workers_late_answer_cannot_replace_a_newer_job() {
    let (_dir, _desk, plan) = prepared();
    let mut session = SetupSession::new(plan);
    session.accept_licences(true);
    let old = session.begin(SetupTask::Download).unwrap();
    assert_eq!(session.cancel(), Some(old.ticket()));
    session.accept_licences(true);
    let next = session.begin(SetupTask::Download).unwrap();
    assert_ne!(old.ticket(), next.ticket());
    assert!(!session.complete(old.ticket(), Err(AiError::invalid("a stale answer"))));
    assert!(session.busy());
    assert_eq!(session.problem(), None);
}

#[test]
fn a_result_from_another_reviewed_directory_is_not_accepted() {
    let (_dir, _desk, plan) = prepared();
    let mut session = SetupSession::new(plan);
    session.accept_licences(true);
    let request = session.begin(SetupTask::Download).unwrap();
    let (_other_dir, _other_desk, other) = prepared();
    std::fs::create_dir_all(other.destination().unwrap()).unwrap();
    for (i, file) in other.route().models.iter().enumerate() {
        std::fs::write(
            other.destination().unwrap().join(&file.filename),
            vec![b'x'; i + 1],
        )
        .unwrap();
    }
    let result = other.download(craft_ai_client::NativeInstall {
        downloader: &craft_ai_comfy_env::Downloader::new(Default::default()),
        token: None,
        licences_accepted: true,
        cancel: &std::sync::atomic::AtomicBool::new(false),
        progress: &mut |_, _| {},
    });
    assert!(result.is_ok());
    assert!(session.complete(request.ticket(), result));
    assert!(session.checked().is_none());
    assert_eq!(
        session.problem(),
        Some(craft_ai_core::ErrorKind::InvalidRequest)
    );
}

#[test]
fn cancelling_a_session_stops_an_unstarted_worker_before_it_creates_files() {
    let (_dir, _desk, plan) = prepared();
    let destination = plan.destination().unwrap().to_owned();
    let mut session = SetupSession::new(plan);
    session.accept_licences(true);
    let request = session.begin(SetupTask::Download).unwrap();
    session.cancel();
    let result = request.run(
        &craft_ai_comfy_env::Downloader::new(Default::default()),
        None,
        &mut |_, _| {},
    );
    assert_eq!(
        result.unwrap_err().kind,
        craft_ai_core::ErrorKind::Cancelled
    );
    assert!(!destination.exists());
}

#[test]
fn progress_belongs_to_a_current_request_and_a_real_file() {
    let (_dir, _desk, plan) = prepared();
    let mut session = SetupSession::new(plan);
    session.accept_licences(true);
    let request = session.begin(SetupTask::Download).unwrap();
    let progress = craft_ai_client::SetupProgress {
        phase: craft_ai_client::SetupPhase::Verifying,
        done: 1,
        total: 2,
    };
    assert!(session.report_progress(request.ticket(), 0, progress));
    assert_eq!(session.progress(), Some((0, progress)));
    assert!(!session.report_progress(request.ticket(), usize::MAX, progress));
    session.cancel();
    assert_eq!(session.progress(), None);
    assert!(!session.report_progress(request.ticket(), 0, progress));
}
