# BRAIN.md — Kiln living memory

Append-only log of decisions, gotchas, ideas and follow-ups. Agents and humans both write here. Newest entries at the top of each section. Keep entries short: date, ticket, one or two sentences.

## Decisions
| Date | Decision | Why | Ticket |
|---|---|---|---|
| (setup) | Name: Kiln | Short, easy to type; public-use check still pending (D6) | — |
| (setup) | Language: Rust, workspace of small crates | Small static binary, strong libraries for editors | — |
| (setup) | Vim-compatible common subset, documented | Full compatibility has unbounded edge cases | — |
| (setup) | No plugin system in v1; WASM plugins in v2 | Avoid locking in a bad API; keep core light | — |
| (setup) | Agents run as external CLIs in a PTY pane; presets are config data | Agent-agnostic, no vendor lock-in | — |
| (setup) | Project config `.kiln.toml` requires a trust prompt | It can launch commands | T28 |

## Open decisions
- D1 Confirm crate layout · D2 ratatui vs custom renderer · D3 terminal emulator crate · D4 Vim compatibility list · D5 license · D6 name clearance. Details in `context/progress-tracker.md`.
- Git library: `gix` vs `git2` (decide in T20).
- Grammars: bundled only, or downloaded on demand (and if downloaded, how to verify them).
- Which agent protocol for v2 (ACP, MCP or both).

## Gotchas and lessons
(Add entries like: `2026-11-02 · T12 · tree-sitter query for X crashes on empty buffer; guard added in highlight()`.)

## Ideas (not in scope yet)
(Unrelated ideas found while working go here, not into the current ticket.)
- Helix-style selection-first keymap option (post-1.0).
- Remote editing over SSH (post-1.0).
- Agent edit highlighting while the agent is typing (PRD AG-11, P2).

## Follow-ups
(Things deliberately postponed, each with the ticket that should pick them up.)

## Session log
(One line per working session: date · agent · ticket · result.)
