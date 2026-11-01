"""Regression guards for the contract fixes already merged in craft-libs.

Do not import the runner here: it executes immediately. Cases use its public tuple format and
name a focused runtime test so a compile error can never count as evidence.
"""
CORE = 'crates/ai-core/src/'
DRAFT = CORE + 'draft.rs'
ROLE = CORE + 'role.rs'
M = 'craft-ai-core'
CL = 'craft-ai-client'
CA = [
    ('late-model-answer-taken', DRAFT,
     'if address.trim() != self.endpoint.trim() {', 'if false {',
     M, 'draft_provenance', 'a_test_that_answers_after_the_address_moved_on'),
    ('probe-address-forgotten', DRAFT,
     'self.models_for = Some(self.endpoint.trim().to_owned());', 'self.models_for = None;',
     M, 'draft_provenance', 'new_draft_discards_models'),
    ('model-address-not-checked', DRAFT,
     '.is_none_or(|origin| origin == address)', '.is_none_or(|_| true)',
     M, 'draft_provenance', 'returning_to_original_address'),
    ('preset-address-forgotten', DRAFT,
     'models_for: (!models.is_empty()).then(|| preset.endpoint.trim().to_owned()),',
     'models_for: None,', M, 'draft_provenance', 'preset_models_belong'),
    ('abort-probe-uses-unconditional-update', 'crates/ai-client/src/manage.rs',
     'self.shared.try_update(|file| {', 'self.shared.update(|file| {',
     CL, 'saved_probe_abort', ''),
    ('abort-callback-still-rewrites-existing-file', 'crates/ai-native/src/shared.rs',
     'Err(error) => return Ok(Err(error)),',
     'Err(error) => { self.write(&store, extras)?; return Ok(Err(error)); },',
     'craft-ai-native', 'shared_try_update', 'callback_error_preserves_raw_nested_settings_and_discards_mutations'),
    ('abort-callback-still-creates-missing-file', 'crates/ai-native/src/shared.rs',
     'Err(error) => return Ok(Err(error)),',
     'Err(error) => { self.write(&store, extras)?; return Ok(Err(error)); },',
     'craft-ai-native', 'shared_try_update', 'callback_error_does_not_create_a_missing_store'),
    ('loaded-model-address-forgotten', DRAFT,
     'models_for: c.endpoint.as_deref().map(|e| e.trim().to_owned()),', 'models_for: None,',
     M, 'draft_provenance', 'changing_model_contents'),
    ('loaded-address-not-trimmed', DRAFT,
     'models_for: c.endpoint.as_deref().map(|e| e.trim().to_owned()),',
     'models_for: c.endpoint.clone(),', M, 'draft_provenance', 'whitespace_in_a_saved_address'),
    ('saved-probe-ignores-changed-settings', 'crates/ai-client/src/manage.rs',
     'if *saved != connection {', 'if false {',
     CL, 'saved_probe_writeback', 'changed_endpoint_keeps_new_models_and_returns_a_stale_error'),
    ('saved-probe-compares-only-endpoint', 'crates/ai-client/src/manage.rs',
     'if *saved != connection {', 'if saved.endpoint != connection.endpoint {',
     CL, 'saved_probe_writeback', 'every_changed_connection_field_prevents_writeback'),
    ('picker-ignores-required-input', ROLE,
     '.all(|i| role.inputs.contains(i))', '.all(|_| true)',
     M, 'required_inputs', 'picker_does_not_offer'),
    ('binding-ignores-required-input', ROLE,
     'spec.requires.iter().find(|i| !role.inputs.contains(i))', 'spec.requires.iter().find(|_| false)',
     M, 'required_inputs', 'binding_refuses'),
    ('app-picker-ignores-required-input', ROLE,
     '.all(|i| role.inputs.contains(i))', '.all(|_| true)',
     CL, 'required_inputs', 'picker_omits'),
    ('app-status-and-plan-ignore-required-input', ROLE,
     'spec.requires.iter().find(|i| !role.inputs.contains(i))', 'spec.requires.iter().find(|_| false)',
     CL, 'required_inputs', 'status_and_plan'),
]
