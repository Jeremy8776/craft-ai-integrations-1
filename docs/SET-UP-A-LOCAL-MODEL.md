# Set up a built-in model without ComfyUI

This is the shared native setup path, not a complete launcher or in-app screen. A model's files,
permissions and an app's ability to run it are separate checks. Keep the selected route's provider
id throughout. Hosts should reuse the shared plan and session rather than rebuilding approval,
cancellation or stale-answer rules themselves.

## The shared flow

1. Get the `Feature` and chosen `Route::Native { feature, provider }` from `setup_options`.
   The app must actually ship that engine. An unavailable engine has no actionable setup link.
2. Use `setup_offer_native(feature, provider, profile, inventory)` and
   `plan_native(feature, provider, inventory)` for that engine's own files. ComfyUI's checkpoint,
   node packs and memory estimate are not its inventory.
3. Build a `FolderDesk` on the same `SharedStore` as the client. `native_files` scans registered
   folders read only and keeps each candidate model's files in one directory. Its inventory is
   a name/size report, **not a checksum or successful inference result**.
4. For existing files, call `verify_native_files(feature, provider, cancel, progress)` before
   enabling the choice. It checks every declared file and returns the main weights path.
   A failed check never repairs, replaces or moves a person's files.
5. For a download, show `native_download_dir` and the offer's size and licences first. Obtain
   explicit approval and any gated-source token, then call `install_native` with `NativeInstall`.
   It uses only the chosen default folder, never a folder imported as read only from ComfyUI.
   It re-hashes cached same-size files too. Cancellation before starting creates nothing.
6. After installation, verify the files and pass the checked weights path to
   `ToolDesk::enable_native_with(feature, provider, weights)`. This sets the shared choice; it
   does not prove inference. The installed files can remain after a later step fails so a retry
   need not download them again.
7. Apps use `AiClient::status`, `plan`, consent where needed, `start` and polling as usual.
   Enabling a shared binding does not add an engine to an app that was built without it.

## Reusable host state and view

Prefer this path for a launcher or app screen. It freezes the reviewed destination and selected
route, keeps worker lifecycle in shared client state, and emits host actions instead of doing
filesystem or network work during drawing:

1. On a worker, call `FolderDesk::prepare_native_setup(feature, provider, client)` and create a
   `SetupSession` from the returned `NativeSetupPlan`. The plan contains no secret. Render its
   selected engine, model files, byte estimate, destination, licence terms and whether this app
   has a matching adapter. Do not label files as ready inference.
2. Render `native_model_setup(ui, &mut session)` from `craft-ai-egui`. Handle `Work(request)` on a
   worker. For `CheckExisting`, run `request.run(downloader, None, progress)`. For `Download`, get
   any approved gated-source token through the host's secret boundary and borrow it only for
   `request.run`; never put it in the plan or shared UI state.
3. Send progress through `session.report_progress(request.ticket(), file_index, progress)` and the
   result through `session.complete(ticket, result)`. Ignore progress/results the session rejects.
   On replacement, cancellation or window shutdown, cancel the returned request so the worker
   sees the same cancellation flag. A reviewed plan change clears old licence approval.
4. When the session exposes `CheckedNative`, the widget shows that files were checked and offers
   a separate **Use this model** action. The host handles `NativeSetupAction::Use` by explicitly
   applying `ToolDesk::enable_native_with` with the checked feature, provider and weights path.
   Checking or downloading never writes a binding.
5. Use `consent_prompt_with_terms` for a duration-aware disclosure. Build the request with the
   exact app, connection, capability, destination and data kinds; apply only remembered terms to
   the saved ledger. A “this job” grant is temporary and must not be persisted. A configured
   connection is not consent.
6. The host supplies its own chrome, translations, destination/file-picker and safe secret prompt.
   Keep Connections and model Folders as normal host-owned sections, with provider management
   under Advanced. The shared crate does not supply a generic toolbar, a platform file picker or
   a complete capability-first overview screen.

The canonical exercised host flow is in `crates/ai-egui/tests/native_setup_view.rs`; session
lifecycle and stale answers are covered in `crates/ai-client/tests/setup_session.rs`. Follow these
files when adding a host, rather than copying their state rules into that host.

Run scanning, hashing and downloads away from the UI thread. The progress callback names a file
index and reports download or verification progress. Keep tokens out of settings, debug output,
errors and documents. The downloader sends a token only to its configured authentication host.

## Executable examples

- `cargo test -p craft-ai-client --test native_download -j 8`: scripted localhost downloads,
  refusal before fetching when the licence is declined, corrupt-file failure, cached-file reuse,
  and setup once followed by successful jobs in two independent client instances.
- `cargo test -p craft-ai-client --test native_install -j 8`: cancelled startup, unusable recipes,
  existing-file hashes and filenames that try to leave the model directory.
- `cargo test -p craft-ai-client --test folders -j 8`: arbitrary layouts, separate-directory
  refusal, default versus read-only folders and reused-file verification.
- `cargo test -p craft-ai-features --test native_setup --test setup -j 8`: the selected engine's
  exact files, size and licence, without falling back to another engine.
- `python tools/mutation/mutation_check.py ns`: deliberately break these boundaries and prove
  their tests notice. Run exclusively, then rerun clean tests after source restoration.

The two-client example uses `FakeRunner`, tiny synthetic files and a text result. It proves the
shared setup and execution plumbing, not SAM 3 segmentation, app document integration, real model
downloads, hardware acceleration or sandbox access. Those remain distinct live gates.
