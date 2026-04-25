# Unbill UI Leptos

Leptos + Tauri implementation of the shared Unbill UI model. See `../DESIGN.md` for screens, features, and behavior. This document covers only what differs from the shared model.

## Layout mode selection

Mode is determined from `window.innerWidth` and updates as the window is resized. The breakpoint is 1080 px: narrower windows use compact mode, wider windows use ranger mode.

## Compact navigation stack

Priority order (first match wins):

1. Detail
2. Bills
3. Ledgers

The settings popup follows the shared model: full-screen overlay in compact mode, floating overlay in ranger mode.

## Ranger column assignment

Column one is always the ledgers list. Column two shows the bills view. Column three shows the detail view or a placeholder. The settings popup floats as an overlay above all three columns.

## Status strip

A fixed strip above all content shows the latest status or error message and a "Working" chip while any async operation is in flight. Hidden when idle.
