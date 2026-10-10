---
core.name: Deployment Topologies
core.desc: The supported ways to colocate or separate devices, consoles, and servers.
core.category:
  - core.concept
core.belongs:
  - unbill
  - applications
core.refines:
  - device-console-split
---

Single-process colocated deployment runs a device and console in the same process.
This fits mobile app sandboxes.
The asymmetric channel is in-process memory communication.

Local two-process desktop deployment runs `unbill-daemon` as the device process.
CLI, TUI, Tauri, and Apple macOS/Catalyst connect over the local RPC socket.
The daemon acquires the desktop data-directory lock when opening its SQLite Store.
The daemon holds storage and sync after the UI exits,
and multiple local consoles can connect concurrently.

Remote access runs `unbill-server` as the device-facing HTTP backend.
Browser and other remote consoles connect through the HTTP asymmetric channel.

Multiple devices are a primary scenario.
A person may use a laptop, VPS, and phone at the same time,
with all devices connected by symmetric channels for peer sync.
