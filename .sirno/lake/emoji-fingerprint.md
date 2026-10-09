---
core.name: Emoji Fingerprint
core.desc: A deterministic emoji encoding of SHA-256 values for display by console clients.
core.category:
  - core.concept
core.belongs:
  - unbill-console
---

`UnbillConsole::ledger_emojis` accepts a ledger ID and returns its current
emoji fingerprint. It obtains the console's projected ledger document,
calls `LedgerDoc::state_hash` internally, encodes the result, and returns the
document to the cache. Unknown ledgers return the usual ledger-not-found error.
The public console API exposes the emoji result; callers do not supply a hash
or handle a ledger document. As with other console reads, the fingerprint
reflects the current console projection; callers sync to refresh it from peers.

The private `fingerprint::sha256_to_emojis` helper preserves all
256 bits as a fixed-width, 22-symbol emoji string separated by single spaces.
It interprets the bytes as an unsigned big-endian integer and encodes them
in the base given by the emoji alphabet size, including leading zero digits.

The alphabet is the complete Unicode 17.0 emoji dataset supplied by the pinned
`emojis` 0.9.0 package, expanding skin-tone variants and adding the nine
standalone skin-tone and hair components omitted by the package. All 3,953
symbols are included. It is sorted by UTF-8
string order and deduplicated. The dataset version, ordering, and width are
part of the encoding contract; updating the dependency requires reviewing
encoding compatibility. Symbols can be multi-code-point emoji sequences,
so symbol count differs from UTF-8 byte count and Unicode scalar count.
Separators preserve symbol boundaries for combined sequences.
