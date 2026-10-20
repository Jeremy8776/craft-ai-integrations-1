//! Built-in model setup must use the files its own runner loads, not the ComfyUI recipe.
use craft_ai_features::{
    InstalledFile, Inventory, Route, State, builtin, plan_native, setup_options,
};

#[test]
fn candle_without_comfyui_plans_its_three_own_files() {
    let feature = builtin()
        .into_iter()
        .find(|f| f.id == "select-subject")
        .unwrap();
    let plan = plan_native(&feature, "candle", &Inventory::default());
    assert_eq!(plan.state, State::NeedsInstall);
    let names: Vec<_> = plan
        .missing_files
        .iter()
        .map(|f| f.filename.as_str())
        .collect();
    assert_eq!(names, ["model.safetensors", "vocab.json", "merges.txt"]);
    assert_eq!(plan.download_bytes, 3_441_325_459);
    assert_eq!(plan.licences.len(), 1);
    assert_eq!(plan.licences[0].name, "Meta SAM License");
    assert!(plan.missing_packs.is_empty());
}

#[test]
fn native_files_already_present_are_ready_without_comfyui() {
    let feature = builtin()
        .into_iter()
        .find(|f| f.id == "select-subject")
        .unwrap();
    let candle = &feature.locals[0].native[0];
    let inventory = Inventory {
        files: candle
            .models
            .iter()
            .map(|m| InstalledFile {
                folder: m.folder.clone(),
                filename: m.filename.clone(),
                size_bytes: m.size_bytes,
            })
            .collect(),
        ..Inventory::default()
    };
    let plan = plan_native(&feature, "candle", &inventory);
    assert_eq!(plan.state, State::Ready);
    assert_eq!(plan.download_bytes, 0);
    assert!(plan.licences.is_empty());
}

#[test]
fn missing_native_files_count_against_free_space() {
    let feature = builtin()
        .into_iter()
        .find(|f| f.id == "select-subject")
        .unwrap();
    let inventory = Inventory {
        free_disk_bytes: Some(100),
        ..Inventory::default()
    };
    let plan = plan_native(&feature, "candle", &inventory);
    assert_eq!(plan.state, State::Blocked);
    assert_eq!(plan.blockers.len(), 1);
}

#[test]
fn unfinished_or_unknown_engines_offer_no_download() {
    let feature = builtin()
        .into_iter()
        .find(|f| f.id == "select-subject")
        .unwrap();
    for engine in ["burn", "not-an-engine"] {
        let plan = plan_native(&feature, engine, &Inventory::default());
        assert_eq!(plan.state, State::NoLocalRoute);
        assert!(plan.missing_files.is_empty());
    }
}

#[test]
fn setup_options_offer_candle_with_its_own_size_but_not_unbuilt_burn() {
    let options = setup_options(&builtin());
    let candle = options
        .iter()
        .find(|o| o.id == "native:select-subject:candle")
        .unwrap();
    assert!(matches!(&candle.route, Route::Native { feature, provider }
        if feature == "select-subject" && provider == "candle"));
    assert_eq!(candle.size_bytes, Some(3_441_325_459));
    assert!(candle.usable);
    assert!(options.iter().all(|o| o.id != "native:select-subject:burn"));
}
