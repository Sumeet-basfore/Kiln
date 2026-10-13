# Kiln — Progress Tracker

Update this file at the end of every ticket. Status values: `todo`, `in-progress`, `blocked`, `done`.

## Current state
- Active milestone: 0.1 Foundation
- Last updated: 2026-10-13
- Open decisions blocking work: see "Decisions" below

## Decisions needed before coding (from TDD section 18)
| # | Decision | Status | Outcome |
|---|---|---|---|
| D1 | Confirm Rust crate layout | done | 2026-10-13: Workspace layout verified with 8 crates |
| D2 | Rendering: ratatui vs custom renderer (prototype and measure) | todo | |
| D3 | Terminal emulator crate for agent panes (spike) | todo | |
| D4 | Vim compatibility list for 0.1 and 0.2 | todo | |
| D5 | License | todo | |
| D6 | Confirm the name "Kiln" is clear for public use | todo | |

## Tickets
| ID | Milestone | Title | Status | Date | Notes |
|---|---|---|---|---|---|
| T01 | 0.1 | Workspace, CI, lint, benchmark skeleton | done | 2026-10-13 | 8 crates, downward deps check, CI, bench skeleton |
| T02 | 0.1 | kiln-text: rope wrapper, graphemes, line endings, encoding | todo | | |
| T03 | 0.1 | kiln-core: buffer, selection, transaction, undo tree | todo | | |
| T04 | 0.1 | kiln-core: command registry and basic edit commands | todo | | |
| T05 | 0.1 | kiln-input: key parsing, mode state machine, keymaps | todo | | |
| T06 | 0.1 | Vim motions and operators, first batch | todo | | |
| T07 | 0.1 | kiln-ui: buffer view, status line, cursor, dark theme | todo | | |
| T08 | 0.1 | File open, atomic save, headless test harness | todo | | |
| T09 | 0.1 | kiln-config: TOML config with defaults and validation | todo | | |
| T10 | 0.2 | Splits, tabs, multiple buffers | todo | | |
| T11 | 0.2 | Registers, clipboard, search/replace, macros, dot repeat | todo | | |
| T12 | 0.2 | kiln-syntax: tree-sitter highlighting, incremental reparse | todo | | |
| T13 | 0.2 | Fuzzy file finder and project search | todo | | |
| T14 | 0.2 | Swap files, crash recovery, panic hook | todo | | |
| T15 | 0.2 | Persistent undo | todo | | |
| T16 | 0.3 | LSP transport and manager | todo | | |
| T17 | 0.3 | Diagnostics, hover, go to definition | todo | | |
| T18 | 0.3 | Completion, rename, references, code actions | todo | | |
| T19 | 0.3 | Language table and server auto-detection | todo | | |
| T20 | 0.3 | Git service: gutter signs, branch display | todo | | |
| T21 | 0.3 | File watcher and external change handling | todo | | |
| T22 | 0.4 | PTY manager, terminal emulator, terminal pane | todo | | |
| T23 | 0.4 | Agent presets in config; launcher UI | todo | | |
| T24 | 0.4 | Change tracker, live file sync, conflict prompts | todo | | |
| T25 | 0.4 | Snapshot store and rollback | todo | | |
| T26 | 0.4 | Diff review with hunk accept/reject | todo | | |
| T27 | 0.4 | Send context to agent; prompt box; activity indicator | todo | | |
| T28 | 0.4 | Project trust prompt for .kiln.toml | todo | | |
| T29 | 0.5 | Command palette, help pages, which-key hints | todo | | |
| T30 | 0.5 | Theme loader, light theme, color fallback | todo | | |
| T31 | 0.5 | Mouse support, large-file and long-line handling | todo | | |
| T32 | 0.5 | Error-handling audit, fuzzing, performance tuning | todo | | |
| T33 | 0.5 | Packaging for Linux and macOS | todo | | |

## Notes
(Short dated notes about blockers, scope changes or surprises.)
