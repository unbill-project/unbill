# unbill-event — Implementation

A single file, `lib.rs`, containing the `ServiceEvent` enum. Derives `Clone` and `Debug`. No dependencies.

`ServiceEvent` is distributed via `tokio::sync::broadcast` channels created in `UnbillEndpoint` and consumed by `UnbillConsole` and `UnbillDevice`.
