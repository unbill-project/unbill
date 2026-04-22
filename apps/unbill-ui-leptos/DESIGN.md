# Unbill UI Leptos

Leptos + Tauri implementation of the shared Unbill UI model. See `../DESIGN.md` for screens, features, and behavior. This document covers only what differs from the shared model.

## Layout

- compact mode shows one screen at a time; ranger mode shows three adjacent columns
- the mode is chosen from window width: narrow windows use compact mode, wider windows use ranger mode
- column one: ledgers; column two: active ledger or device settings; column three: bill editor or ledger settings
- device settings occupies column two and clears column three in ranger mode
- ledger settings occupies column three beside the active ledger in ranger mode
- in compact mode, device settings and ledger settings are full-page navigations rather than overlays
