# Unbill UI Design

## Structure

`unbill-ui-leptos` is the shared UI for mobile and desktop shells.

Mobile presents one page at a time.

Desktop presents a three-column ranger layout:
- Column 1: ledgers page
- Column 2: selected ledger page or device settings page
- Column 3: selected bill page or ledger settings page

Each column uses the same page structure, actions, and data logic as the corresponding mobile page.

## Shared Page Frame

Each page uses a vertical stack:
- Top bar with page title, primary context, and trailing actions
- Scrollable content area
- Primary action anchored at the bottom edge or presented as a full-width action row near the bottom of the content

Top bar actions always operate on the current page context.

Lists use large touch rows with a primary label and a compact metadata line when metadata exists.

Selecting a row opens that row's page context. On mobile this pushes a page. On desktop this fills the next column.

## Ledgers Page

### Elements

- Top bar title: `Ledgers`
- Top right `More` button
- Scrollable ledger list
- Bottom `New Ledger` button

Each ledger row shows:
- Ledger name
- Member count
- Latest bill timestamp when at least one bill exists

### Layout

The page opens on the ledger list.

The list fills the full content area.

The `New Ledger` button stays visually separate from the list as the page's primary action.

### Data Logic

- Load all ledgers available on the device.
- Sort ledgers by latest bill timestamp descending.
- Ledgers without bills sort after ledgers with bills and then by ledger name ascending.
- Tapping a ledger opens that ledger page.
- Tapping `More` opens the device settings page.
- Tapping `New Ledger` creates a draft ledger flow and opens the new ledger setup when that flow exists.

## Ledger Page

### Elements

- Top bar title: ledger name
- Top right `More` button
- Scrollable bill list
- Bottom `New Bill` button

Each bill row shows:
- Bill description
- Payment date
- Payer name
- Total amount

### Layout

The page opens with the selected ledger as the active context.

The bill list fills the content area.

The `New Bill` button stays available as the page's primary action.

### Data Logic

- Load all bills for the selected ledger.
- Sort bills by payment timestamp descending.
- Tapping a bill opens the bill page in amend mode with the selected bill loaded into the form.
- Tapping `More` opens the ledger settings page for the current ledger.
- Tapping `New Bill` opens the bill page in create mode with the current ledger preselected.

## New/Edit Bill Page

### Elements

- Top bar title: `New Bill` or bill title
- Save action in the top bar
- Scrollable form

The form shows two sections:
- Payment setup
- Participant setup

Payment setup shows:
- Description field
- Payer picker
- Amount field
- Currency field
- Payment date field
- Optional note field

Participant setup shows:
- Participant list sourced from current ledger members
- Per-participant inclusion toggle
- Split mode control
- Per-participant share editor when the split mode uses custom shares
- Derived per-participant amount summary

### Layout

The page uses a single-column form with section headers and grouped input rows.

The save action remains visible from the top bar.

### Data Logic

- In create mode, initialize fields from ledger defaults and device-local defaults.
- In amend mode, load the selected bill and populate all fields from the persisted bill data.
- The payer picker and participant list both load members from the current ledger.
- The payer must be one of the current ledger members.
- The participant list is derived from the current ledger members and writes to `shares`.
- Equal split mode assigns `1` share to each included participant.
- Custom split mode allows editing the integer share value for each included participant.
- The derived per-participant amount summary recalculates immediately from `amount`, selected participants, and share weights.
- Saving validates required fields, creates a new bill entry in the current ledger, and returns to the ledger page with the saved effective bill selected.
- Saving in amend mode writes the new bill with `prev` containing the superseded bill identifier so the prior bill is no longer effective.

## Device Settings Page

### Elements

- Top bar title: `Device Settings`
- Scrollable saved identity list
- `Add Identity` button
- `Import Ledger` button
- `Scan QR Code` button

Each saved identity row shows:
- Identity name
- Identity identifier

### Layout

Saved identities appear first as the main content block.

Import actions appear as full-width action rows below the identities block.

### Data Logic

- Load all device-local saved identities.
- Sort identities by name ascending.
- Tapping `Add Identity` opens identity creation for a new device-local saved identity and persists it locally on save.
- Tapping `Import Ledger` reads the current clipboard text, parses it as an invitation URL, and opens a join confirmation sheet when the URL is valid.
- Tapping `Scan QR Code` opens the device scanner, reads a QR payload as an invitation URL, and opens a join confirmation sheet when the payload is valid.
- The join confirmation sheet shows the invitation URL, a required device label field, and a confirm action.
- Confirming the join confirmation sheet calls ledger join with the parsed invitation URL and the entered device label.

## Ledger Settings Page

### Elements

- Top bar title: `Ledger Settings`
- Scrollable ledger member list
- `Add Member` button
- `Invitation` section

Each ledger member row shows:
- Member name
- Member identifier in the ledger

The invitation section shows:
- `Device Invitation` button
- Invitation QR code after generation
- `Copy URL` button after generation

### Layout

Ledger members appear first as the main content block.

The invitation section sits below the members list as a dedicated settings block.

### Data Logic

- Load all members in the current ledger.
- Sort members by creation order within the ledger.
- Tapping `Add Member` opens a single-name input flow and appends the new member to the ledger on save.
- Tapping `Device Invitation` generates the invitation URL for the current ledger, stores it in page state, and renders both the QR code and the `Copy URL` button.
- Tapping `Copy URL` writes the generated invitation URL to the clipboard.

## Selection Model

- The selected ledger is the active ledger context across ledger page, bill page, and ledger settings page.
- The selected bill is the active bill context for bill viewing and editing.
- Device settings is a top-level page context and does not require a selected ledger.
- Desktop columns keep the active selection visible across columns.
- On desktop, opening device settings from the ledgers page fills column 2 and clears column 3.
- On desktop, opening ledger settings from the ledger page fills column 3 while column 2 continues to show the selected ledger page.
- Mobile navigation keeps the current selection in page state and restores it when returning to the previous page.
