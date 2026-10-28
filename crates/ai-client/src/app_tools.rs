//! What an app reads from the person's shared AI choices, in one call.
//!
//! Every Craft app does the same first step: build one [`AiClient`] under its own app id, then ask,
//! for each tool in the suite catalogue, whether it can run now. [`AppTools`] is that step, so an
//! app copies no catalogue code and names no model, engine or file.
//!
//! Invariants:
//! - Reading changes nothing. No key is read, nothing is sent and the shared store is not written.
//! - A tool with no choice is `Unavailable` with a reason a person can read. It is never an error
//!   and never falls back to some other model.
//! - The answer is computed on every call, so a choice made in the launcher while the app is open
//!   is seen at once.

use craft_ai_core::{AppId, Role};
use craft_ai_features::builtin;

use crate::{AiClient, RoleStatus};

/// One catalogue tool and whether it can run in this app now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolReport {
    /// The catalogue id, such as `select-subject`.
    pub id: String,
    /// The name a person sees.
    pub title: String,
    /// Ready (with the model it is bound to) or unavailable (with the reason).
    pub status: RoleStatus,
}

/// An app's read-only view of the shared tool choices.
pub struct AppTools {
    client: AiClient,
    roles: Vec<(String, Role)>,
}

impl AppTools {
    /// The real shared store and key stores, for the running app. `app` is the lower-case name the
    /// launcher uses (`photocraft`, `lightcraft`) and never changes: consent and per-app choices
    /// are scoped by it.
    ///
    /// # Errors
    /// Returns the sentence to show when `app` is not a valid app id or the store has no folder.
    pub fn standard(app: &str) -> Result<Self, String> {
        let app = AppId::new(app).map_err(|e| e.to_string())?;
        let client = AiClient::standard(app).map_err(|e| e.message)?;
        Ok(Self::with_client(client))
    }

    /// Any client, for tests and for apps that supply their own key stores or adapters.
    pub fn with_client(client: AiClient) -> Self {
        let roles = builtin()
            .into_iter()
            .filter_map(|feature| feature.role().map(|role| (feature.title, role)))
            .collect();
        Self { client, roles }
    }

    /// Every catalogue tool, in catalogue order, with its status right now.
    pub fn report(&self) -> Vec<ToolReport> {
        self.roles
            .iter()
            .map(|(title, role)| ToolReport {
                id: role.id.as_str().to_owned(),
                title: title.clone(),
                status: self.client.status(role),
            })
            .collect()
    }

    /// The status of one tool by catalogue id. An id the catalogue does not have is `Unavailable`,
    /// so a typo greys a tool out rather than running something else.
    pub fn status(&self, tool: &str) -> RoleStatus {
        match self.roles.iter().find(|(_, role)| role.id.as_str() == tool) {
            Some((_, role)) => self.client.status(role),
            None => RoleStatus::Unavailable {
                reason: "This tool is not part of the suite yet.".to_owned(),
            },
        }
    }

    /// The tool's role, for the `plan` and `start` calls on [`client`](Self::client).
    pub fn role(&self, tool: &str) -> Option<&Role> {
        self.roles
            .iter()
            .find(|(_, role)| role.id.as_str() == tool)
            .map(|(_, role)| role)
    }

    /// The client behind the view, for `plan`, `start` and the consent calls.
    pub fn client(&self) -> &AiClient {
        &self.client
    }
}
