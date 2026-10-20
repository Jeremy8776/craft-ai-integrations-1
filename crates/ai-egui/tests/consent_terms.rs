//! Consent durations exercised through the actual prompt, without a native window or a key.
use craft_ai_core::{AppId, CapabilityId, ConnectionId, ConsentTerm, Disclosure, Locality};
use craft_ai_egui::{ConsentPromptState, TimedConsentChoice, consent_prompt_with_terms};
use egui_kittest::{Harness, kittest::Queryable};

struct State {
    disclosure: Disclosure,
    prompt: ConsentPromptState,
    last: Option<TimedConsentChoice>,
}

fn harness() -> Harness<'static, State> {
    Harness::new_ui_state(
        |ui, state: &mut State| {
            if let Some(choice) =
                consent_prompt_with_terms(ui, &state.disclosure, "An app", &mut state.prompt)
            {
                state.last = Some(choice);
            }
        },
        State {
            disclosure: Disclosure {
                app: AppId::new("first-app").unwrap(),
                connection: ConnectionId::new("cloud").unwrap(),
                capability: CapabilityId::parse("text.complete").unwrap(),
                locality: Locality::Remote,
                destination: "https://example.com".into(),
                items: vec![],
                param_names: vec![],
            },
            prompt: ConsentPromptState::default(),
            last: None,
        },
    )
}

#[test]
fn allow_defaults_to_this_job_and_nothing_is_agreed_before_clicking() {
    let mut h = harness();
    h.run();
    h.get_by_label("This time only");
    assert_eq!(h.state().last, None);
    h.get_by_label("Allow").click();
    h.run();
    assert_eq!(
        h.state().last,
        Some(TimedConsentChoice::Allow(ConsentTerm::ThisTime))
    );
}

#[test]
fn choosing_a_duration_returns_that_duration() {
    for (label, term) in [
        ("For one hour", ConsentTerm::OneHour),
        ("For one day", ConsentTerm::OneDay),
        ("Until I turn it off", ConsentTerm::UntilRevoked),
    ] {
        let mut h = harness();
        h.run();
        h.get_by_label(label).click();
        h.run();
        h.get_by_label("Allow").click();
        h.run();
        assert_eq!(h.state().last, Some(TimedConsentChoice::Allow(term)));
    }
}

#[test]
fn refusing_does_not_return_a_permission() {
    let mut h = harness();
    h.run();
    h.get_by_label("For one day").click();
    h.run();
    h.get_by_label("Don't send").click();
    h.run();
    assert_eq!(h.state().last, Some(TimedConsentChoice::Deny));
}

#[test]
fn a_changed_destination_does_not_inherit_the_longer_duration() {
    let mut h = harness();
    h.run();
    h.get_by_label("Until I turn it off").click();
    h.run();
    h.state_mut().disclosure.destination = "https://another.example.com".into();
    h.run();
    h.get_by_label("Allow").click();
    h.run();
    assert_eq!(
        h.state().last,
        Some(TimedConsentChoice::Allow(ConsentTerm::ThisTime))
    );
}
