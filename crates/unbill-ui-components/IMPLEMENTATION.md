# unbill-ui-components Implementation

The crate contains pure Leptos components plus small plain data models used by those components. It depends on Leptos, browser utility crates, `serde` for frontend DTO/event structs, and currency metadata for the currency input. It does not import Tauri, HTTP stores, or domain crates.

`bill_editor.rs` owns the shared bill editor draft model. Custom share weights are held as raw text while a form is open. Preview helpers parse drafts tolerantly, including simple addition and subtraction expressions, and the save-request builder performs the strict amount, participant, and share-weight validation used at submission.

`conflict.rs` owns the reusable conflict group panel. Consuming apps map their backend DTOs into plain display items before rendering it. The component stores only the selected bill ID for its group and emits the selected bill ID with the competing bill ID set when the commit button is pressed.

Tests live beside the modules they cover. Pure helpers such as bill-editor validation are unit-tested directly; component rendering remains covered by the consuming frontend crates.
