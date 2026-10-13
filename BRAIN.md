# BRAIN.md — Kiln living memory

Append-only log of decisions, gotchas, ideas and follow-ups. Agents and humans both write here. Newest entries at the top of each section. Keep entries short: date, ticket, one or two sentences.

## Decisions
| Date | Decision | Why | Ticket |
|---|---|---|---|
| 2026-10-13 | ChangeSet normalized ops representation for transactions | Retain/Delete/Insert ops merged and non-zero; inverses calculated from start text; skipping history prohibited to protect inverse integrity | T03 |
| 2026-10-13 | Undo group lifecycle: start snapshot at depth 0->1, compose at 0 | Nested groups supported cleanly; dirty state tracks saved undo node ID; jump/undo/save while group open strictly rejected | T03 |
| 2026-10-13 | Range mapping rules: empty range assoc after; non-empty start after / end before | Preserves Vim point cursor invariants and keeps insertions outside selected ranges | T03 |
| 2026-10-13 | Buffer explicitly excludes syntax tree | Preserves strict crate dependency hierarchy (kiln-syntax keys syntax trees by (BufferId, version)) | T03 |
| 2026-10-13 | Normalized `\n` in Text rope; denormalize on save | Clean char offsets, column math, and tree-sitter sync without CRLF double-offset complications | T02 |
| 2026-10-13 | External crates in kiln-text: ropey, unicode-segmentation, unicode-width, thiserror | Drop encoding_rs in favor of manual standards-compliant UTF-8/UTF-16/Latin-1 decoding | T02 |
| 2026-10-13 | `kiln-benches` as tooling package | `benches/` is developer tooling, not a product crate; ignored by check-deps.sh | T01 |
| 2026-10-13 | scripts/check-deps.sh for dependency-direction check | Fast, zero compile overhead in CI and local checks, strictly enforces downward rules | T01 |
| 2026-10-13 | D1 Crate layout confirmed: 8 crates + benches | Verified according to architecture-context.md; downward dependencies only | T01 |
| (setup) | Name: Kiln | Short, easy to type; public-use check still pending (D6) | — |
| (setup) | Language: Rust, workspace of small crates | Small static binary, strong libraries for editors | — |
| (setup) | Vim-compatible common subset, documented | Full compatibility has unbounded edge cases | — |
| (setup) | No plugin system in v1; WASM plugins in v2 | Avoid locking in a bad API; keep core light | — |
| (setup) | Agents run as external CLIs in a PTY pane; presets are config data | Agent-agnostic, no vendor lock-in | — |
| (setup) | Project config `.kiln.toml` requires a trust prompt | It can launch commands | T28 |

## Open decisions
- D2 ratatui vs custom renderer · D3 terminal emulator crate · D4 Vim compatibility list · D5 license · D6 name clearance. Details in `context/progress-tracker.md`.
- Git library: `gix` vs `git2` (decide in T20).
- Grammars: bundled only, or downloaded on demand (and if downloaded, how to verify them).
- Which agent protocol for v2 (ACP, MCP or both).

## Gotchas and lessons
- 2026-10-13 · T03 · Criterion benchmark on 100 MB buffer: `apply_1char_insert` scales with $O(\log N)$ across file sizes (1 MB: 1.20 µs, 10 MB: 1.44 µs, 100 MB: 1.63 µs); `apply_100char_replace_100mb`: 1.73 µs; `apply_100_scattered_edits_100mb`: 160.03 µs; `undo_1char_insert_100mb`: 2.39 µs; `invert_only_100char_replace_100mb`: 160.51 ns; `map_1000_marks`: 1.08 µs.
- 2026-10-13 · T03 · Benchmarks must use `black_box` and the budgets come from the PRD, never invented. An earlier apply benchmark showed ~179 ms because `ChangeSet::apply` rebuilt an entire new `Text` rope by slicing and re-inserting all retained characters, which was $O(N)$ with ~100 MB allocations. Making `apply` incremental via `text.clone()` and in-place `remove`/`insert` on mutations dropped single-edit application from 179 ms down to 1.63 µs (~110,000x speedup).
- 2026-10-13 · T02 · Criterion benchmark numbers on 100 MB rope (~104.8M chars, ~1.12M lines): `read_slice_middle` (1000 chars): ~311 ns; `char_to_line_col`: ~166 ns; `insert_remove_middle` (100 chars): ~451 ns.
- 2026-10-13 · T02 · In deny.toml `[bans]`, `allow-workspace = true` and `wildcards = "allow"` are required when workspace crates reference path dependencies without version numbers.
- 2026-10-13 · T01 · In Cargo virtual workspaces, `benches/` needs its own package manifest or membership in `[workspace.members]` to run `cargo bench`.
- 2026-10-13 · T01 · Setting lint groups alongside individual lints in `[workspace.lints.rust]` requires explicit priorities (e.g. `priority = -1`) to satisfy `clippy::lint_groups_priority`.

## Ideas (not in scope yet)
(Unrelated ideas found while working go here, not into the current ticket.)
- Helix-style selection-first keymap option (post-1.0).
- Remote editing over SSH (post-1.0).
- Agent edit highlighting while the agent is typing (PRD AG-11, P2).

## Follow-ups
(Things deliberately postponed, each with the ticket that should pick them up.)
- History tree size cap and pruning: Implement history node limit and pruning policy in T15 (persistent undo).

## Session log
- 2026-10-13 · Antigravity · T03 · kiln-core editing model: Buffer, Range, Selections, ChangeSet, Transaction, UndoTree, Marks, comprehensive unit tests, proptests, and 100 MB buffer benchmark.
- 2026-10-13 · Antigravity · T02 · kiln-text primitives, rope wrapper, graphemes, line endings, encoding, binary detection, proptests, and 100 MB criterion benchmark.
- 2026-10-13 · Antigravity · T01 · Workspace skeleton, CI workflow, clippy/fmt clean, check-deps.sh passing, benchmark skeleton compiled.

