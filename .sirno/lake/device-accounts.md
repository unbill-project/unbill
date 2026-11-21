---
core.desc: Local accounts, ledger grants, password and session lifecycle.
core.name: Device Accounts
core.category:
  - core.concept
core.belongs:
  - unbill-device
core.refines:
  - security-model
---

Each device has its own accounts, grants, and sessions. Account names are unique
within one device. Registration is open to anyone; an account initially has no
ledgers. `admin` is an ordinary account created only while upgrading a database
whose old schema had no account tables. Its initial password is `admin`, and it
receives grants for all legacy ledgers. A newly created database has no accounts.

Passwords are stored as independently salted Argon2id PHC hashes. Login issues
a random 256-bit bearer token. Storage persists only its SHA-256 digest so
sessions survive client restarts. Logout deletes one session. A password change
requires the old password and deletes all sessions for that account. Deletion
cascades to sessions and grants. Local maintenance may delete an account, reset
its password, or revoke a grant; these operations are absent from RPC and HTTP.

`UnbillDevice` owns registration, login, session resolution, and the Iroh
endpoint. Its `account_view(account_name)` constructor is trusted process-local
code and does not authenticate the name. External channels must resolve a
session first and construct the view from the server-returned name. The view
checks a fresh grant for each ledger list, metadata write, archive operation,
Automerge round, invitation, join, and share. A new ledger grants only its
creator. Local sharing grants an existing account; cross-device joining grants
the account that accepted the invitation on the joining device. Joining an
already present ledger merges document state and does not duplicate its NodeId.
Accounts and grants are device-local and never enter the ledger CRDT or peer
sync protocol. Peer sync remains a device operation after client authentication.

RPC sessions are scoped to each socket connection, with a user-private token
file beside the socket for restart. HTTP uses the bearer token on every
protected request. Browser and mobile clients retain the token locally and
resume it on startup. Account switching logs out before logging in as another
account.
