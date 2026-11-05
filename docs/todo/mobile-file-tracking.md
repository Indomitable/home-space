# Mobile file tracking

The mobile apps download a file when it is tapped and show a badge while the copy matches the
server. Nothing keeps that copy up to date or sends edits back. This records what is known for the
change that adds it, which tracks single files rather than the desktop client's whole folders.

## Wanted

A file opened by tapping (not one saved to a chosen location) is tracked: it updates when it
changes on the server, and edits to the local copy are uploaded. Watching the local copy for edits
is a separate step, since viewers currently get read-only access.

## What exists

- Copies are laid out by node id and hash (see "Opened files" in `clients/mobile/README.md`), with
  no database. The folders under `<downloads>/<user id>/` are already a list of the tracked files.
- The badge has only one state so far. Tracking needs more, such as updating, conflict and failed.

## What the research found

- `GET /changes` has no filter by node, but filtering client-side by id is enough. A `Deleted`
  change carries the id, so a copy can be dropped without an index.
- Moving a folder doesn't change the ids of the files inside it, so tracked files survive moves.
- An upload whose `modified_at` is older than the current content is archived as a version rather
  than made current. A rename updates the node's time, so uploading an edit made before a rename
  can land as a version. Compare against the version the copy came from before uploading.

## Open questions

- Background work: reqwest can't run while an iOS app is suspended, so updates either wait for the
  next launch or need a background `URLSession` and Android `WorkManager`.
- Conflicts: whether to follow the desktop client's rules, in `clients/desktop/src/sync`.
