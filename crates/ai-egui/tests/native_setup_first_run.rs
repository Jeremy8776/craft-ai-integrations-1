//! The setup widget on the first run: no tick box unless a licence needs one, and a way to take
//! the suggested folder when none is chosen. Real layout and clicks; tiny files, no network.
#[path = "../../ai-client/tests/native_setup_support/mod.rs"]
mod fixtures;
use craft_ai_client::{AiClient, FolderDesk, ProviderFactory, SetupSession};
use craft_ai_core::{AiError, AppId};
use craft_ai_egui::{
    ACCEPT_LICENCES, DOWNLOAD_MODEL, NativeSetupAction, USE_SUGGESTED_FOLDER, native_model_setup,
};
use craft_ai_native::{secrets::MemoryStore, shared::SharedStore};
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use std::sync::Arc;

struct State {
    _dir: tempfile::TempDir,
    session: SetupSession,
    last: Option<NativeSetupAction>,
}

/// A harness around a setup session. `licence` says whether the files' licences need agreement
/// and `folder` whether a default model folder is registered.
fn harness(licence: bool, folder: bool) -> Harness<'static, State> {
    let (dir, desk, _) = fixtures::world();
    let desk = if folder {
        desk
    } else {
        FolderDesk::new(SharedStore::new(dir.path().join("empty.json")))
    };
    let store = if folder {
        "connections.json"
    } else {
        "empty.json"
    };
    let mut factory = ProviderFactory::standard();
    factory.register("candle", |_, _| {
        Err(AiError::unsupported("setup does not run inference"))
    });
    let client = AiClient::new(
        AppId::new("test-app").unwrap(),
        SharedStore::new(dir.path().join(store)),
        Arc::new(MemoryStore::new()),
    )
    .with_factory(factory);
    let mut feature = fixtures::feature();
    for model in &mut fixtures::candle_mut(&mut feature).models {
        model.licence.accept_required = licence;
    }
    let plan = desk
        .prepare_native_setup(&feature, "candle", &client)
        .unwrap();
    let mut h = Harness::new_ui_state(
        |ui, state: &mut State| {
            if let Some(action) = native_model_setup(ui, &mut state.session) {
                state.last = Some(action);
            }
        },
        State {
            _dir: dir,
            session: SetupSession::new(plan),
            last: None,
        },
    );
    h.run();
    h
}

/// Files with no licence to agree to show no tick box, and Download works straight away.
#[test]
fn files_with_no_licence_to_accept_show_no_tick_box() {
    let mut h = harness(false, true);
    assert!(h.query_by_label(ACCEPT_LICENCES).is_none());
    assert!(
        !h.get_by_label(DOWNLOAD_MODEL)
            .accesskit_node()
            .is_disabled()
    );
    h.get_by_label(DOWNLOAD_MODEL).click();
    h.run();
    assert!(matches!(h.state().last, Some(NativeSetupAction::Work(_))));
}

/// Files whose licence needs agreement still show the box and still wait for it.
#[test]
fn files_that_need_agreement_still_wait_for_the_tick() {
    let h = harness(true, true);
    h.get_by_label(ACCEPT_LICENCES);
    assert!(
        h.get_by_label(DOWNLOAD_MODEL)
            .accesskit_node()
            .is_disabled()
    );
}

/// With no model folder the widget offers the suggested one, keeps Download off, and reports the
/// choice for the host to act on.
#[test]
fn with_no_folder_the_suggested_one_is_offered() {
    let mut h = harness(false, false);
    assert!(
        h.get_by_label(DOWNLOAD_MODEL)
            .accesskit_node()
            .is_disabled()
    );
    h.get_by_label(USE_SUGGESTED_FOLDER).click();
    h.run();
    assert!(matches!(
        h.state().last,
        Some(NativeSetupAction::UseSuggestedFolder)
    ));
    assert!(!h.state().session.busy());
}

/// With a folder chosen the suggestion is not offered.
#[test]
fn with_a_folder_chosen_the_suggestion_is_not_offered() {
    let h = harness(false, true);
    assert!(h.query_by_label(USE_SUGGESTED_FOLDER).is_none());
}
