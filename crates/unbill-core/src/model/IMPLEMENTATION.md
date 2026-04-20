# model — Implementation

The module is split by semantic type: IDs, time, currency, node identity, invitation tokens, users and devices, and bills.

`autosurgeon` traits are derived on the shared ledger structs so they can hydrate from and reconcile back into Automerge. Input-only helper types such as `NewBill`, `NewUser`, and `NewDevice` stay outside the persisted schema.

The design goal is to make illegal states harder to express at compile time and keep serialization details close to the types they belong to.
