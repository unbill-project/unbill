// Formally verified settlement math for unbill.
// This crate contains only pure arithmetic — no dyn, no errors, no IO.
// Verified with Creusot: `cargo creusot -p unbill-console-verified`

pub mod settlement;
