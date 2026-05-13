# unbill-ui-components

Reusable Leptos UI components for unbill frontends.

## Contract

- Components are pure Leptos — no Tauri IPC, no HTTP, no unbill-core imports.
- All operations that touch external state (loading data, submitting forms, navigating) are passed as `Callback` props. The caller wires them to whatever backend is in use (Tauri, mock, test stub, etc.).
- Components own only local UI state: expanded/collapsed, hover, validation feedback, pending spinner.
- Data types used in props and events are defined in this crate. They are plain structs — no domain-model imports.
- Bill-editor custom share weights are raw draft text until submission and may be simple addition or subtraction expressions that resolve to a positive whole-number weight.

## Structure

```
src/
  lib.rs          re-exports all public modules
  button.rs       ActionButton, IconButton, ListRow, TagPill
  input.rs        CurrencyCombobox
  layout.rs       ScreenFrame, EmptyColumn, SectionCard, FieldBlock, ModalSheet
  status.rs       StatusStrip
  bill.rs         bill-related components
  bill_editor.rs  bill-editor draft data, validation, and page component
  user.rs         user-related components
  ledger.rs       ledger-related components
  settlement.rs   settlement-related components
```

## Layout components

**ScreenFrame** is the outermost container for a full-height screen column. It accepts an optional `header: Option<AnyView>` and an optional `footer: Option<AnyView>`. When `header` is `Some`, the caller passes an `AnyView` fragment containing three flex children: `div.screen-leading` (navigation control), `div.screen-copy` (title text), and `div.screen-trailing` (action control). The topbar background and border are applied by the component; callers supply only the content. When `header` is `None`, no topbar is rendered.

**SectionCard** groups related form fields or list items under a title. It has no background or border of its own — only a horizontal separator between the header row and the content body.

**EmptyColumn** renders a single short text centered in the available space. It has no title, card, or decorative wrapper.

## Rules

- A component that submits a form calls `on_submit: Callback<FormData>` and returns to idle state; the caller decides what happens next.
- Error and loading states are expressed through the signal values, not through separate props.
