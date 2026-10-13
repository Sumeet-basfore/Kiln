# Kiln — Product Requirements Document

**Author:** Sumeet\
**Status:** Draft v0.1\
**Last updated:** October 13, 2026\
**One-line summary:** Kiln is a fast, lightweight, batteries-included modal terminal editor, written in Rust, built for production use and designed from day one to work with AI coding agents.

---

## 1. Overview

Kiln is a terminal-based text and code editor with Vim-style modal editing. It targets developers who like the Neovim workflow but do not want to assemble and maintain a large plugin setup, and who want a first-class place to run and supervise AI coding agents such as Claude Code, Codex, Antigravity CLI and OpenCode.

Kiln ships as a single static binary. LSP, syntax highlighting, fuzzy finding, a file tree, git integration and AI agent tooling are built in. Configuration is one small TOML file. There is no plugin system in v1; a sandboxed WASM plugin system is planned for v2.

## 2. Problem Statement

Developers who want a fast terminal editor face a trade-off today:

- **Neovim** is powerful and fast, but a usable setup requires many plugins, a Lua config and ongoing maintenance. A broken plugin update can break the whole workflow.
- **Helix** is batteries-included and light, but has no plugin system yet and uses a different editing model from Vim.
- **Kakoune** is minimal and elegant, but has a small ecosystem and requires external tools for most features.
- **VS Code, Zed and similar GUI editors** are polished but heavy, and are awkward over SSH or inside tmux.
- **AI coding agents** run as separate CLI tools. Editors treat them as an afterthought: the user switches panes manually, files change underneath the editor, and reviewing agent changes is clumsy.

There is room for an editor that is light, complete out of the box, safe to depend on, and built around the way developers now work with agents.

## 3. Goals and Non-Goals

### 3.1 Goals

1. **Production-ready.** Stable, crash-safe, and trustworthy with real work and large files.
2. **Lightweight.** Fast startup, low memory, small binary, no runtime dependencies.
3. **Zero-config useful.** Works well immediately after install with no setup.
4. **Familiar.** Vim-compatible modal editing so existing Vim and Neovim users feel at home.
5. **Agent-native.** Run, observe, review and steer external AI coding agents from inside the editor.
6. **Remote-friendly.** Works well over SSH, in tmux and on low-power machines.

### 3.2 Non-Goals (v1)

- Building a custom AI agent or model integration inside the editor. Kiln hosts existing agent CLIs instead.
- A plugin system. This is deferred to v2 to avoid locking in a bad API.
- A GUI or graphical frontend.
- Full Vimscript or Neovim Lua API compatibility.
- Collaborative or real-time multi-user editing.
- Replacing a full IDE (debugger UI, project templates, build system integrations are out of scope for v1).

## 4. Target Users

| Persona | Description | Main need |
| --- | --- | --- |
| Vim/Neovim user tired of config | Experienced with modal editing, spends time maintaining plugins | Same workflow with no setup and no breakage |
| Agent-heavy developer | Uses Claude Code, Codex, OpenCode or similar daily in the terminal | A tight loop between agent edits and human review |
| Remote/server developer | Works over SSH, in containers, on small VPS machines | A light editor that starts instantly and has modern features |
| Student/beginner using modal editing | Wants a gentler start than a bare Neovim | Sensible defaults and built-in help |

## 5. Product Principles

1. **Small core, strong defaults.** Include what most developers need; leave out the rest.
2. **Never block the UI.** Anything slow (LSP, search, git, file loading) runs asynchronously.
3. **Never lose work.** Swap files, persistent undo and atomic saves are mandatory.
4. **The human stays in control.** Agent changes are always reviewable and reversible.
5. **Agent-agnostic.** No hard dependency on any single agent or vendor.
6. **Boring and reliable beats clever.** Prefer proven designs and libraries.

## 6. Functional Requirements

Priority levels: **P0** = required for the milestone, **P1** = should have, **P2** = nice to have.

### 6.1 Core Editing

| ID | Requirement | Priority |
| --- | --- | --- |
| ED-1 | Rope-based text buffer with efficient edits on very large files | P0 |
| ED-2 | Modes: Normal, Insert, Visual (char, line, block), Command, Search | P0 |
| ED-3 | Core Vim motions, operators and text objects (e.g. `w b e`, `d c y`, `iw aw i( a"`) | P0 |
| ED-4 | Counts and repeat (`3dw`, `.`) | P0 |
| ED-5 | Registers, yank/paste, system clipboard integration | P0 |
| ED-6 | Undo/redo with persistent undo history across sessions | P0 |
| ED-7 | Macros (`q`, `@`) | P1 |
| ED-8 | Marks and jump list | P1 |
| ED-9 | Multiple buffers, splits and tabs | P0 |
| ED-10 | Search and replace with regex (`/`, `?`, `:s`) | P0 |
| ED-11 | Unicode support including wide characters and combining marks | P0 |
| ED-12 | Line endings and encoding detection (UTF-8 primary; UTF-16 and Latin-1 read support) | P1 |
| ED-13 | Folding based on syntax tree | P2 |
| ED-14 | Optional Helix-style selection-first keymap | P2 |

### 6.2 Language Intelligence

| ID | Requirement | Priority |
| --- | --- | --- |
| LS-1 | Tree-sitter syntax highlighting with bundled grammars for common languages | P0 |
| LS-2 | LSP client: diagnostics, hover, go to definition, references, rename, completion | P0 |
| LS-3 | Auto-detect and launch language servers from a built-in language table, overridable in config | P0 |
| LS-4 | Code actions and format on save through LSP | P1 |
| LS-5 | Signature help and inlay hints | P1 |
| LS-6 | Tree-sitter based text objects and smart indentation | P1 |
| LS-7 | Symbol outline and workspace symbol search | P1 |

### 6.3 Navigation and Project Tools

| ID | Requirement | Priority |
| --- | --- | --- |
| NV-1 | Built-in fuzzy file finder (respects `.gitignore`) | P0 |
| NV-2 | Built-in project-wide text search (ripgrep-style engine, no external dependency) | P0 |
| NV-3 | Buffer switcher and recent files list | P0 |
| NV-4 | File tree sidebar | P0 |
| NV-5 | Command palette listing all commands and keybindings | P0 |
| NV-6 | Git integration: gutter signs, branch in status line, blame line, stage/unstage hunks | P1 |
| NV-7 | Session restore (open files, layout, cursor positions) | P1 |

### 6.4 User Interface

| ID | Requirement | Priority |
| --- | --- | --- |
| UI-1 | Terminal UI with true color and graceful 256-color/16-color fallback | P0 |
| UI-2 | Status line with mode, file, git branch, diagnostics count, cursor position | P0 |
| UI-3 | Built-in themes (at least one dark and one light) and user theme files | P0 |
| UI-4 | Mouse support (click, scroll, select), optional and toggleable | P1 |
| UI-5 | Built-in help and keybinding cheat sheet (`:help`) | P0 |
| UI-6 | Which-key style hints after a prefix key is pressed | P1 |
| UI-7 | Works correctly inside tmux and over SSH | P0 |

### 6.5 Reliability and Safety

| ID | Requirement | Priority |
| --- | --- | --- |
| RL-1 | Atomic saves (write to temp file, then rename) | P0 |
| RL-2 | Swap/recovery files with crash recovery prompt on next open | P0 |
| RL-3 | Detect external file changes and prompt or auto-reload safely | P0 |
| RL-4 | Graceful handling of binary files and very long lines | P0 |
| RL-5 | Language server crashes never crash the editor; automatic restart with backoff | P0 |
| RL-6 | Panic handler that saves buffers before exit | P0 |

### 6.6 Configuration

| ID | Requirement | Priority |
| --- | --- | --- |
| CF-1 | Single TOML config file (`~/.config/kiln/config.toml`) with documented defaults | P0 |
| CF-2 | Per-project override file (`.kiln.toml`) | P1 |
| CF-3 | Custom keybindings (remap keys per mode) | P0 |
| CF-4 | Custom themes in TOML | P0 |
| CF-5 | Config validation with clear error messages; the editor never fails to start because of a bad config (falls back to defaults and shows a warning) | P0 |
| CF-6 | Hot reload of config | P1 |

## 7. AI Agent Integration

This is Kiln's main differentiator. The editor does not contain its own agent. It is the best place to run and supervise external agent CLIs.

### 7.1 Supported Agents

Any agent that runs as a terminal command works. First-class presets (command, arguments, and detection) are provided for:

- Claude Code
- Codex
- Antigravity CLI
- OpenCode
- Cursor CLI
- Generic custom command (user-defined in config)

Preset details must be verified against each tool's current CLI before release, since these tools change quickly.

### 7.2 Agent Requirements

| ID | Requirement | Priority |
| --- | --- | --- |
| AG-1 | Built-in terminal pane (PTY) that can run any command, toggled with a keybinding | P0 |
| AG-2 | Agent launcher: choose an agent preset and start it in the project root in a dedicated pane | P0 |
| AG-3 | Multiple agent sessions at once, each in its own named pane or tab | P1 |
| AG-4 | Live file sync: files changed on disk by an agent are detected and open buffers reload automatically (unless the buffer has unsaved edits, in which case show a conflict prompt) | P0 |
| AG-5 | Changed-files panel: list of files modified since the agent session started, with status | P0 |
| AG-6 | Diff review view: hunk-by-hunk accept and reject of agent changes against the last git commit or a session snapshot | P0 |
| AG-7 | Session snapshots: take an automatic snapshot before an agent runs so any change can be rolled back with one command, independent of git | P1 |
| AG-8 | Send context to agent: send current selection, file path with line range, or LSP diagnostics to the running agent pane with one keypress | P0 |
| AG-9 | Send-and-prompt: open a small prompt box, type an instruction, and send it together with selected context to the agent | P1 |
| AG-10 | Agent activity indicator in the status line (running, idle, waiting for input) | P1 |
| AG-11 | Highlight lines currently being edited by the agent in open buffers | P2 |
| AG-12 | Per-project agent config in `.kiln.toml` (default agent, extra arguments, environment variables) | P1 |

### 7.3 Agent Protocol Server (v2 target)

In v2, Kiln exposes a local, opt-in protocol server so agents can query and drive the editor:

- Read open files, current cursor position, selection and diagnostics.
- Open a file at a line, show a diff, and request user review.
- Candidate protocols to evaluate: Agent Client Protocol (ACP) and Model Context Protocol (MCP). Their current status and fit must be researched before design starts.
- Security: local-only socket, token-authenticated, disabled by default, with a clear permission model.

### 7.4 Safety Principles for Agent Features

- Kiln never runs an agent automatically; the user starts it explicitly.
- Kiln never auto-accepts agent changes to files with unsaved user edits.
- Kiln does not send file contents anywhere by itself. Only the agent process the user launched receives what the user explicitly sends.
- All agent-related features work offline with local agents.

## 8. Performance and Quality Targets

Targets are measured on a mid-range laptop (4 cores, 8 GB RAM, SSD) unless noted.

| Metric | Target |
| --- | --- |
| Cold startup to usable editor (no file) | under 30 ms |
| Startup with a typical project open (10 files, LSP starting async) | under 100 ms to first paint |
| Idle memory (one small file, no LSP) | under 50 MB |
| Open a 100 MB text file | under 1 second to first screen; no UI freeze |
| Keystroke to screen latency | under 16 ms (one frame at 60 Hz) |
| Binary size (stripped, release, grammars included) | under 40 MB; stretch goal under 25 MB |
| Project search over 100k files | first results under 200 ms |
| Crash rate | zero data-loss crashes in internal testing before 1.0 |

These targets should be tracked with automated benchmarks in CI so regressions are caught early.

## 9. Technical Approach

### 9.1 Language and Libraries

Kiln is written in **Rust** for speed, memory safety and a single static binary. Proposed building blocks (to be confirmed during the architecture phase):

| Area | Candidate |
| --- | --- |
| Text buffer | `ropey` |
| Terminal I/O and UI | `crossterm`, `ratatui` (or a custom renderer if needed for performance) |
| Syntax | `tree-sitter` with bundled grammars |
| LSP | `lsp-types` plus a custom async client on `tokio` |
| Async runtime | `tokio` |
| PTY for agent panes | `portable-pty` plus a terminal emulator crate such as `vt100` or `alacritty_terminal` |
| File watching | `notify` |
| Fuzzy matching | `nucleo` |
| Search | `grep-searcher` / `ignore` (ripgrep internals) |
| Git | `gix` (gitoxide) or `git2` |
| Config | `serde` with `toml` |

### 9.2 Architecture Outline

- **Core:** buffers, selections, edit operations, undo tree, modes and the command/keymap system. No UI or I/O dependencies, so it is easy to test.
- **Event bus:** all state changes (buffer changed, file saved, diagnostics updated, agent file change detected) are emitted as internal events. This is the foundation for the v2 plugin API.
- **UI layer:** renders state to the terminal; separate from core.
- **Services:** LSP manager, file watcher, git service, search service, agent/PTY manager. Each runs asynchronously and communicates through messages.
- **Config and theme loader:** validates TOML and applies defaults.

Keeping these as separate crates in a Cargo workspace enforces boundaries and supports testing.

### 9.3 Platform Support

- **Tier 1:** Linux (x86\_64, aarch64), macOS (Apple Silicon and Intel).
- **Tier 2:** Windows 10/11 (via ConPTY), with PTY and terminal behavior tested separately.
- Static or minimal-dependency builds distributed as single binaries.

## 10. Extensibility and Plugin Roadmap

**v1:** no plugin system. Customization through config, keybindings and themes only. The internal event bus and clean module boundaries are designed so a plugin API can be added without a rewrite.

**v2:** sandboxed **WASM plugins** (for example via `wasmtime` and the component model).

- Plugins can be written in any language that compiles to WASM.
- Capability-based permissions (filesystem, network, process) declared in a manifest and approved by the user.
- API surface starts small: commands, keybindings, status line segments, buffer events and simple UI panels.
- Simple plugin install from a Git URL or registry, with version pinning.

**Decision rule:** add a plugin API only after the core features are stable and at least a few real use cases cannot be served by built-in features or config.

## 11. Milestones and Roadmap

### Milestone 0.1 — Foundation (editor you can edit a file in)

- Rope buffer, rendering, Normal/Insert/Visual/Command modes, core motions and operators
- Open, edit, save (atomic), undo/redo
- Config file, one dark theme
- Benchmark harness in CI

### Milestone 0.2 — Daily-usable editing

- Splits, tabs, multiple buffers, registers, search and replace, macros
- Tree-sitter highlighting
- Fuzzy file finder and project search
- Swap files and crash recovery

### Milestone 0.3 — Language intelligence

- LSP client with diagnostics, hover, go to definition, completion, rename
- Built-in language table with auto-detected servers
- Git gutter and status line

### Milestone 0.4 — Agent integration

- Terminal pane and agent launcher with presets
- Live file sync and changed-files panel
- Diff review with hunk accept/reject
- Send context to agent; session snapshots

### Milestone 0.5 — Polish and hardening

- Command palette, help, which-key hints, themes, mouse support
- Large-file and long-line handling, error handling audit
- Packaging for Linux and macOS

### Milestone 1.0 — Production release

- All P0 requirements complete and P1 mostly complete
- Performance targets met and documented
- Windows support in beta
- Documentation site, install scripts, and package manager entries

### Post-1.0 (v2)

- WASM plugin system
- Agent protocol server (ACP/MCP evaluation)
- Helix-style keymap option
- Remote editing and further agent features

## 12. Testing Strategy

- **Unit tests** for the core (motions, operators, undo tree, text objects) with property-based testing for buffer operations.
- **Snapshot tests** for rendering output of the UI layer.
- **Integration tests** driving the editor with scripted key sequences against fixture files.
- **LSP tests** using mock language servers plus smoke tests against real servers (rust-analyzer, typescript-language-server, pyright, gopls).
- **Fuzzing** of the buffer, input parser and config loader.
- **Performance benchmarks** in CI for startup, large file open, and keystroke latency, failing on regressions.
- **Crash and recovery tests** simulating kill signals during edits and saves.
- **Cross-platform CI** on Linux, macOS and Windows.
- **Dogfooding:** the author uses Kiln for real work starting from Milestone 0.3.

## 13. Success Metrics

- Performance targets in section 8 are met on the reference machine.
- Author uses Kiln as the primary editor for at least a month before 1.0.
- A new user can install, open a project and edit with LSP and highlighting working in under 2 minutes with no config.
- Zero known data-loss bugs at 1.0.
- Community signals after launch: GitHub stars, issues opened, and first-time contributors (targets to be set after launch).

## 14. Risks and Mitigations

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Scope is very large for a solo project | Project stalls before it is usable | Strict milestones; ship a usable editor at 0.2; defer plugins, protocol server and Helix keymap |
| Vim compatibility is deep and has many edge cases | Users hit missing behavior and leave | Define a documented compatibility list; prioritize the 90% of commands used daily |
| Terminal emulator inside the editor is hard to get right | Broken agent panes | Use a proven emulator crate; test against the target agent CLIs early |
| Agent CLIs and protocols change quickly | Presets break, integrations go stale | Keep presets as config data, not code; verify before each release |
| Competing with Neovim and Helix | Low adoption | Focus on the clear differentiators: zero-config, agent-native workflow, small footprint |
| Windows terminal behavior differs | Bugs and delays | Treat Windows as Tier 2 until 1.0; use ConPTY and dedicated tests |
| Hand-coding with AI agents can produce inconsistent architecture | Hard-to-maintain code | Write architecture and conventions docs before coding; keep modules small; require tests per feature |
| Language grammars increase binary size | Misses size target | Bundle a core set; allow on-demand grammar download |

## 15. Open Questions

1. Should Kiln be fully Vim-compatible or only compatible with the commonly used subset? (Proposed: common subset, documented.)
2. Custom TUI renderer or `ratatui`? Decide after a rendering performance prototype.
3. Bundle all grammars, or download on demand?
4. Which of ACP and MCP, if either, is the right protocol for the v2 agent server?
5. Licensing (MIT or Apache-2.0 are likely candidates).
6. How should project-level `.kiln.toml` files be trusted, since they can launch commands? (Proposed: explicit trust prompt per project.)
7. Final name check: confirm that "Kiln" does not conflict with an existing well-known editor or trademark before public release.

## 16. Appendix: Competitive Snapshot

| Editor | Strengths | Gaps that Kiln targets |
| --- | --- | --- |
| Neovim | Huge ecosystem, Vim model, very fast | Heavy setup and maintenance; agent workflow depends on plugins |
| Helix | Batteries-included, light | No plugin system, different keymap, no agent tooling |
| Kakoune | Minimal, elegant model | Small ecosystem, needs external tools |
| Zed | Polished, strong AI features | GUI only, heavier, not terminal-first |
| VS Code | Largest ecosystem | Heavy; poor fit for SSH-first workflows |
