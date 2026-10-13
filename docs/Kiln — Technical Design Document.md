# Kiln — Technical Design Document

**Author:** Sumeet\
**Status:** Draft v0.1\
**Last updated:** October 13, 2026\
**Companion document:** Kiln Product Requirements Document (PRD)\
**One-line summary:** Kiln is a Rust workspace of small crates around a UI-free editing core, with asynchronous services (LSP, search, git, file watching, agent terminals) that talk to the core through typed messages.

---

## 1. Purpose and Scope

This document describes how Kiln will be built to meet the requirements in the PRD. It covers architecture, data structures, concurrency, rendering, input handling, language intelligence, the AI agent subsystem, configuration, reliability, testing, and a task breakdown per milestone.

Crate names and library choices are proposals. Each one should be confirmed with a small prototype before it is locked in, especially the terminal UI approach (section 8) and the embedded terminal emulator (section 11).

## 2. Design Principles

1. **The core knows nothing about the terminal.** Buffers, selections, modes and commands are plain Rust with no I/O, so they are easy to test and could be reused later.
2. **One owner of state.** A single editor thread owns all editor state. Services never touch it directly; they send messages.
3. **Never block the editor thread.** Disk, network, process and parsing work happens elsewhere.
4. **Everything is a command.** Every action (keypress, menu, palette, agent request) resolves to a named command, so keybindings, the command palette and the future plugin API share one path.
5. **Events out, commands in.** State changes are published as events. This is the foundation of the v2 plugin API.
6. **Fail soft.** A crashing language server, a bad config or an unreadable file must never take the editor down.

## 3. Workspace and Crate Layout

Kiln is a Cargo workspace. Dependencies point downward only: crates lower in the list never depend on crates higher up.

```text
kiln/
  Cargo.toml              # workspace
  crates/
    kiln-bin/             # main(), CLI args, wiring
    kiln-ui/              # terminal rendering, layout, widgets, themes
    kiln-input/           # key parsing, keymaps, mode handling
    kiln-services/        # lsp, search, git, watcher, agent/pty
    kiln-config/          # TOML loading, validation, defaults, themes
    kiln-syntax/          # tree-sitter integration, highlight queries
    kiln-core/            # buffers, selections, commands, undo, registers
    kiln-text/            # rope wrapper, encoding, line endings, graphemes
  grammars/               # bundled tree-sitter grammars and queries
  docs/                   # PRD, TDD, architecture notes, ADRs
  benches/                # performance benchmarks
  tests/                  # integration and end-to-end tests
```

| Crate | Responsibility | Depends on |
| --- | --- | --- |
| kiln-text | Rope wrapper, grapheme and width handling, encoding and line-ending detection | external text crates only |
| kiln-core | Editor state, buffers, selections, undo tree, registers, commands, events | kiln-text |
| kiln-syntax | Parse trees, incremental reparse, highlight and text-object queries | kiln-text |
| kiln-config | Parse and validate config, keymaps and themes into typed structs | none of the above |
| kiln-services | Async services: LSP, search, git, file watcher, agent terminals | kiln-core types, kiln-syntax |
| kiln-input | Terminal key events to commands, using keymaps and modes | kiln-core, kiln-config |
| kiln-ui | Draw editor state to the terminal | kiln-core, kiln-config |
| kiln-bin | Starts everything and runs the main loop | all |

## 4. Runtime Architecture

### 4.1 Threads and Tasks

- **Editor thread (main):** owns `Editor` state, handles input, applies commands, and triggers rendering. It is single-threaded by design, so there are no locks around buffers.
- **Async runtime (tokio):** runs all services. Each service is a task with an inbox (channel) and sends results back to the editor thread.
- **Blocking pool:** used for heavy synchronous work such as large-file reading, project search and tree-sitter parsing of large files.

### 4.2 Message Flow

The editor thread runs one loop:

1. Wait for the next message from any source: terminal input, a service result, a timer, or a signal.
2. Convert it to a command or an internal update.
3. Apply it to editor state. Emit events describing what changed.
4. If anything visible changed, mark the UI dirty and render at most once per frame (about 16 ms).

Message types (simplified):

| Source | Example messages |
| --- | --- |
| Terminal | Key, Mouse, Resize, Paste, FocusGained |
| LSP service | Diagnostics, CompletionResult, HoverResult, ServerExited |
| Search service | SearchProgress, SearchDone |
| File watcher | FileChangedOnDisk, FileDeleted |
| Git service | GutterUpdated, BranchChanged |
| Agent service | PtyOutput, AgentExited, FilesTouched |
| Timers | Autosave swap, debounce expired, config reload |

### 4.3 Event Bus

Every state change produces an event (for example `BufferChanged`, `BufferSaved`, `ModeChanged`, `DiagnosticsUpdated`, `AgentFileChanged`). Internal subsystems subscribe to the events they care about; for example, the LSP service listens to `BufferChanged` to send incremental document updates. In v2 the same events are exposed to WASM plugins.

## 5. Text Model (kiln-text and kiln-core)

### 5.1 Buffer

- Text is stored in a **rope** (`ropey`), giving fast inserts and deletes anywhere in very large files and cheap snapshots.
- A buffer holds: the rope, file path, encoding, line ending style, a dirty flag, a version number, the undo tree, marks, and a link to its syntax tree.
- Positions are tracked as **character offsets** internally and converted to line/column and to byte or UTF-16 offsets only at boundaries (rendering, tree-sitter, LSP).
- Wide characters and combining marks are handled with grapheme clusters and display widths during rendering and cursor movement.

### 5.2 Selections

All editing uses a list of selections (ranges with an anchor and head), even in Vim mode where there is usually one. This keeps the design open for multiple cursors later and makes Helix-style editing possible without a rewrite.

### 5.3 Edits and Undo

- Every change is a **transaction**: an ordered list of insert/delete operations plus the selections before and after.
- Transactions can be inverted, which gives undo for free, and mapped over later edits, which keeps marks and diagnostics aligned.
- The undo history is a **tree**, not a stack, so no state is lost after undoing and editing again.
- **Persistent undo:** the tree is serialized to a file under the Kiln state directory, keyed by file path and content hash, and discarded if the file changed outside Kiln.

### 5.4 Large Files and Odd Content

- Files are read in a background task and shown progressively when large; very large files open in a read-mostly mode with syntax highlighting limited to the visible region.
- Very long lines are soft-wrapped or truncated in rendering with a visible indicator; they never stall layout.
- Binary files are detected (null bytes in the first block) and opened read-only with a warning.

## 6. Commands, Modes and Keymaps (kiln-input)

### 6.1 Command System

A command is a named function that takes the editor state and an argument, and returns a result. Examples: `move_word_forward`, `delete_selection`, `open_file_finder`, `agent_send_selection`. The registry of commands drives the keymap, the command palette, `:` commands and the help page.

### 6.2 Modes

Modes are an enum in core (Normal, Insert, Visual with a sub-kind, Command, Search, plus modal overlays such as finder and diff review). Each mode has a keymap trie. Key sequences are matched against the trie with a timeout for partial matches, and the which-key hint shows the possible continuations.

### 6.3 Vim Grammar

Normal mode follows the Vim grammar: optional register, optional count, operator, then motion or text object. This is implemented as a small state machine that builds an `Action { register, count, operator, motion }` and hands it to the core. Motions and text objects are pure functions from (buffer, selection) to a range, which makes them simple to unit test.

The documented compatibility list (PRD open question 1) decides which commands are implemented in each milestone.

### 6.4 Repeat and Macros

The dot command repeats the last change by replaying its recorded action. Macros record raw key events into a register and replay them through the same input path.

## 7. Syntax and Language Intelligence

### 7.1 Syntax (kiln-syntax)

- Each buffer with a known language has a tree-sitter parse tree, updated **incrementally** from transactions.
- Parsing runs off the editor thread. The editor shows the last good highlights and updates when the new tree arrives, so typing never waits on parsing.
- Highlight, indent and text-object queries are bundled per language. Languages are compiled into the binary for a core set; others can be downloaded on demand (PRD open question 3).
- Syntax-aware features reuse the tree: text objects (function, class, argument), smart indent, folding and the symbol outline.

### 7.2 LSP Client (kiln-services)

- One manager task starts, tracks and restarts language servers per workspace and language, using a built-in language table (file patterns, root markers, server command) that users can override in config.
- Communication is JSON-RPC over stdio. Request and response types come from `lsp-types`; the transport and request tracking are Kiln's own small async code.
- Document sync is **incremental**, driven by `BufferChanged` events, with a short debounce.
- Features by milestone: diagnostics, hover and go to definition first; then completion, rename, references, code actions, signature help and inlay hints.
- Every request has a timeout and can be cancelled when the cursor moves on. Stale responses (older document version) are dropped.
- A server that exits is restarted with exponential backoff and a visible warning; after repeated failures it stays off until the user restarts it.
- Diagnostics are stored per buffer, mapped through edits so they stay attached to the right text until the server sends new ones.

## 8. Rendering (kiln-ui)

### 8.1 Approach

The UI is an immediate-mode renderer: each frame, the UI reads editor state and draws into an in-memory cell grid, then **diffs against the previous frame** and writes only the changed cells to the terminal. Two options are on the table:

- **ratatui + crossterm:** faster to start, well maintained, good widgets. Possible overhead on very large screens and awkward for fully custom layouts.
- **Custom renderer on crossterm (or direct escape sequences):** full control over performance and memory, but more code to write and test.

**Decision process:** build a small prototype of the buffer view on both, measure frame time with a 200x60 terminal and a large file, and pick. Start with ratatui and keep drawing code behind a small trait so it can be swapped.

### 8.2 Layout

A window tree (splits, tabs) describes how buffers, the file tree, the terminal/agent panes and overlays are arranged. Layout is computed from terminal size on resize and on window commands. Overlays (finder, palette, diff review, completion menu, hover popup) draw above the layout.

### 8.3 Text Drawing

For each visible line, the renderer combines text, syntax highlights, diagnostics underlines, selection, git signs and cursor into styled cells. Tabs, wide characters and soft-wrap are applied here. Only the visible range of a buffer is ever processed.

### 8.4 Themes and Colors

Themes are TOML files mapping scopes (for example `keyword`, `string`, `diagnostic.error`, `ui.statusline`) to colors and modifiers. At startup the terminal's color support is detected (true color, 256, 16) and colors are downgraded automatically. A semantic palette keeps themes short.

## 9. Project Tools

- **Fuzzy finder:** file list built by a background walker that respects `.gitignore` (the `ignore` crate); matching uses a fast fuzzy matcher (`nucleo` is the candidate). Results stream into the picker as they arrive.
- **Project search:** ripgrep-style engine (`grep-searcher`, `grep-regex`) running in the blocking pool, streaming matches in batches with cancellation when the query changes.
- **File tree:** lazily loaded directory model, updated by the file watcher.
- **Git:** gutter signs from a diff of the buffer against the index/HEAD version, computed in the background and cached per buffer version. Candidate libraries: `gix` (pure Rust) or `git2` (libgit2 bindings). Prefer the one with the smaller binary and fewer system dependencies after a quick comparison.
- **Sessions:** open buffers, layout and cursor positions are saved per project on exit and restored on start.

## 10. Configuration (kiln-config)

- Files: user config `~/.config/kiln/config.toml`, optional project config `.kiln.toml`, theme files, and a keymap section.
- Parsed with `serde` and `toml` into typed structs with defaults for every field, so an empty file is valid.
- **Validation:** unknown keys and wrong types produce readable messages with file and line. On error, Kiln starts with defaults for the bad section and shows a warning; it never refuses to start.
- **Project trust:** a `.kiln.toml` can define commands (agent presets, language servers), so Kiln asks for explicit trust once per project (PRD open question 6) and stores the decision.
- **Hot reload:** a watcher on the config files triggers a reload; the result is applied atomically or rejected as a whole.
- A JSON Schema for the config is generated from the structs for editor support.

## 11. AI Agent Subsystem

This subsystem delivers the PRD's main differentiator (PRD section 7). It treats agents as ordinary programs running in a terminal pane, and builds the supervision features around them.

### 11.1 Components

| Component | Job |
| --- | --- |
| PTY manager | Spawns the agent command in a pseudo-terminal (`portable-pty`), in the project root, with configured arguments and environment |
| Terminal emulator | Interprets the agent's output (escape sequences, colors, cursor movement) into a screen grid the UI can draw. Candidates: `alacritty_terminal` or `vt100` |
| Agent presets | Data in config describing how to start each agent (command, args, env, display name). No agent-specific code |
| Change tracker | Detects which files changed during an agent session |
| Snapshot store | Records file states before a session so changes can be reverted independently of git |
| Review UI | Shows the changed-files list and a hunk-by-hunk diff with accept and reject |
| Context sender | Writes selected text, file references or diagnostics into the agent's PTY input |

### 11.2 Agent Session Lifecycle

1. **Start:** the user picks a preset. Kiln takes a **snapshot** (see 11.4), spawns the PTY in the project root, and opens a pane.
2. **Run:** PTY output flows to the terminal emulator and the pane redraws. The file watcher reports changes in the project directory.
3. **Observe:** each changed file is recorded in the session's change set and open buffers are reloaded or marked in conflict (11.3).
4. **Review:** the user opens the changed-files panel and the diff view and accepts or rejects hunks.
5. **End:** when the process exits, the pane shows the exit status and the change set stays available for review until closed.

### 11.3 Live File Sync and Conflicts

When the watcher reports a change to a file:

- **Buffer not open:** just update the changed-files list and the file tree.
- **Buffer open and clean:** reload from disk, keep the cursor position by mapping it through a diff, and refresh git signs and diagnostics.
- **Buffer open with unsaved edits:** do not touch the buffer. Mark it as conflicted and offer: keep mine, take theirs, or view the diff. Nothing is ever overwritten silently (PRD principle 4).

File watcher events are debounced (for example 50 ms) and batched, because agents often write many files quickly. Atomic writes by other programs (write to temp, then rename) are handled by watching the directory, not only the file.

### 11.4 Snapshots and Rollback

- Before an agent session starts, Kiln records the content hash of every tracked file in the project, and stores the full content of a file the first time the watcher reports a change to it.
- Because the original content is saved just before the first modification is observed, any file can be restored to its pre-session state with one command, even in a repository with uncommitted changes or no git at all.
- Snapshots live in the Kiln state directory and are removed when the session is closed or after a retention period.
- Limits: very large files and ignored paths (build output, `node_modules`, `target`) are excluded from snapshots by default.

Timing caveat: a file changed in the short window between the agent writing and the watcher reporting could be missed by this approach. The pre-session hash list plus a git baseline (when available) lets the review UI still show an accurate diff in that case. This should be validated with tests that simulate fast agent writes.

### 11.5 Diff Review

- The diff baseline is the session snapshot, or the last git commit when the user chooses it.
- Diffs are computed with a standard line diff (Myers or histogram, via a library such as `similar`) with a word-level diff inside changed lines.
- Each hunk can be accepted (kept), rejected (reverted to the baseline text, as a normal undoable edit), or skipped. Rejection is implemented as a transaction on the buffer, so it can be undone like any edit.
- The view supports navigation by hunk and by file, and shows agent-edited lines in open buffers with a distinct gutter mark.

### 11.6 Sending Context to Agents

The context sender formats a reference such as `path/to/file.rs:42-58` plus optional text and writes it to the PTY as if typed, using bracketed paste so multi-line text is delivered safely. Because different agents expect different conventions, the format can be set per preset (for example, plain text or a path-and-range reference). A small prompt box lets the user add an instruction before sending.

### 11.7 Safety

- Agents start only when the user starts them. Kiln never launches one on its own.
- Kiln does not read agent conversations or send project content anywhere; it only moves bytes between the PTY and the screen, plus what the user explicitly sends.
- Agent file changes never overwrite unsaved user edits.
- Preset commands from a project config require the project trust step (section 10).

### 11.8 Agent Protocol Server (v2)

A local server (Unix socket on Linux and macOS, named pipe on Windows) exposes editor state and actions to agents: open files, selection, diagnostics, open file at line, and show a diff for review. Token authentication, disabled by default, and an explicit permission list per client. The protocol (ACP, MCP or both) is decided after research at the start of v2; the internal event bus and command system are designed so the server is a thin adapter.

## 12. Reliability and Data Safety

| Concern | Design |
| --- | --- |
| Atomic saves | Write to a temp file in the same directory, flush, then rename over the original. Preserve permissions. Fall back to in-place write if rename is not possible |
| Swap and recovery | Periodic swap files with unsaved changes (journal of transactions). On next start, offer recovery if a swap file is newer than the file |
| Crash handling | A panic hook writes unsaved buffers to recovery files, restores the terminal state, then exits |
| Terminal restoration | Raw mode and alternate screen are always restored on exit, panic and signals (SIGTERM, SIGHUP) |
| External changes | Detected on focus and by watcher; prompt or reload as described in 11.3 |
| Service failures | Each service runs in its own task with panic isolation; failures restart the service and show a status message |
| Resource limits | Caps on completion list size, search results held in memory, undo history size, and agent scrollback |

## 13. Performance Plan

The PRD targets (startup under 30 ms, idle memory under 50 MB, keystroke latency under 16 ms, 100 MB file open under 1 second) are treated as budgets that tests enforce.

- **Startup:** lazy-initialize everything not needed for first paint (LSP, git, grammars, file index). Avoid reading config twice. Use a release profile with LTO and `codegen-units = 1`. Using `panic = abort` would shrink the binary but prevents the panic hook from saving buffers, so verify this trade-off before choosing it.
- **Rendering:** diff-based terminal output, one render per frame, cached line layout for unchanged lines.
- **Memory:** ropes for text, shared syntax trees, streaming results instead of collecting them, bounded caches.
- **Binary size:** strip symbols, disable unused crate features, evaluate size-optimized builds for non-hot crates, load rarely used grammars on demand.
- **Measurement:** criterion benchmarks in `benches/` for buffer edits, rendering a frame, opening large files, and project search; an end-to-end startup timer; results compared in CI against a stored baseline.

## 14. Testing Strategy

| Level | What | Tools |
| --- | --- | --- |
| Unit | Motions, text objects, operators, undo tree, selections, config parsing | built-in test framework, `proptest` for property tests |
| Snapshot | Rendered frames for fixed editor states | `insta` or similar |
| Integration | Scripted key sequences against fixture files, checking final text and cursor | test harness that drives the editor without a real terminal |
| LSP | Mock server for protocol behavior; smoke tests with real servers (rust-analyzer, pyright, typescript-language-server, gopls) | mock server binary, CI jobs |
| Agent | Fake agent program that writes files on a schedule; checks sync, conflicts, snapshots and diff review | test fixtures |
| Terminal | PTY tests with a real shell and escape-sequence samples | `portable-pty` |
| Fuzz | Input parser, config loader, buffer edit sequences | `cargo-fuzz` |
| Performance | Startup, latency, large file, search | `criterion`, CI regression check |
| Crash/recovery | Kill during edit and during save, then verify recovery | scripted tests |
| Platforms | Linux, macOS, Windows | CI matrix |

The editor is built with a **headless mode** from the start: the same core and input path run without a terminal, which is what makes integration and agent tests practical.

## 15. Build, Packaging and Release

- **Toolchain:** stable Rust, pinned with `rust-toolchain.toml`. `rustfmt` and `clippy` (warnings as errors) in CI.
- **CI:** build and test on Linux, macOS and Windows; run benchmarks and fuzz smoke tests on the main branch; track binary size per build.
- **Releases:** tagged releases build static or minimal-dependency binaries for Linux (x86\_64, aarch64) and macOS (Intel, Apple Silicon), with Windows added by 1.0. Checksums published with each release.
- **Distribution:** install script, GitHub releases, then package managers (Homebrew, AUR, cargo install, winget) as the project matures.
- **Versioning:** semantic versioning from 1.0; config format changes documented and migrated where possible.
- **Licensing:** to be decided (PRD open question 5).

## 16. Task Breakdown by Milestone

Each milestone is split into small tasks that fit one agent session and have clear tests.

### 0.1 Foundation

1. Workspace setup, CI, lint and format rules, benchmark skeleton
2. `kiln-text`: rope wrapper, grapheme width, line endings, encoding detection
3. `kiln-core`: buffer, selection, transaction, undo tree (with property tests)
4. `kiln-core`: command registry and basic edit commands
5. `kiln-input`: key parsing, mode state machine, Normal/Insert/Visual/Command keymaps
6. Vim motions and operators, first batch (`h j k l w b e 0 $ gg G d c y p x u`)
7. `kiln-ui`: buffer view, status line, cursor, one dark theme
8. File open, atomic save, headless test harness
9. `kiln-config`: TOML config with defaults and validation

### 0.2 Daily-usable editing

10. Splits, tabs and multiple buffers
11. Registers, system clipboard, search and replace, macros, dot repeat
12. `kiln-syntax`: tree-sitter highlighting with incremental reparse
13. Fuzzy file finder and project search services
14. Swap files and crash recovery; panic hook
15. Persistent undo

### 0.3 Language intelligence

16. LSP transport and manager (start, restart, backoff)
17. Diagnostics, hover, go to definition
18. Completion, rename, references, code actions
19. Language table and server auto-detection
20. Git service: gutter signs and branch display
21. File watcher and external change handling

### 0.4 Agent integration

22. PTY manager and terminal emulator integration; terminal pane
23. Agent presets in config; launcher UI
24. Change tracker, live file sync and conflict prompts
25. Snapshot store and rollback command
26. Diff view with hunk accept and reject
27. Send context to agent; prompt box; status line activity indicator
28. Project trust prompt for `.kiln.toml`

### 0.5 Polish and hardening

29. Command palette, help pages, which-key hints
30. Theme loader, light theme, color fallback
31. Mouse support, large-file and long-line handling
32. Error handling audit, fuzzing, performance tuning against budgets
33. Packaging for Linux and macOS

## 17. Risks and Technical Open Questions

| Item | Risk or question | Plan |
| --- | --- | --- |
| Terminal rendering library | ratatui may not meet frame-time goals on large screens | Prototype and measure before 0.1 UI work; keep drawing behind a trait |
| Embedded terminal emulator | Correctly emulating modern agent TUIs (colors, cursor control, resize) is hard | Use an existing emulator crate; test against real agent CLIs early in 0.4 |
| Snapshot timing | Fast agent writes could be observed after the fact | Pre-session hash list plus git baseline; simulate in tests |
| Abort vs unwind on panic | Abort is smaller and faster but prevents the panic hook from saving buffers | Keep unwinding; measure the cost; decide with data |
| Grammar size | Bundling many grammars grows the binary | Core set bundled, others downloaded on demand |
| Windows | ConPTY and console behavior differ from Unix | Tier 2 until 1.0; separate tests |
| Git library | `gix` and `git2` differ in size, maturity and dependencies | Quick comparison spike at the start of 0.3 |
| Vim compatibility depth | Edge cases can consume unlimited time | Documented subset; add by usage, not by completeness |
| Agent CLI changes | Agent commands and behavior change quickly | Presets are config data; verify before each release |

## 18. Decisions Needed Before Coding

1. Confirm the Rust crate layout in section 3.
2. Run the rendering prototype and pick ratatui or a custom renderer.
3. Choose the embedded terminal emulator crate with a small spike.
4. Set the Vim compatibility list for 0.1 and 0.2.
5. Pick a license.
6. Confirm the project name is clear to use publicly.

Once these are settled, the next document is a set of coding prompts, one per task in section 16, written for your AI coding agents.
