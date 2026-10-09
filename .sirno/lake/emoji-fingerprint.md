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

The ledger model continues to calculate the full 256-bit SHA-256 state hash.
The private display helper `fingerprint::sha256_to_emojis` takes that full hash,
uses its first eight bytes (64 bits) as an unsigned big-endian integer, and
encodes that prefix as exactly six emoji symbols separated by single spaces.
Encoding uses the emoji alphabet size as its base and preserves leading zero
digits. This is a short visual comparison aid; it cannot reconstruct the full
hash, and different ledger states can share a fingerprint.

The alphabet is the complete Unicode 17.0 emoji dataset supplied by the pinned
`emojis` 0.9.0 package, expanding skin-tone variants and adding the nine
standalone skin-tone and hair components omitted by the package. All 3,953
symbols are included. It is sorted by UTF-8
string order and deduplicated. The dataset version, ordering, and width are
part of the encoding contract; updating the dependency requires reviewing
encoding compatibility. Symbols can be multi-code-point emoji sequences,
so symbol count differs from UTF-8 byte count and Unicode scalar count.
Separators preserve symbol boundaries for combined sequences.
The pinned alphabet is nonempty, so radix division has a nonzero divisor
and each remainder is a valid alphabet index. Lint exceptions for these
operations remain local and document those bounds.
