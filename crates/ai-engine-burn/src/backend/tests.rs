use super::*;

/// A folder holding a saved-weights-looking encoder and decoder with `body` after the first bytes.
fn folder_with(head: &[u8], body: &[u8]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for name in [ENCODER_FILE, DECODER_FILE] {
        let mut bytes = head.to_vec();
        bytes.extend_from_slice(body);
        std::fs::write(dir.path().join(name), bytes).unwrap();
    }
    dir
}

/// The error text a person would read says what to do and names no part of the machinery.
fn assert_plain(error: &AiError) {
    assert!(
        error.message.contains("Set up AI"),
        "says what to do: {}",
        error.message
    );
    for word in ["Burn", "wgpu", "GPU", "burnpack", "panic", "tensor"] {
        assert!(
            !error.message.contains(word),
            "{word} in: {}",
            error.message
        );
    }
}

/// A mask the best proposal fills with its own index, so the winner can be told from its logits.
fn masks_filled_by_index() -> Vec<f32> {
    (0..PROPOSALS)
        .flat_map(|i| vec![i as f32; MASK_SIDE * MASK_SIDE])
        .collect()
}

/// The proposal the model rates highest is the one returned, with its own score.
#[test]
fn the_best_rated_proposal_is_chosen_with_its_score() {
    let got = choose_proposal(&masks_filled_by_index(), &[0.2, 0.9, 0.5, 0.1]).unwrap();
    assert_eq!(got.score, 0.9);
    assert_eq!(got.logits.len(), MASK_SIDE * MASK_SIDE);
    assert!(got.logits.iter().all(|l| *l == 1.0), "the second mask");
    let first = choose_proposal(&masks_filled_by_index(), &[f32::NAN; 4]).unwrap();
    assert!(
        first.logits.iter().all(|l| *l == 0.0),
        "no usable score falls back to the first"
    );
}

/// An answer of the wrong length means the files are not this model: an error, never a slice
/// out of range.
#[test]
fn an_answer_of_the_wrong_size_is_an_error_not_a_panic() {
    let masks = masks_filled_by_index();
    for (masks, scores) in [
        (&masks[..masks.len() - 1], vec![0.1; PROPOSALS]),
        (&masks[..], vec![0.1; PROPOSALS - 1]),
        (&masks[..], vec![0.1; PROPOSALS + 1]),
        (&[][..], Vec::new()),
    ] {
        let error = choose_proposal(masks, &scores).unwrap_err();
        assert_eq!(error.kind, ErrorKind::Unsupported);
        assert_plain(&error);
    }
}

/// Only a file that starts with the saved-weights marker is let through to the loader.
#[test]
fn only_a_file_that_starts_like_saved_weights_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let write = |name: &str, bytes: &[u8]| {
        let path = dir.path().join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    };
    assert!(looks_saved(&write("ok", b"NRUB and the rest")));
    assert!(looks_saved(&write("exact", b"NRUB")));
    assert!(!looks_saved(&write("junk", b"not weights at all")));
    assert!(!looks_saved(&write("short", b"NRU")));
    assert!(!looks_saved(&write("empty", b"")));
    assert!(!looks_saved(&dir.path().join("missing")));
    assert!(!looks_saved(dir.path()), "a folder is not a file");
}

/// A panic inside guarded work is a missing answer, and ordinary work is untouched.
#[test]
fn a_panic_in_guarded_work_becomes_no_answer() {
    assert_eq!(guarded(|| 7), Some(7));
    let none: Option<u8> = guarded(|| {
        let empty: Vec<u8> = Vec::new();
        empty[3]
    });
    assert_eq!(none, None);
}

/// Files that are missing, empty, or not saved weights are refused in plain words before any
/// model is built.
#[test]
fn files_that_are_not_saved_weights_are_refused_in_plain_words() {
    let empty = tempfile::tempdir().unwrap();
    assert_plain(&Burn::load(empty.path()).err().unwrap());
    for body in [&b""[..], b"this is not a weights file"] {
        let dir = folder_with(b"", body);
        assert_plain(&Burn::load(dir.path()).err().unwrap());
    }
    // One good-looking file is not enough.
    let half = tempfile::tempdir().unwrap();
    std::fs::write(half.path().join(ENCODER_FILE), b"NRUB1234").unwrap();
    assert_plain(&Burn::load(half.path()).err().unwrap());
}

/// A file that has the right marker but a body that is not the model's weights goes all the way
/// to the weights store and comes back as an error. A bad file must never be a crash.
#[test]
fn a_damaged_weights_file_is_an_error_not_a_panic() {
    let dir = folder_with(MAGIC, &[0u8; 64]);
    let error = Burn::load(dir.path()).err().unwrap();
    assert_eq!(error.kind, ErrorKind::Unsupported);
    assert_plain(&error);
}
