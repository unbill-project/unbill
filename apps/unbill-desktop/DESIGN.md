# unbill-desktop

Early React desktop shell served inside Tauri. It is a presentation layer, not a second implementation of unbill.

## Role

- own desktop presentation and window chrome
- talk to Rust only through typed Tauri commands and events
- treat IDs, settlement, and bill projection as opaque backend data

## Current shape

The app is still minimal. Its long-term scope is ledger browsing, bill editing, and settlement display once the desktop shell is filled out.

## Rule

Business logic stays in Rust. Frontend state is transient UI state only.
