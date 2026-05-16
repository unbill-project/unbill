# unbill-asymmetric-channel — Implementation

## Layout

- `lib.rs` — `AsymChannel` trait, `AsymChannelEvent` enum
- `local.rs` (feature `local`) — `LocalAsymChannel`: wraps `unbill_device::UnbillDevice`; forwards all trait calls directly; translates `ServiceEvent` into `AsymChannelEvent` via a background task
- `rpc.rs` (feature `rpc`) — tarpc service definition (`AsymChannelService`), `rpc::serve` (server loop over a Unix local socket), and `RpcAsymChannel` (client); event subscription uses a per-connection queue polled via `poll_events`
- `http.rs` (feature `http`) — `HttpAsymChannel`: REST client pointing at an `unbill-server` instance; mirrors the `AsymChannel` methods as HTTP calls

## LocalAsymChannel

`LocalAsymChannel::open` creates a `UnbillDevice` and a separate `broadcast::Sender<AsymChannelEvent>`. A `tokio::spawn` background task converts `ServiceEvent` to `AsymChannelEvent` and fans them out. `accept_loop` delegates directly to `UnbillDevice::accept_loop`.

## RPC implementation

The tarpc service uses `String` for error returns so all types are serializable. The transport is `interprocess` Unix local sockets, not TCP. Event delivery uses a polling model: the server accumulates events per connection in a `Mutex<Vec<WireEvent>>`; clients call `poll_events` periodically and feed their own broadcast channel.

## HTTP implementation

`HttpAsymChannel` wraps a `reqwest::Client` with a base URL and bearer token. Each `AsymChannel` method maps to one REST endpoint on `unbill-server`. `subscribe_to_server` returns a receiver from a broadcast channel that is never written to — push events are not yet delivered over HTTP. Callers receive an empty stream.

## Dependencies

- `unbill-model` — domain types
- `unbill-device` (feature `local`) — `UnbillDevice`
- `tarpc` (feature `rpc`) — RPC framework
- `reqwest` (feature `http`) — HTTP client
