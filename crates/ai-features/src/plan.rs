//! Compares a recipe with what this computer already has and says what is left to do.
//!
//! Pure: the caller builds an [`Inventory`] (the native side scans the disk) and gets a [`Plan`].
//! A file counts as present only when folder, name and exact size all match, so a half-finished
//! or different file with the right name is treated as missing.

use crate::feature::Feature;
use crate::recipe::{Licence, LocalRecipe, ModelFile, NodePack};
use serde::{Deserialize, Serialize};

/// A model file found on disk.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledFile {
    pub folder: String,
    pub filename: String,
    pub size_bytes: u64,
}

/// What this computer has. Every field is what was found, never what was assumed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inventory {
    /// A ComfyUI installation was found.
    pub comfyui_found: bool,
    pub files: Vec<InstalledFile>,
    /// Folder names under `custom_nodes`, lower case.
    pub node_folders: Vec<String>,
    pub vram_mb: Option<u32>,
    pub free_disk_bytes: Option<u64>,
}

impl Inventory {
    /// The installed copy of a wanted file: same folder, same size, and a path that is the wanted
    /// name either directly or inside a sub-folder (a user may keep it in `FLUX/` and so on).
    pub fn find(&self, wanted: &ModelFile) -> Option<&InstalledFile> {
        let leaf = wanted.filename.to_ascii_lowercase();
        self.files.iter().find(|f| {
            let path = f.filename.to_ascii_lowercase();
            f.folder.eq_ignore_ascii_case(&wanted.folder)
                && f.size_bytes == wanted.size_bytes
                && (path == leaf || path.ends_with(&format!("/{leaf}")))
        })
    }

    fn has_pack(&self, pack: &NodePack) -> bool {
        pack.folder_names
            .iter()
            .any(|n| self.node_folders.iter().any(|f| f.eq_ignore_ascii_case(n)))
    }
}

/// Why a feature cannot be set up here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Blocker {
    NotEnoughVram { needed_mb: u32, have_mb: u32 },
    NotEnoughDisk { needed_bytes: u64, free_bytes: u64 },
}

/// Where a feature stands on this computer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum State {
    /// Nobody has worked out how to run it locally yet.
    NoLocalRoute,
    /// ComfyUI was not found.
    NoRuntime,
    /// Everything it needs is here.
    Ready,
    /// Something has to be installed first.
    NeedsInstall,
    /// It cannot run here as things stand.
    Blocked,
}

/// The result of comparing one feature with one inventory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub feature: String,
    /// The recipe this plan is about: its workflow id, such as `sam3-segment`. Empty for a
    /// feature with no local route.
    #[serde(default)]
    pub recipe: String,
    pub state: State,
    pub missing_files: Vec<ModelFile>,
    pub missing_packs: Vec<NodePack>,
    /// What would be downloaded.
    pub download_bytes: u64,
    /// Licences the user has to accept before the downloads start, once each.
    pub licences: Vec<Licence>,
    pub blockers: Vec<Blocker>,
}

/// Works out what `feature` still needs on a computer described by `inventory`.
pub fn plan(feature: &Feature, inventory: &Inventory) -> Plan {
    let mut out = Plan {
        feature: feature.id.clone(),
        recipe: String::new(),
        state: State::NoLocalRoute,
        missing_files: Vec::new(),
        missing_packs: Vec::new(),
        download_bytes: 0,
        licences: Vec::new(),
        blockers: Vec::new(),
    };
    let Some(recipe) = feature.locals.first() else {
        return out;
    };
    if !inventory.comfyui_found {
        out.state = State::NoRuntime;
        return out;
    }
    fill(&mut out, recipe, inventory);
    out.state = if !out.blockers.is_empty() {
        State::Blocked
    } else if out.missing_files.is_empty() && out.missing_packs.is_empty() {
        State::Ready
    } else {
        State::NeedsInstall
    };
    out
}

/// Works out what every one of `feature`'s local recipes still needs, one [`Plan`] each, in
/// catalogue order. Each names its recipe in [`Plan::recipe`], so several models for one tool
/// can be shown and installed side by side.
pub fn plans(feature: &Feature, inventory: &Inventory) -> Vec<Plan> {
    feature
        .locals
        .iter()
        .map(|recipe| {
            let mut out = Plan {
                feature: feature.id.clone(),
                recipe: recipe.workflow.clone(),
                state: State::NoLocalRoute,
                missing_files: Vec::new(),
                missing_packs: Vec::new(),
                download_bytes: 0,
                licences: Vec::new(),
                blockers: Vec::new(),
            };
            if !inventory.comfyui_found {
                out.state = State::NoRuntime;
                return out;
            }
            fill(&mut out, recipe, inventory);
            out.state = if !out.blockers.is_empty() {
                State::Blocked
            } else if out.missing_files.is_empty() && out.missing_packs.is_empty() {
                State::Ready
            } else {
                State::NeedsInstall
            };
            out
        })
        .collect()
}

/// Compares a built-in runner's own files with the folders on this computer.
/// A runner does not need ComfyUI or its node packs.
pub fn plan_native(feature: &Feature, provider: &str, inventory: &Inventory) -> Plan {
    let mut out = Plan {
        feature: feature.id.clone(),
        recipe: feature
            .locals
            .first()
            .map(|r| r.workflow.clone())
            .unwrap_or_default(),
        state: State::NoLocalRoute,
        missing_files: Vec::new(),
        missing_packs: Vec::new(),
        download_bytes: 0,
        licences: Vec::new(),
        blockers: Vec::new(),
    };
    let Some(route) = feature.locals.first().filter(|r| r.usable).and_then(|r| {
        r.native
            .iter()
            .find(|n| n.provider == provider && !n.models.is_empty())
    }) else {
        return out;
    };
    fill_files(&mut out, &route.models, inventory);
    out.state = if !out.blockers.is_empty() {
        State::Blocked
    } else if out.missing_files.is_empty() {
        State::Ready
    } else {
        State::NeedsInstall
    };
    out
}

fn fill(out: &mut Plan, recipe: &LocalRecipe, inventory: &Inventory) {
    fill_files(out, &recipe.models, inventory);
    out.missing_packs = recipe
        .packs
        .iter()
        .filter(|p| !inventory.has_pack(p))
        .cloned()
        .collect();
    if let (Some(needed), Some(have)) = (recipe.min_vram_mb, inventory.vram_mb)
        && have < needed
    {
        out.blockers.push(Blocker::NotEnoughVram {
            needed_mb: needed,
            have_mb: have,
        });
    }
}

fn fill_files(out: &mut Plan, models: &[ModelFile], inventory: &Inventory) {
    for model in models {
        if inventory.find(model).is_none() {
            out.download_bytes = out.download_bytes.saturating_add(model.size_bytes);
            if model.licence.accept_required && !out.licences.contains(&model.licence) {
                out.licences.push(model.licence.clone());
            }
            out.missing_files.push(model.clone());
        }
    }
    if let Some(free) = inventory.free_disk_bytes
        && out.download_bytes > free
    {
        out.blockers.push(Blocker::NotEnoughDisk {
            needed_bytes: out.download_bytes,
            free_bytes: free,
        });
    }
}
