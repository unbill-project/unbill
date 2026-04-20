# unbill-desktop — Implementation

The current app is a small React/Vite shell with custom window chrome in `src/App.tsx`. The richer desktop architecture described for later milestones is not implemented here yet.

When this app grows, it should keep one typed Tauri boundary, refresh from backend events, and avoid duplicating ledger logic from Rust.
