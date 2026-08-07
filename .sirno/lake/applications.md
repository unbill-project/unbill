---
core.name: Applications
core.desc: The review neighborhood for Unbill binaries and frontend applications.
core.category:
  - core.concept
core.belongs:
  - unbill
---

Applications adapt the reusable Unbill crates to concrete runtimes.

The CLI provides scripting and end-to-end verification.
The TUI provides a keyboard terminal interface.
The daemon owns long-running local device state.
The server exposes a remote HTTP device API.
The native and remote web frontends render the shared UI model through Leptos.
The Tauri crate hosts the desktop shell and exposes frontend commands.
A native SwiftUI frontend renders the shared UI model on Apple platforms alongside the Tauri shell.
