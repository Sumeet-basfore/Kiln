# Kiln

A lightweight, Vim-style modal terminal editor written in Rust, built for supervising
external AI coding agents (Claude Code, Codex, and others) directly inside the terminal.
Single static binary, one TOML config file, no plugin system in v1.

> **Status:** early development. The core editor is functional, but language intelligence,
> splits/tabs, and the AI-agent PTY workspace are still being built out (see the [roadmap](#roadmap)).

## Vision

Kiln treats the terminal as a canvas: edit files with Vim keys while agent panes run
side-by-side in PTYs, share context with agents with a single keystroke, and review
changes with first-class diff views. v1 targets a fast, correct, dependency-light
single-binary editor foundation; the agent supervison workspace arrives in v2.

## Building

Kiln targets **Rust 1.99** (see `rust-toolchain.toml`). It is licensed under an
undecided license (tracked as decision D5); use at your own risk until 1.0.

```bash
# Quick check / build
cargo build                # debug build
cargo build --release      # optimized, LTO

# Checks that must pass before any PR
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/check-deps.sh      # enforces downward-only crate dependencies
cargo audit                # vulnerability scan
```

## Usage

```bash
# Open a file (Vim keybindings)
./target/debug/kiln path/to/file.txt

# Headless / scripted session for testing or automation
./target/debug/kiln --headless --keys 'iHello<Esc>:wq<Enter>' file.txt

# Version
./target/debug/kiln --version

# Global config lives at:
#   macOS / Linux:   $XDG_CONFIG_HOME/kiln/config.toml  (default ~/.config/kiln/config.toml)
#   Windows:         %APPDATA%\kiln\config.toml
```

### Global configuration

Kiln loads optional global configuration from `config.toml`. Invalid sections fall back
to defaults independently — a typo in `[editor]` does not discard your `[theme]` settings.
Diagnostics are line-aware and are reported (non-fatally) at startup.

```toml
[editor]
tab_width = 4
scrolloff = 3
mouse = true
line_numbers = true   # hybrid gutter; false disables line numbers

[keybindings.modes.normal]
# Rebind "<keys>" = "<command>"; only no-argument, built-in commands are accepted.
u = "redo"           # multi-key sequences supported, e.g. "g d"
# "<NotAKey>" = "undo"   # -> ignored with a warning

[theme]
file = "dark.toml"            # optional custom theme file (resolved relative to config dir)
```

Custom theme file format (`dark.toml`):

```toml
name = "My Theme"

[scopes."ui.text"]
foreground = "gray"      # or RGB: foreground = [200, 200, 200]
background = [18, 18, 26]
modifiers = ["bold"]     # bold | italic | underline | reverse
```

See `docs/PRD.md` and `context/` for the full design and `context/progress-tracker.md`
for the current ticket status.

## Project layout

```text
crates/
  kiln-text     - UTF-8/UTF-16/Latin-1 ropes, graphemes, line endings, binary detection
  kiln-core     - buffer, selections, transactions, undo tree, command registry, event bus
  kiln-syntax   - future home for tree-sitter integration (T12)
  kiln-config   - TOML config loader, defaults, validation, theme parsing, JSON schema
  kiln-services - stdlib-only, synchronous byte I/O (read / atomic save)
  kiln-input    - terminal-agnostic key parsing, mode state machine, keymap trie, Vim grammar
  kiln-ui       - renderer trait + ratatui/crossterm backend, buffer view, gutter, status line
  kiln-bin        - CLI entrypoint, EditorSession driver, interactive + headless
benches/          - criterion benchmarks (developer tooling)
fuzz/             - cargo-fuzz target for the config loader
docs/             - PRD, TDD, SECURITY, UI/UX spec, tickets, architecture context
context/          - progress tracker and architecture/conventions docs
```

Crates form a strict downward dependency hierarchy (enforced by `scripts/check-deps.sh`):
library crates depend only on `kiln-text`/`kiln-core` and a small allow-listed set of
external crates; all disk I/O and decoding is confined to `kiln-services` and
`kiln-bin` respectively.

## Roadmap

| Milestone | Focus | Status |
|-----------|-------|--------|
| 0.1 Foundation | Core editing, UI, file open/save | ✅ T01–T09 |
| 0.2 Daily-usability | Splits/tabs, search, registers, tree-sitter | T10–T15 |
| 0.3 Language intelligence | LSP transport, diagnostics, completion | T16–T19 |
| 0.4 Agent integration | PTY terminal, agent presets, trust prompts | T20–T28 |
| 0.5 Polish | Command palette, themes, mouse, packaging | T29–T33 |

See `context/progress-tracker.md` for the full ticket list.
