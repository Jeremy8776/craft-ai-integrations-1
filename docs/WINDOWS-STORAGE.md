# Windows installation and AI storage

This is a proposed integration policy for review, not a released installer guarantee. The
[Craft installation policy proposal](https://github.com/storytold/craft-libs/pull/371)
is being reviewed separately. This repository supplies libraries; it does not own the app MSIs
or the launcher installer. Each installer and host must enforce and test its own behaviour.

## Separate installed programs from a person's AI data

- An app's installation folder, whether per-user, per-machine or custom, is not the owner of
  the shared AI store or separately downloaded models. Do not place those assets under an
  app's installation folder as a consequence of an install scope choice.
- On Windows the shared AI store normally lives at `%LOCALAPPDATA%\ArtCraft\ai`, and the
  suggested model root is `%LOCALAPPDATA%\ArtCraft\models`. Both are per Windows user, even
  when an app was installed for everyone. `CRAFT_AI_DIR` overrides the AI store and makes
  its suggested models folder a child of the override. Hosts must not infer access for other
  Windows accounts or migrate existing files when installation scope changes.
- The model root is a suggestion, not an instruction to move an existing folder. Preserve
  saved and custom paths. Downloaded models are persistent user assets, not disposable cache.
  A normal app update, uninstall, settings reset or cache cleanup must not delete them or
  other apps' data. Bundled files owned by an installer are separate from downloaded models.
- Discovery of another program's models must be read-only. Do not download into, modify or
  remove those files merely because they were discovered. Before writing to a folder, obtain
  the person's explicit choice and show the destination; fail visibly if it is unavailable.
- Installers must not own, relocate or remove the shared AI store, model folders or their
  contents. A host that offers separate data removal needs an explicit, scoped user action;
  removing an app is not that action.

## Current implementation and outstanding work

`craft-ai-native::paths` supplies the Windows locations above. `FolderDesk::remove` forgets a
folder without deleting its files, and an imported folder from another program cannot be set
as the download default. A native setup plan retains its proposed destination for its worker.
These are code-level observations, not proof of complete installer behaviour.

There is an important distinction in the current API: `FolderDesk::add` registers a manually
chosen folder and makes it the default if there is no chosen default. It is not a discovery-only
link operation. `ensure_default` can also promote the first chosen folder when no default is
marked. Hosts must not describe that operation as read-only linking with no download-target
side effect. A separate discovery-only link and explicit set-default flow still need to be
implemented and tested end to end before making that claim.

The current Linux suggested model folder is derived from the config directory, while the
proposed suite policy uses `XDG_DATA_HOME`. This Windows policy does not migrate or change
Linux paths; reconciliation needs a separate compatibility review that preserves existing
choices. Shared AI settings under local application data are also a separate cross-app storage
review question, not proof of conformance with the per-app settings policy.

## Release checks

Before shipping an adopting app, test both install scopes, custom folders, upgrade, repair and
uninstall, including a second Windows account. Verify the exact stored model paths and file
contents remain unchanged, and that no installer or cleanup removes them. Test an unavailable
chosen drive without falling back silently. Exercise discovery, explicit download destination,
and another app using the same user's shared store. Scripted library tests alone do not prove
these installer, multi-user or host integration cases.
