# Mobile feature parity

## Problem

Desktop and web have most features. Android and iOS browse, rename, move, delete and download, but
they cannot upload, create folders, search, restore from the trash, or receive live updates. Status
per client, as of 2026-11-06 (checked in source, not by running the apps):

| Feature | Desktop | Web | Android | iOS |
| --- | --- | --- | --- | --- |
| Sign in with password | Yes | Yes | Yes | Yes |
| Sign in with SSO | Yes | Yes | Yes | Yes |
| Stay signed in between launches | Yes | Yes | Yes | Yes |
| Sign out | Yes | Yes | Yes | Yes |
| Register this device | Yes | Yes | Yes | Yes |
| Device list (rename, sign out devices) | Yes | Yes | No | No |
| Browse folders | Yes | Yes | Yes | Yes |
| Breadcrumb or parent navigation | Yes | Yes | Yes | Partial: back navigation only |
| Sort by name, size and date | Yes | Yes | Partial: folders first, then name | Partial: folders first, then name |
| Select several items | Yes | Yes | Yes | Yes |
| Details panel (created, edited on, device) | Yes | Yes | Yes | Yes |
| Live updates while browsing | Yes | Yes | No | No |
| Create folder | Yes | Yes | No | No |
| Rename | Yes | Yes | Yes | Yes |
| Move, with conflict choice | Yes | Yes | Yes | Yes |
| Delete to bin | Yes | Yes | Yes | Yes |
| Trash: list and restore | Yes | Yes | No | No |
| Search | Yes | Yes | No | No |
| Share with links or users | No | No | No | No |
| Upload files | Yes | Yes | Partial: auto-upload only, no file picker | Partial: auto-upload only, no file picker |
| Drag and drop to upload | No | Yes | n/a | n/a |
| Resumable uploads | Yes | Yes | Yes | Yes |
| Download a file | Yes | Yes | Yes | Yes |
| Save to a chosen location | Yes | Partial: the browser picks the folder | Yes | Yes |
| Keep offline copies on the device | Yes | Yes | Yes | Yes |
| Progress for transfers | Yes | Yes | Yes | Yes |
| Folder sync from this device | Yes | No | Auto-upload one way, offline folders both ways | Auto-upload one way, offline folders both ways |
| Quick upload of camera photos | No | No | Yes | Yes |
| Transfers continue with the app closed | n/a | n/a | Auto-upload: yes | Auto-upload: when iOS allows |

Sources: `clients/desktop/src/browser`, `clients/desktop/src/transfers.rs`, `clients/sync-core`,
`clients/web/src/upload.rs`, `clients/web/src/pages/browser`, `clients/mobile/androidApp`,
`clients/mobile/iosApp`, `clients/mobile-core/src/lib.rs`.

## Scope

Close the gaps for Android and iOS that the desktop and web clients already cover. The
`mobile-core` crate already exposes the browse, rename, move, delete and download calls. New
features need the calls added there, so both apps get them. Sharing and folder sync have their own
write-ups and are only listed here.

## Proposal

Work in this order:

1. **Upload files** (both apps). Use the chunked upload API (`POST /uploads`, `PATCH /uploads/{id}`
   with `Upload-Offset`, resume from `HEAD /uploads/{id}`). Export it from `mobile-core`, then
   add a file picker on each platform. This is the main reason to have the app.
2. **Create folders** (`POST /folders`) and **trash** (list and restore, `GET /trash`,
   `POST /trash/restore`). Both are small calls.
3. **Search** (`GET /search`).
4. **Live updates** through the websocket (`/api/v1/ws`), which desktop and web already use.
5. **Medium items**: a device list screen, a sort choice, and an iOS breadcrumb trail.

Each step adds `mobile-core` calls first, then Android and iOS screens. Keep parity by checking this
table after each step.

## Open questions

- Upload from the phone: start with the file picker only, or also a folder picker? Folder upload
  is available on desktop.
- Is a delete from the phone a move to the bin, as now? (Yes, unless the trash UI makes it
  confusing.)
- Should iOS and Android share one data flow for search and trash, or keep the native list state?
