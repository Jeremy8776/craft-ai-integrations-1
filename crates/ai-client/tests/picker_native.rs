//! A built-in model must not advertise an actionable setup route to an app without its engine.
use craft_ai_client::{AiClient, GroupState, SetupLink};
use craft_ai_core::AppId;
use craft_ai_native::secrets::MemoryStore;
use craft_ai_native::shared::SharedStore;
use std::sync::Arc;

#[test]
fn the_builtin_model_only_offers_setup_when_this_app_has_the_engine() {
    let dir = tempfile::tempdir().unwrap();
    let client = AiClient::new(
        AppId::new("test-app").unwrap(),
        SharedStore::new(dir.path().join("connections.json")),
        Arc::new(MemoryStore::new()),
    );
    let features = craft_ai_features::builtin();
    let role = features
        .iter()
        .find(|feature| feature.id == "select-subject")
        .unwrap()
        .role()
        .unwrap();
    let view = client
        .picker(&role, &craft_ai_features::setup_options(&features))
        .unwrap();
    let native = view
        .groups
        .iter()
        .find(|g| g.title.contains("built in"))
        .unwrap();
    if cfg!(feature = "candle") {
        assert_eq!(native.state, GroupState::NotSetUp);
        assert!(matches!(&native.setup, Some(SetupLink::New { option })
            if option == "native:select-subject:candle"));
    } else {
        assert_eq!(native.state, GroupState::NeedsAdapter);
        assert!(native.setup.is_none());
    }
}
