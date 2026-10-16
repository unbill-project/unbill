---
core.name: About Page
core.desc: Shared project story and acknowledgements, rendered locally by every frontend.
core.category:
  - core.concept
core.belongs:
  - frontend-ui
  - workspace-layout
core.refines:
  - ui-shared-model
---

About introduces private, local-first expense sharing for small trusted groups, the ongoing software architecture experiment, contributor acknowledgements, and an optional invitation to star the repository.

All editorial text, section ordering, action labels, and external destinations have one source in crates/unbill-about/content.json. Paragraph arrays keep the story editable by region without frontend copies. The transport-free unbill-about crate embeds and decodes that resource for Rust consumers; XcodeGen bundles the same source file for Swift decoding. Frontends own presentation and interaction, not alternate prose. Content never comes from a daemon or network request.

Leptos desktop and browser settings share an on-demand About dialog with native modal focus handling. Apple settings pushes a native scrollable page. Browser connection and Apple startup failure screens also expose About without a service. The TUI provides a keyboard-operated, scrollable About popup; the CLI provides an offline about command with text and JSON output.

Presentation uses restrained brand color, typography, and the existing zipper logo from the root unbill-icon.svg. Leptos embeds that SVG directly; the Apple image set links to the same file, preserving vector data without a separately maintained logo. A light backing keeps the original logo colors legible in both color schemes. It follows the current color scheme, accommodates small windows and larger text, and has no automatic animation, popup, tracking, remote avatars, or live star counts. External links open only after user action. Tauri resolves an embedded action identifier before opening its URL in the system browser. About never appears in, or interrupts, bill entry and settlement.

Contributor acknowledgements thank code, design, documentation, testing, and issue-reporting work and link to the full repository contributor list with all=1. The support action invites a GitHub star. There is no donation solicitation.

Apple's direct Swift compile checks select the SDK for each target. Catalyst
checks also search the macOS SDK's iOSSupport frameworks so SwiftUI resolves
the iOS implementation used by the app, rather than the macOS module.
