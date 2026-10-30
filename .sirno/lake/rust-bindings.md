---
core.name: Rust Bindings
core.desc: The native Rust Console interface shared by Cargo consumers and foreign-language adapters.
core.category:
  - core.concept
core.belongs:
  - workspace-layout
---

The unbill Cargo package lives in bindings/rust. It owns the public Console API shared with Swift and Python: constructors, operations, records, errors, observer traits, and subscription handles. Rust callers invoke ordinary Rust methods and the package has no UniFFI dependency.

The public interface preserves the existing Console names, string identifiers, data shapes, synchronous operations, and asynchronous sync_once and observe_ble methods. Console owns its Tokio runtime; synchronous calls and final Console release occur outside async runtime contexts, and callbacks forward work before invoking synchronous Console methods.

Construction selects either an in-process SQLite device or an existing daemon through RPC. Mobile targets omit RPC at compilation. Storage ownership, runtime tasks, projections, and subscriptions retain their existing behavior.

unbill-console remains the underlying orchestration library. bindings/ffi contains unbill-ffi, which adapts the unbill interface through UniFFI records, errors, callbacks, and thin forwarding methods. Business operations and projection assembly live only in the Rust binding. The Python package remains under bindings/python.

The Cargo package inherits the workspace version. Registry publication requires the supporting library crates and versioned dependencies; applications and the UniFFI bridge are not part of the Rust package.

The Rust packaging command selects the Rust binding and its local library dependencies,
prepares the license catalog snapshot, and asks Cargo to package and verify them together.
Publication is a separate explicit option. Registry archives must build without paths
back into the checkout. CI checks packaging and the absence of UniFFI from the Rust
binding's normal and build dependency graph.
The packaging helper requires a clean source checkout by default, before staging
the generated catalog. An explicit allow-dirty option permits development builds.
