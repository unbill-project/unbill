# Unbill UI Leptos

Leptos + Tauri implementation of the shared Unbill UI model. See `../DESIGN.md` for screens, features, and behavior. This document covers only what differs from the shared model.

## Visual system

The desktop UI uses a native utility style. It favors dense panes, list rows, simple tables, compact toolbars, restrained color, and system typography. The app shell fills the viewport and avoids decorative backgrounds, oversized cards, nested cards, preview mockups, and ornamental gradients. Interactive controls use stable dimensions so row hover states, status text, and action buttons do not shift layout.

Navigation and utility controls use Lucide icons in square icon-only buttons. These controls include back, more, close, sync, share, copy, row add, and editor save actions. Each icon-only control keeps an accessible label, hover title, and hidden text equivalent. Primary workflow actions remain text-first so actions such as creating ledgers, adding users, joining ledgers, and submitting forms are explicit.

## Layout mode selection

Mode is determined from `window.innerWidth` on startup and whenever the window is resized. The breakpoint is 1200 px: narrower windows use compact mode, wider windows use ranger mode.

## Compact navigation stack

Priority order (first match wins):

1. Detail
1. Bills
1. Ledgers

The settings popup follows the shared model: full-screen overlay in compact mode, floating overlay in ranger mode.

## Ranger column assignment

Column one is always the ledgers list. Column two shows the bills view. Column three shows the detail view or a placeholder. Ranger mode is only used when there is enough width for comfortable stable pane minimums. The settings popup floats as an overlay above all three columns.

## Settings overlay

Settings is one overlay with Device Settings and Ledger Settings tabs. Opening settings from the global device action activates Device Settings and does not require an active ledger. Opening settings from a ledger activates Ledger Settings and preselects the active ledger. When there is no active ledger, Ledger Settings preselects the first available ledger. Changing the ledger selector inside the overlay changes only overlay state and does not change the visible Bills page selection behind it.

## Status strip

A fixed strip above all content shows the latest status or error message and a "Working" chip while any async operation is in flight. Hidden when idle.

## Ledger sync actions

Ledger detail data includes the peer devices authorized for that ledger. The Ledger Settings page renders those peers in a ledger-scoped sync section so operators can trigger one-shot sync without leaving the ledger context. The button calls the shared sync action by peer device ID, then refreshes bootstrap state and the selected ledger detail.

## User actions

Device Settings shows the local device ID, known sync peers, and join actions. Ledger Settings adds ledger users by presenting the picker of users from all ledgers on the device (excluding those already in the selected ledger) alongside a create-new option. Ledger invitation URLs live in overlay state and are copied through the Tauri clipboard bridge.
