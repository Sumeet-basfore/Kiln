# AGENTS.md — Kiln

Vendor-neutral instructions for AI coding agents (Codex, Cursor CLI, OpenCode, Antigravity CLI, Claude Code and others). `CLAUDE.md` and `GEMINI.md` point here.

## Project in one paragraph
Kiln is a lightweight, production-ready, Vim-style modal terminal editor written in Rust, with built-in LSP, tree-sitter and first-class support for supervising external AI coding agents. Single binary, one TOML config, no plugins in v1.

## Read first, every session
1. `context/project-overview.md`
2. `context/architecture-context.md`
3. `context/code-standards.md`
4. `context/ai-workflow-rules.md`
5. `context/progress-tracker.md`
6. `BRAIN.md`
7. Your ticket in `docs/TICKETS.md`, plus the sections of `docs/TDD.md`, `docs/SECURITY.md`, `docs/UI-UX-SPEC.md` it references.

Authority when documents conflict: `docs/SECURITY.md` > `docs/TDD.md` > `docs/PRD.md` > others. Report conflicts; do not pick silently.

## Non-negotiable rules
- **One ticket per session.** Do exactly what the acceptance criteria say; nothing more.
- **Respect the crate dependency rule** (downward only; `kiln-core` and `kiln-text` have no I/O, terminal or async dependencies).
- **Never block the editor thread.** All slow work is async and cancellable.
- **Never lose user work.** Atomic saves, no silent overwrites, agent changes never overwrite unsaved edits.
- **No `unwrap()`/`expect()`** in non-test code without a justified comment. No `unsafe` without a `// SAFETY:` comment and a test.
- **Security first.** Follow `docs/SECURITY.md`; run the checklist in its section 14 for every ticket.
- **Ask when unsure.** Stop on conflicting docs, open decisions (D1–D6) or ambiguous requirements.
- **Never** skip or delete failing tests, run destructive commands without approval, or add dependencies without justification.

## Commands
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo bench            # when touching hot paths
cargo audit            # or cargo deny check, before adding dependencies
```
All of the first three must pass before a ticket is done.

## Definition of done
Acceptance criteria met · tests written and green · fmt + clippy clean · no benchmark regression · `context/progress-tracker.md` updated · `BRAIN.md` updated · one logical commit `T##: message`.

## Final report format
Ticket ID and summary · files changed by crate · tests added and results · anything skipped or uncertain · suggested next ticket.

## Where things live
```text
crates/   kiln-text, kiln-core, kiln-syntax, kiln-config, kiln-services, kiln-input, kiln-ui, kiln-bin
docs/     PRD.md, TDD.md, SECURITY.md, UI-UX-SPEC.md, TICKETS.md
context/  the five context files listed above
BRAIN.md  decisions, gotchas, ideas (living log)
```
