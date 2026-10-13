# Kiln — Architecture Context

Condensed from `docs/TDD.md`. If this file and the TDD disagree, the TDD wins; flag the difference.

## Workspace layout
```text
crates/
  kiln-text/      rope wrapper, graphemes, encoding, line endings
  kiln-core/      buffers, selections, transactions, undo tree, registers, commands, events
  kiln-syntax/    tree-sitter parsing, highlight/indent/text-object queries
  kiln-config/    TOML loading, validation, defaults, themes, keymaps
  kiln-services/  lsp, search, git, file watcher, agent/pty (async)
  kiln-input/     key parsing, mode state machine, keymaps -> commands
  kiln-ui/        terminal rendering, layout, widgets, themes
  kiln-bin/       main(), CLI args, wiring, main loop
grammars/  benches/  tests/  docs/  context/
```

## Dependency rule (strict)
Dependencies point downward only:
`kiln-bin` → `kiln-ui`, `kiln-input`, `kiln-services` → `kiln-core`, `kiln-syntax`, `kiln-config` → `kiln-text`.
- `kiln-core` and `kiln-text` have **no I/O, no terminal, no async runtime** dependencies.
- `kiln-config` does not depend on `kiln-core`.
- Never add an upward or sideways dependency to make something compile. Move the shared type down instead, or raise it in `BRAIN.md`.

## Runtime model
- **Editor thread** owns all editor state. Single-threaded, no locks around buffers.
- **Async runtime (tokio)** runs services; each service has an inbox channel and sends results back as messages.
- **Blocking pool** for large-file reads, project search, parsing large files.
- Main loop: wait for message → convert to command/update → apply → emit events → render at most once per frame (~16 ms).

## Core concepts
- **Rope** buffer (`ropey`); internal positions are character offsets; convert to line/col, byte or UTF-16 offsets only at boundaries.
- **Selections**: always a list of ranges (anchor + head), even in Vim mode.
- **Transaction**: ordered insert/delete ops plus selections before/after; invertible (undo) and mappable (marks, diagnostics).
- **Undo tree** (not a stack), persisted across sessions keyed by path + content hash.
- **Command**: named function over editor state; keymaps, palette, `:` commands and help all use one registry.
- **Event bus**: every state change emits an event (`BufferChanged`, `BufferSaved`, `ModeChanged`, `DiagnosticsUpdated`, `AgentFileChanged`, ...).

## Services
| Service | Notes |
|---|---|
| LSP manager | JSON-RPC over stdio, incremental sync, request timeouts and cancellation, drop stale responses, restart with exponential backoff |
| Search | `ignore` walker + ripgrep-style engine, streaming, cancellable |
| Git | gutter diff vs index/HEAD, cached per buffer version |
| File watcher | `notify`, debounced and batched, watches directories |
| Agent/PTY | `portable-pty` + terminal emulator crate, presets are config data |

## Agent subsystem summary
Start session → snapshot → spawn PTY in project root → watcher records changed files → open clean buffers reload, buffers with unsaved edits are marked conflicted (never overwritten) → review changed files and hunks (accept / reject / skip; reject is an undoable transaction) → rollback from snapshot on request.

## Reliability rules
Atomic saves (temp + rename), swap/recovery files, panic hook saves buffers and restores the terminal, service failures never crash the editor, resource caps on completions, search results, undo history and agent scrollback.

## Performance budgets (enforced by benchmarks)
Cold start < 30 ms · idle memory < 50 MB · keystroke-to-screen < 16 ms · 100 MB file first screen < 1 s · binary < 40 MB.

## Open decisions (do not assume; ask)
ratatui vs custom renderer · terminal emulator crate · git library (`gix` vs `git2`) · bundled vs downloaded grammars · license · exact Vim compatibility list.
