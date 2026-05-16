# Unbill Daemon

Long-running background process that owns the local device state and exposes it to other processes on the same machine.

## Responsibilities

- Holds the exclusive `FsStore` file lock for the data directory, preventing data corruption from concurrent writes.
- Runs the Iroh network endpoint (`accept_loop`) to accept peer sync and join requests from remote devices.
- Serves the local RPC socket so CLI processes and other local clients can issue commands without directly touching storage.

## Contract

- Prints `listening on: <node_id>` to stdout once the Iroh endpoint is fully bound and the RPC socket is accepting connections. This line is the readiness signal for automated tooling.
- All other output goes to stderr.
- Runs until killed; exits on a fatal network or storage error.
