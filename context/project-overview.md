# Kiln — Project Overview

> Read this file first, at the start of every session.

## What Kiln is
Kiln is a fast, lightweight, batteries-included **modal terminal editor** written in **Rust**, with Vim-style editing and first-class support for supervising external **AI coding agents** (Claude Code, Codex, Antigravity CLI, OpenCode, Cursor CLI). It ships as a single binary with LSP, tree-sitter, fuzzy finding, file tree and git integration built in, and one small TOML config file.

## Why it exists
- Neovim is powerful but needs many plugins and constant maintenance.
- Helix is light but has no plugin system and no agent tooling.
- GUI editors are heavy and awkward over SSH.
- Nobody treats agent CLIs as a first-class part of the editor.

## Goals
1. Production-ready: crash-safe, never loses work, handles large files.
2. Lightweight: startup under 30 ms, idle memory under 50 MB, binary under 40 MB.
3. Zero-config useful: works well right after install.
4. Familiar: Vim-compatible modal editing (documented common subset).
5. Agent-native: run, observe, review and steer external agents from the editor.
6. Remote-friendly: works well over SSH and in tmux.

## Non-goals (v1)
- A built-in AI agent or model integration (Kiln hosts external agent CLIs).
- A plugin system (planned for v2 as sandboxed WASM).
- A GUI frontend.
- Full Vimscript / Neovim Lua compatibility.
- Collaborative editing; full IDE features such as a debugger UI.

## Key decisions already made
- Language: Rust. Workspace of small crates (see `architecture-context.md`).
- Single editor thread owns all state; services are async tasks that send messages.
- Everything is a command; every state change emits an event (foundation for v2 plugins).
- Config: one TOML file plus optional per-project `.kiln.toml` (requires a trust prompt).
- Agents are ordinary programs in a PTY pane. No agent-specific code; presets are config data.

## Source documents
- `docs/PRD.md` — product requirements (export from the PRD doc)
- `docs/TDD.md` — technical design (export from the TDD doc)
- `docs/SECURITY.md` — security and access rules
- `docs/UI-UX-SPEC.md` — terminal UI and keybinding spec
- `docs/TICKETS.md` — feature tickets (T01–T33), the unit of work for agents
- `context/progress-tracker.md` — current status

If documents conflict, the order of authority is: `SECURITY.md` > `TDD.md` > `PRD.md` > everything else. Report the conflict; do not silently pick one.

## Milestones
0.1 Foundation → 0.2 Daily-usable editing → 0.3 Language intelligence → 0.4 Agent integration → 0.5 Polish and hardening → 1.0 Production release. v2: WASM plugins, agent protocol server.
