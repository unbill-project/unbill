______________________________________________________________________

## name: rust-programmer description: >- Program in Rust for this repository. Use when Codex edits Rust code, public Rust APIs, Rust module or item docs, domain types, errors, builders, parsers, state machines, tracing instrumentation, Cargo formatting, or Rust lint cleanup.

# Rust Programmer

## Inline Documentation

Inline Rust documentation is the canonical documentation source for this repository.
Use `//!` module docs and `///` item docs.
Keep key understandings, new findings, invariants, and design rationale close to the relevant code.
Write `/// Note: ` to explain why unusual design choices or compromises exist.
Do not rely on a standalone `docs/` tree as the canonical source.

Actively write documentation for the program.
Make sure all public APIs are documented.
All written documentation must be concise, clear, accurate, and written in English unless instructed otherwise.
Keep sentences short. The line budget is 120 characters.
Break Markdown prose at natural punctuation boundaries or conjunctions.
A line may slightly exceed the line-length budget when that makes the text read better.
Do not use emojis unless strictly necessary.
Add bold text only if it emphasizes truly valuable information.

Prefer direct definitions over defensive framing.
Define what the system does before explaining limits or exclusions.
Keep definition-by-negation to a minimum.
Use it only when a nearby confusion is likely and the contrast is genuinely clarifying.
Avoid prose that reads like a rebuttal, disclaimer, or argument with an imaginary reviewer.
When documenting a constraint, state the positive rule first, then the consequence if needed.

## Error Handling

Panic fast.
If some edge case is not specified by design, log and panic.
Never attempt to recover from an error path if it is not absolutely correct.

## Declaration

Prefer declaration instead of manual implementation.
Use `thiserror` for error messages instead of manual implementations.
Prefer `serde` for serialization and deserialization instead of manual parsing and pretty printing.
Prefer derive-style `clap` for command-line argument parsing.

Encode invariants into Rust's type system so the compiler enforces them.
Document invariants per struct, field, function, and method.
Use constructor or builder patterns for creating instances that satisfy invariants.

## Typed Data

Always prefer typed data structures over strings plus parsers.
Prefer introducing a named type over reusing a generic one,
such as `String` or `HashMap`, when the type carries domain meaning.

Include specific variants when creating an error type.
Do not use strings as the error model.
Parse user input into structured data as soon as possible.
Do not use strings to represent states in the software's state machine.
Do not pass strings between internal components when the message could be typed.

Whenever a hashmap of strings is created,
consider whether the keys represent a closed set of fields.
If so, replace it with a struct or a trait object.

## Methods

Prefer structs that pack a group of useful functions.
Prefer methods over functions.
Rust structs have better namespace-like features than Rust modules.
Free functions are acceptable for trait implementations, entry points such as `main`,
or cases where no meaningful receiver exists.

When several functions share the same leading domain argument,
introduce or reuse a small domain type and group the operations as methods.
Prefer the type that owns the main invariant or resource path.
This keeps APIs discoverable and prevents modules from becoming loose function bags.

Mention `self` in the signature when the method is built around the struct type.
Take ownership with `self` when the method is the elimination form of the struct type.
Take `&self` or `&mut self` when the method only needs to borrow the struct.

Use associated functions when the struct is purely a namespace.
Write `fn new` for constructors with no perspective.
Write `fn with_*` for constructors that indicate how the struct is created.

## Builders

For builder patterns, pick receivers based on whether the finalizer must move owned fields out.

If build or finish consumes the builder:

- Use `fn build(self) -> T` for the builder.
- Make all setter methods take and return `self`.
- Use `fn with_*(mut self, ...) -> Self` for easy chaining.

If build can borrow the builder:

- Prefer setters shaped as `fn set_*(&mut self, ...) -> &mut Self`.
- Prefer `fn build(&self) -> T` so the builder can be reused.

Expose an associated entry point named `fn new(required, ...)`.
Use `with_*` or `set_*` names consistently for optional configuration.

## Observability

When adding new features, record and observe details with the `tracing` crate.
Use `trace` level at the beginning and end of major interface calls to record arguments.
Use `debug` for task-specific debug output.
Remove task-specific `debug` output when the problem is solved.

## Imports

Avoid nested `super::` imports.
Use absolute paths in that case.

## Verification

Run `cargo fmt` and `cargo clippy` before committing Rust changes.
