"""Deliberate-bug cases for group `fr`: the first-run setup flow.

Loaded by mutation_check.py. Each case breaks one rule and names the test that should notice.
Format: (name, file, old, new, package, test target, test filter).
"""
C = 'crates/ai-client/src/'
F = 'crates/ai-features/src/'
CL = 'craft-ai-client'
FE = 'craft-ai-features'
EG = 'craft-ai-egui'

FR = [
    ('agreement-always-asked', F + 'recipe.rs', 'self.models.iter().any(|m| m.licence.accept_required)', 'true', CL, 'first_run', 'asks_for_agreement_only_when'),
    ('download-ignores-a-missing-agreement', C + 'setup_flow.rs', 'if (self.plan.needs_agreement() && !self.accepted)', 'if (false && !self.accepted)', CL, 'first_run', 'asks_for_agreement_only_when'),
    ('start-always-downloads', C + 'setup_flow.rs', 'if self.plan.existing_directory().is_some() {\n            SetupTask::CheckExisting', 'if false {\n            SetupTask::CheckExisting', CL, 'first_run', 'start_checks_files'),
    ('suggested-folder-replaces-a-chosen-default', C + 'folders.rs', 'if let Some(default) = chosen.iter().find(|f| f.default) {', 'if let Some(default) = chosen.iter().find(|f| f.default && false) {', CL, 'first_run', 'suggested_folder_is_made_only_when'),
    ('adopt-forgets-the-kept-choice', C + 'local.rs', '            self.view(&role)?.bound\n', '            None\n', CL, 'first_run', 'adopting_checked_files'),
    ('widget-always-shows-the-tick-box', 'crates/ai-egui/src/native_setup.rs', 'if needs_agreement {', 'if true {', EG, 'native_setup_first_run', 'no_licence_to_accept'),
]
