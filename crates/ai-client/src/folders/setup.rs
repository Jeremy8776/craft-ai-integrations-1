//! Immutable, reviewable built-in model setup. No secret is stored in the plan.
use crate::{AiClient, FolderDesk, NativeInstall};
use craft_ai_core::AiError;
use craft_ai_features::{Feature, NativeRoute};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeSetupPlan {
    feature: Feature,
    route: NativeRoute,
    destination: Option<PathBuf>,
    existing: Option<PathBuf>,
    download_bytes: u64,
    available: bool,
}

/// Constructed only after all of the selected model's files pass their checks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedNative {
    feature: Feature,
    provider: String,
    weights: PathBuf,
}
impl CheckedNative {
    pub fn weights(&self) -> &Path {
        &self.weights
    }
    pub fn feature(&self) -> &Feature {
        &self.feature
    }
    pub fn provider(&self) -> &str {
        &self.provider
    }
}
impl NativeSetupPlan {
    /// Hash existing files without downloading or writing a binding.
    pub fn check_existing(
        &self,
        cancel: &std::sync::atomic::AtomicBool,
        progress: &mut dyn FnMut(usize, craft_ai_comfy_env::Progress),
    ) -> Result<CheckedNative, AiError> {
        if !self.available {
            return Err(AiError::unsupported("this app cannot use this option yet"));
        }
        let directory = self
            .existing
            .as_deref()
            .ok_or_else(|| AiError::invalid("the model files have not been found"))?;
        let weights = super::native::verify_directory(&self.route, directory, cancel, progress)?;
        Ok(CheckedNative {
            feature: self.feature.clone(),
            provider: self.route.provider.clone(),
            weights,
        })
    }

    pub fn feature(&self) -> &Feature {
        &self.feature
    }
    pub fn route(&self) -> &NativeRoute {
        &self.route
    }
    pub fn destination(&self) -> Option<&Path> {
        self.destination.as_deref()
    }
    pub fn existing_directory(&self) -> Option<&Path> {
        self.existing.as_deref()
    }
    pub fn download_bytes(&self) -> u64 {
        self.download_bytes
    }
    pub fn available(&self) -> bool {
        self.available
    }
    /// On a worker, download into the exact destination shown in this plan. No binding is written.
    pub fn download(&self, request: NativeInstall<'_>) -> Result<CheckedNative, AiError> {
        if !self.available {
            return Err(AiError::unsupported("this app cannot use this option yet"));
        }
        let directory = self
            .destination
            .as_deref()
            .ok_or_else(|| AiError::invalid("choose a model folder for downloads first"))?;
        let weights = self
            .route
            .weights()
            .ok_or_else(|| AiError::invalid("that model has no main file to open"))?;
        super::native::install_into(&self.route, directory, request)?;
        Ok(CheckedNative {
            feature: self.feature.clone(),
            provider: self.route.provider.clone(),
            weights: directory.join(&weights.filename),
        })
    }
}
impl FolderDesk {
    /// Build on a worker: folder discovery reads disk, but does not hash, download or write.
    pub fn prepare_native_setup(
        &self,
        feature: &Feature,
        provider: &str,
        client: &AiClient,
    ) -> Result<NativeSetupPlan, AiError> {
        let route = super::native::native_route(feature, provider)?;
        let found = self.native_files(feature, provider)?;
        let complete = craft_ai_features::plan_native(feature, provider, &found.inventory)
            .missing_files
            .is_empty();
        let existing = found.directory.filter(|_| complete);
        let destination = self.native_download_dir(feature, provider).ok();
        // Files elsewhere are not reused by a download into this destination.
        let download_bytes = route
            .models
            .iter()
            .filter(|file| {
                !destination
                    .as_ref()
                    .and_then(|dir| std::fs::metadata(dir.join(&file.filename)).ok())
                    .is_some_and(|metadata| metadata.is_file() && metadata.len() == file.size_bytes)
            })
            .fold(0u64, |total, file| total.saturating_add(file.size_bytes));
        Ok(NativeSetupPlan {
            feature: feature.clone(),
            route: route.clone(),
            destination,
            existing,
            download_bytes,
            available: client.has_adapter(provider),
        })
    }
}
