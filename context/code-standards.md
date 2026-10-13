# Kiln — Code Standards

## Toolchain
- Stable Rust, pinned in `rust-toolchain.toml`. Edition 2021 or later.
- `cargo fmt` clean. `cargo clippy --all-targets -- -D warnings` clean.
- Every change must pass `cargo test --workspace` before it is considered done.

## Structure
- Small crates, small modules. Prefer files under ~400 lines; split by responsibility.
- Respect the dependency rule in `architecture-context.md`. No cycles, no upward dependencies.
- Public API of each crate is small and documented. Everything else is `pub(crate)` or private.

## Error handling
- No `unwrap()` / `expect()` in non-test code, except for provably impossible states with a comment explaining why.
- Libraries use typed errors (`thiserror`); the binary may use `anyhow` at the top level only.
- User-facing errors are short, specific and actionable (what failed, which file/line, what to do).
- Never let a service error crash the editor; convert it to a status message and a log entry.

## Concurrency
- The editor thread owns state. Services never mutate editor state; they send messages.
- Never block the editor thread: no disk, network, process or heavy parsing work on it.
- Every request that can go stale (LSP, search) carries a version/ID and can be cancelled; drop stale results.
- No global mutable state. No `unsafe` unless unavoidable; every `unsafe` block needs a `// SAFETY:` comment and a test.

## Text and positions
- Internal positions are character offsets in the rope. Convert at boundaries only.
- Always treat text as grapheme clusters for cursor movement and as display width for rendering.
- Never assume one byte = one char = one column.

## Performance
- No allocation in per-keystroke or per-frame hot paths unless measured to be fine.
- Render only the visible range. Cache what is unchanged.
- Anything that can be slow is asynchronous and cancellable.
- Add or update a benchmark when touching a hot path.

## Dependencies
- Adding a crate requires a one-line justification in the PR/commit message and a check of size, maintenance and licence.
- Disable default features when not needed. Prefer pure-Rust crates.

## Testing
- Every ticket ships with tests (see the ticket's "Tests" section).
- Core logic (motions, operators, text objects, undo, selections) is covered by unit tests; buffer operations by property tests (`proptest`).
- UI changes are covered by snapshot tests of rendered frames.
- Behavior is tested through the headless mode (same core + input path, no terminal).
- Bug fixes start with a failing test.

## Style and documentation
- Names are descriptive; avoid abbreviations except well-known ones (`lsp`, `pty`).
- Comments explain *why*, not *what*. Public items have doc comments with at least one sentence.
- Commit messages: imperative, one logical change, reference the ticket (`T07: add status line`).
- No dead code, no commented-out code, no TODOs without a ticket number.

## Config and user data
- Never write outside the Kiln config/state directories or the user's project without an explicit user action.
- Config parsing never fails startup: fall back to defaults and show a warning.
