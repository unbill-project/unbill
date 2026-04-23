# Unbill UI Leptos

Leptos + Tauri implementation of the shared Unbill UI model. See `../DESIGN.md` for screens, features, and behavior. This document covers only what differs from the shared model.

## Layout mode selection

Mode is determined from `window.innerWidth` and updates as the window is resized. The breakpoint is 1080 px: narrower windows use compact mode, wider windows use ranger mode.

## Compact navigation stack

Priority order (first match wins):

1. Bill editor
2. Ledger Settings (requires an active ledger)
3. Device Settings
4. Active ledger
5. Ledgers list

## Ranger column assignment

Column one is always the ledgers list. When Device Settings is open it occupies column two and column three renders a placeholder. Otherwise column two shows the active ledger and column three shows the bill editor, Ledger Settings, or a placeholder.

## Status strip

A fixed strip above all content shows the latest status or error message and a "Working" chip while any async operation is in flight. Hidden when idle.
