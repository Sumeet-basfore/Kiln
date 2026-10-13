# Kiln — Feature Tickets

Each ticket is one agent session. Rules: `context/ai-workflow-rules.md`. Format per ticket: **Crates**, **Depends on**, **Goal**, **Acceptance**, **Tests**. Every ticket also requires: fmt + clippy clean, tests green, progress tracker updated, `BRAIN.md` notes, security checklist (SECURITY.md section 14) when relevant.

Pre-work decisions D1–D6 are listed in `context/progress-tracker.md`. T07 depends on D2 (renderer) and T22 on D3 (terminal emulator).

---
## Milestone 0.1 — Foundation

### T01 Workspace, CI, lint, benchmark skeleton
- **Crates:** workspace root, all crate stubs
- **Depends on:** D1
- **Goal:** Compiling workspace matching the layout in `architecture-context.md`, with CI and tooling.
- **Acceptance:** All eight crates exist with correct dependency directions (checked by a script or `cargo deny` bans); `rust-toolchain.toml`; `rustfmt.toml`; clippy with `-D warnings`; CI (Linux, macOS, Windows) runs fmt, clippy, test; criterion bench skeleton in `benches/`; `cargo audit`/`deny` step; `.gitignore`, README stub, license placeholder.
- **Tests:** Dependency-direction check fails if an upward dependency is added; CI green on an empty test per crate.

### T02 kiln-text: rope wrapper, graphemes, line endings, encoding
- **Crates:** kiln-text
- **Depends on:** T01
- **Goal:** Text primitives used by the core.
- **Acceptance:** Rope wrapper over `ropey` with char-offset API; conversions char↔line/col↔byte↔UTF-16; grapheme cluster iteration and display width (wide and combining characters); line-ending detection (LF, CRLF, CR) and preservation on save; encoding detection (UTF-8 primary, UTF-16 and Latin-1 read support); binary detection (null bytes in first block).
- **Tests:** Unit tests for each conversion with ASCII, emoji, CJK, combining marks, CRLF; property tests that conversions round-trip; benchmark for inserting into a 100 MB rope.

### T03 kiln-core: buffer, selection, transaction, undo tree
- **Crates:** kiln-core
- **Depends on:** T02
- **Goal:** Editing model with invertible transactions and an undo tree.
- **Acceptance:** `Buffer` (rope, path, encoding, line ending, dirty flag, version); `Selection` list (anchor/head); `Transaction` (ordered insert/delete ops with selections before/after), invertible and mappable over later edits; undo tree with branches; marks mapped through transactions; no I/O or async dependencies.
- **Tests:** Property tests: apply then invert returns the original text; undo/redo sequences preserve text; mapped positions stay valid. Unit tests for branching undo.

### T04 kiln-core: command registry and basic edit commands
- **Crates:** kiln-core
- **Depends on:** T03
- **Goal:** Central command system and event bus.
- **Acceptance:** Command registry (name → function, docs, argument type); event bus with typed events (`BufferChanged`, `BufferSaved`, `ModeChanged`); basic commands: insert text, delete, move cursor, newline, backspace, undo, redo; commands return results, never panic on invalid input.
- **Tests:** Command execution unit tests; events emitted exactly once per change; fuzz-style test feeding random command sequences without panics.

### T05 kiln-input: key parsing, mode state machine, keymaps
- **Crates:** kiln-input
- **Depends on:** T04
- **Goal:** Turn key events into commands per mode.
- **Acceptance:** Key event type independent of the terminal library; mode enum (Normal, Insert, Visual variants, Command, Search); keymap trie per mode with partial-match timeout; Vim grammar state machine building `Action { register, count, operator, motion }`; default keymaps loaded from data, not hard-coded logic.
- **Tests:** Sequence tests (`3dw`, `"ayy`, `d2j`, `ciw`); timeout handling; invalid sequences reset state cleanly.

### T06 Vim motions and operators, first batch
- **Crates:** kiln-core, kiln-input
- **Depends on:** T05, D4
- **Goal:** Core editing vocabulary.
- **Acceptance:** Motions `h j k l w b e W B E 0 ^ $ gg G f F t T ; ,` and `%`; operators `d c y` with motions and counts; commands `x p P u Ctrl-r o O a A i I J`; text objects `iw aw i" a" i( a( i{ a{`; operations respect grapheme clusters and wide characters.
- **Tests:** Table-driven tests (before, keys, after) for every motion/operator/text object including multi-line, empty line, end-of-buffer and Unicode cases.

### T07 kiln-ui: buffer view, status line, cursor, dark theme
- **Crates:** kiln-ui, kiln-bin
- **Depends on:** T06, D2
- **Goal:** First usable screen following `UI-UX-SPEC.md` sections 2, 3 (basic), 4, 10.
- **Acceptance:** Diff-based rendering to the terminal (cell grid, one render per frame); buffer view with scrolling, cursor, selection; status line with mode, file, modified flag, position; one dark theme; resize and focus events handled; terminal state always restored on exit and panic; drawing behind a small trait so the renderer can be swapped.
- **Tests:** Snapshot tests of rendered frames for fixed states; resize test; frame-time benchmark at 200x60.

### T08 File open, atomic save, headless test harness
- **Crates:** kiln-bin, kiln-core, kiln-services (file I/O)
- **Depends on:** T07
- **Goal:** Open and save real files safely; enable scripted tests.
- **Acceptance:** Open from CLI path; read in a background task; binary files open read-only with a warning; atomic save (temp file in same directory, flush, rename, preserve permissions); `:w`, `:q`, `:wq`, `:q!`; unsaved-changes protection; headless mode driving the same core and input path with scripted keys and no terminal.
- **Tests:** Save-during-kill test leaves the original intact; permission preservation; headless integration tests editing fixture files; symlink handling per SECURITY.md section 5.

### T09 kiln-config: TOML config with defaults and validation
- **Crates:** kiln-config, kiln-bin
- **Depends on:** T07
- **Goal:** Typed config that never blocks startup.
- **Acceptance:** `~/.config/kiln/config.toml` parsed with serde into structs with defaults for every field; empty file valid; unknown keys and wrong types give readable errors with file and line; invalid sections fall back to defaults with a warning; keybinding overrides; theme file loading; JSON Schema generated from the structs.
- **Tests:** Valid, empty, partially invalid and fully invalid configs; error message snapshot tests; fuzz target for the loader.

---
## Milestone 0.2 — Daily-usable editing

### T10 Splits, tabs, multiple buffers
- **Crates:** kiln-core, kiln-ui, kiln-input
- **Depends on:** T09
- **Goal:** Window tree and buffer list.
- **Acceptance:** Horizontal/vertical splits, window navigation (`Ctrl-w h j k l`), close/only, tabs with tabline (UI spec section 2), buffer list commands (`:ls`, `:b`, `:bn`, `:bp`), modified marker; layout recomputed on resize.
- **Tests:** Layout unit tests for nested splits and tiny terminals; snapshot tests; integration tests closing windows with modified buffers.

### T11 Registers, clipboard, search/replace, macros, dot repeat
- **Crates:** kiln-core, kiln-input
- **Depends on:** T10
- **Goal:** Complete daily Vim workflow.
- **Acceptance:** Named, numbered and unnamed registers; system clipboard registers `+` and `*` (graceful failure when unavailable); `/ ? n N * #`; `:s` and `:%s` with regex and flags; macros `q` and `@`; dot repeat; marks and jump list basics.
- **Tests:** Table-driven tests per feature; regex edge cases (empty match, multiline, Unicode); macro replay equals typed result.

### T12 kiln-syntax: tree-sitter highlighting, incremental reparse
- **Crates:** kiln-syntax, kiln-ui
- **Depends on:** T10
- **Goal:** Syntax highlighting without blocking typing.
- **Acceptance:** Bundled grammars for a core set (Rust, TOML, Markdown, JSON, YAML, Python, JavaScript/TypeScript, Go, C/C++, Bash, HTML/CSS); language detection by extension and shebang; parse off the editor thread; incremental reparse from transactions; last good highlights shown while reparsing; visible-range-only highlighting for large files.
- **Tests:** Highlight output snapshots per language; incremental parse equals full parse after random edits; benchmark on a large file.

### T13 Fuzzy file finder and project search
- **Crates:** kiln-services, kiln-ui, kiln-input
- **Depends on:** T10
- **Goal:** Fast project navigation (UI spec section 6).
- **Acceptance:** Background file walker respecting `.gitignore`; fuzzy matcher with streaming results; project-wide regex search (ripgrep-style) streaming in batches, cancellable; overlays with preview; buffer switcher; recent files; result and memory caps.
- **Tests:** Finder correctness and ranking tests; cancellation tests; benchmark: first results under 200 ms on a 100k-file tree.

### T14 Swap files, crash recovery, panic hook
- **Crates:** kiln-core, kiln-services, kiln-bin
- **Depends on:** T08
- **Goal:** Never lose work.
- **Acceptance:** Periodic swap/journal files for unsaved changes with owner-only permissions; recovery prompt on next open when a swap is newer than the file; panic hook saves buffers to recovery files and restores the terminal; SIGTERM/SIGHUP handled; stale swap cleanup.
- **Tests:** Kill-during-edit and kill-during-save recovery tests; panic injection test; permission test.

### T15 Persistent undo
- **Crates:** kiln-core, kiln-services
- **Depends on:** T14
- **Goal:** Undo history survives sessions.
- **Acceptance:** Undo tree serialized under the Kiln state directory keyed by path and content hash; discarded if the file changed externally; size cap and pruning; corrupted undo file ignored safely.
- **Tests:** Round-trip test; external-change invalidation; corrupted-file test; fuzz the deserializer.

---
## Milestone 0.3 — Language intelligence

### T16 LSP transport and manager
- **Crates:** kiln-services
- **Depends on:** T12
- **Goal:** Robust LSP plumbing.
- **Acceptance:** JSON-RPC over stdio with request tracking, timeouts and cancellation; server lifecycle per workspace and language; initialize/shutdown; incremental document sync driven by `BufferChanged` with debounce; stale responses dropped; restart with exponential backoff and visible warning; crashes never crash the editor; output size limits. Follow SECURITY.md sections 4 and 7.
- **Tests:** Mock LSP server tests (slow, crashing, malformed, huge output); backoff test; cancellation test.

### T17 Diagnostics, hover, go to definition
- **Crates:** kiln-services, kiln-core, kiln-ui, kiln-input
- **Depends on:** T16
- **Goal:** First visible LSP features.
- **Acceptance:** Diagnostics stored per buffer and mapped through edits; gutter signs, underlines, status counts, `[d`/`]d`; hover popup (`K`); go to definition (`gd`) across files with jump list; all server-provided text sanitized (SECURITY.md section 6).
- **Tests:** Diagnostic mapping after edits; popup snapshots; mock-server integration tests; sanitization tests with control characters.

### T18 Completion, rename, references, code actions
- **Crates:** kiln-services, kiln-core, kiln-ui, kiln-input
- **Depends on:** T17
- **Goal:** Editing-assistance features.
- **Acceptance:** Completion menu with capped item count; rename showing affected files before applying; references list; code actions menu; format on save (configurable); workspace edits applied as undoable transactions and rejected if outside the workspace without confirmation.
- **Tests:** Mock-server tests for each feature; edit-application tests (multi-file, overlapping ranges); out-of-workspace edit rejection test.

### T19 Language table and server auto-detection
- **Crates:** kiln-services, kiln-config
- **Depends on:** T16
- **Goal:** Zero-config language servers.
- **Acceptance:** Built-in table (file patterns, root markers, server command) for rust-analyzer, pyright, typescript-language-server, gopls, clangd, lua-language-server, bash-language-server, taplo, marksman, and similar; overridable in config; missing server shows an actionable message and never downloads anything; project overrides require trust.
- **Tests:** Detection tests for sample project trees; override precedence tests; untrusted-config test.

### T20 Git service: gutter signs, branch display
- **Crates:** kiln-services, kiln-ui
- **Depends on:** T12, D (git library choice)
- **Goal:** Git awareness without blocking.
- **Acceptance:** Gutter diff vs index/HEAD computed in the background and cached per buffer version; branch name in status line; hunk navigation `[g`/`]g`; stage/unstage hunk; works outside repositories; large-repo safe. Compare `gix` and `git2` first and record the decision in `BRAIN.md`.
- **Tests:** Fixture repos (clean, modified, untracked, detached HEAD, no repo); gutter correctness after edits; benchmark on a large repo.

### T21 File watcher and external change handling
- **Crates:** kiln-services, kiln-core
- **Depends on:** T14
- **Goal:** Detect disk changes safely.
- **Acceptance:** Directory watching with debounce and batching; handles atomic rename-writes; clean buffers reload with cursor preserved (mapped through a diff); buffers with unsaved edits are marked conflicted and the conflict prompt is shown; deleted files handled; file tree and git signs refreshed.
- **Tests:** Scripted external writers (in-place, temp+rename, delete, rapid bursts); conflict-prompt tests; no silent overwrite test.

---
## Milestone 0.4 — Agent integration

### T22 PTY manager, terminal emulator, terminal pane
- **Crates:** kiln-services, kiln-ui, kiln-input
- **Depends on:** T21, D3
- **Goal:** Run any command in a pane.
- **Acceptance:** Spawn programs with an explicit argument list (no shell string building) in the project root; terminal emulator renders colors, cursor movement and resize correctly; focus switching (`Ctrl-\ Ctrl-n` to leave); scrollback cap; exit status display; escape-sequence restrictions per SECURITY.md section 6 (no clipboard write, no title abuse).
- **Tests:** PTY tests with a real shell and recorded escape-sequence samples; resize test; malicious escape-sequence test; tests with a real agent TUI if available.

### T23 Agent presets in config; launcher UI
- **Crates:** kiln-config, kiln-services, kiln-ui
- **Depends on:** T22
- **Goal:** One-keypress agent start.
- **Acceptance:** Presets as config data (name, command, args, env) for Claude Code, Codex, Antigravity CLI, OpenCode, Cursor CLI and a custom command (verify each tool's current CLI before shipping); launcher overlay (`Space a o`); multiple sessions with names and `Space a n`; status in status line; presets from untrusted project config are disabled.
- **Tests:** Preset parsing tests; launcher snapshot tests; fake-agent session lifecycle tests.

### T24 Change tracker, live file sync, conflict prompts
- **Crates:** kiln-services, kiln-core, kiln-ui
- **Depends on:** T21, T22
- **Goal:** See what an agent changed, safely.
- **Acceptance:** Per-session change set (modified, created, deleted); changed-files panel (UI spec 7.3); clean buffers reload; buffers with unsaved edits get the conflict prompt (UI spec 7.5); gutter marks for agent-edited lines.
- **Tests:** Fake agent writing files on a schedule; conflict scenarios; large burst of changes.

### T25 Snapshot store and rollback
- **Crates:** kiln-services
- **Depends on:** T24
- **Goal:** Independent rollback of agent changes.
- **Acceptance:** Session start records content hashes of tracked files; original content stored the first time a change is observed; rollback of one file or all files with confirmation; works without git; excludes ignored paths and very large files; owner-only permissions; cleanup after session or retention.
- **Tests:** Rollback correctness incl. created and deleted files; fast-write timing test (changes observed late); no git repo; permission test.

### T26 Diff review with hunk accept/reject
- **Crates:** kiln-ui, kiln-core, kiln-input
- **Depends on:** T25
- **Goal:** Human review loop (UI spec 7.4).
- **Acceptance:** Line diff plus word-level diff; inline and side-by-side views; keys `]h [h ]f [f a x s A X b q`; reject applies baseline text as an undoable transaction; baseline switch between snapshot and git HEAD; progress footer; rejected hunks in buffers with unsaved edits go through the conflict prompt.
- **Tests:** Diff correctness tests; accept/reject round trips; undo after reject; snapshot tests of both views.

### T27 Send context to agent; prompt box; activity indicator
- **Crates:** kiln-services, kiln-ui, kiln-input
- **Depends on:** T23
- **Goal:** Fast editor → agent handoff.
- **Acceptance:** Send selection or line with `path:start-end` reference (`Space a s`); prompt box with instruction (`Space a p`); send diagnostics (`Space a d`); delivery via bracketed paste; per-preset format setting; status line shows running/idle/waiting; nothing is sent without an explicit key press.
- **Tests:** Fake agent receiving exact bytes (including multi-line and special characters); format-per-preset tests; no background sending test.

### T28 Project trust prompt for .kiln.toml
- **Crates:** kiln-config, kiln-ui, kiln-bin
- **Depends on:** T09, T23
- **Goal:** Safe project config (SECURITY.md section 3).
- **Acceptance:** Detect `.kiln.toml`; prompt listing exactly which commands it enables; options Trust / Trust once / Don't trust (default); trust keyed by canonical path plus content hash and revoked on change; untrusted projects use defaults; trust store with owner-only permissions.
- **Tests:** Trust, revoke-on-edit, once, deny; malicious `.kiln.toml` fixtures; symlinked project paths.

---
## Milestone 0.5 — Polish and hardening

### T29 Command palette, help pages, which-key hints
- **Crates:** kiln-ui, kiln-input, kiln-core
- **Depends on:** T13
- **Goal:** Discoverability.
- **Acceptance:** Palette lists all commands with current keybindings; `:help` topics and `:tutor`; which-key popup after prefix keys with configurable delay; first-run welcome screen; `:messages` history.
- **Tests:** Palette completeness test (every registered command appears and has docs); snapshot tests; which-key timing tests.

### T30 Theme loader, light theme, color fallback
- **Crates:** kiln-config, kiln-ui
- **Depends on:** T09
- **Goal:** Themes and robust colors (UI spec section 10).
- **Acceptance:** Theme TOML with required scopes; validation with clear errors; dark and light themes shipped; color-depth detection with automatic downgrade; `NO_COLOR` respected; meaning never conveyed by color alone.
- **Tests:** Theme validation tests; downgrade tests (true color → 256 → 16); contrast check for shipped themes.

### T31 Mouse support, large-file and long-line handling
- **Crates:** kiln-ui, kiln-core, kiln-text
- **Depends on:** T10
- **Goal:** Robustness on big or odd input.
- **Acceptance:** Optional mouse (click, drag, scroll, tabs, tree); 100 MB file shows first screen in under 1 s with no UI freeze; read-mostly mode and visible-range-only highlighting for very large files; long lines soft-wrapped or truncated with an indicator; binary files read-only.
- **Tests:** Benchmarks with 100 MB and long-line fixtures; no-freeze test (frame time stays within budget during load); mouse event tests.

### T32 Error-handling audit, fuzzing, performance tuning
- **Crates:** all
- **Depends on:** T29, T30, T31
- **Goal:** Meet budgets and remove sharp edges.
- **Acceptance:** No `unwrap`/`expect` in non-test code (or each justified); fuzz targets for input parser, config loader, buffer ops, undo deserializer, escape-sequence handler run in CI; all PRD performance budgets measured and met or documented gaps; memory caps verified; `cargo audit` clean.
- **Tests:** CI benchmarks fail on regression; fuzz smoke runs; crash-recovery regression suite.

### T33 Packaging for Linux and macOS
- **Crates:** release tooling
- **Depends on:** T32
- **Goal:** Installable releases.
- **Acceptance:** CI release builds for Linux (x86_64, aarch64) and macOS (Intel, Apple Silicon), stripped, with checksums; install script; Homebrew formula draft; man page and shell completions; version command; docs for install and uninstall; binary size tracked against the 40 MB budget.
- **Tests:** Install-script test in a clean container; binary size gate in CI.
