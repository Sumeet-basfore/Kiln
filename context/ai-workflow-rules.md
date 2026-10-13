# Kiln — AI Workflow Rules

These rules apply to every AI coding agent working on this repository (Codex, Claude Code, Antigravity CLI, Cursor CLI, OpenCode, others).

## Before you start
1. Read, in this order: `context/project-overview.md`, `context/architecture-context.md`, `context/code-standards.md`, `context/progress-tracker.md`, `BRAIN.md`.
2. Read the ticket you were given in `docs/TICKETS.md`, and any section of `docs/TDD.md`, `docs/SECURITY.md` or `docs/UI-UX-SPEC.md` it references.
3. Run `git status` and the test suite to confirm the starting state is clean and green. If it is not, stop and report.

## Scope discipline
- Work on **one ticket at a time**. Do exactly what the ticket's acceptance criteria say.
- Do not refactor unrelated code, rename things, or "improve" other crates in the same change.
- Do not add features that are not in the ticket. Write unrelated ideas to `BRAIN.md` under "Ideas".
- Do not change the dependency rule, crate layout or public APIs of other crates without asking.
- Do not add a dependency without the justification required in `code-standards.md`.

## When unsure, ask
Stop and ask the human when:
- The ticket and the TDD/PRD/SECURITY disagree.
- A decision listed under "Open decisions" is needed.
- The ticket cannot be done without touching another ticket's scope.
- You would have to guess a requirement.
Do not invent requirements and do not silently pick between conflicting documents.

## Definition of done
A ticket is done only when all of these are true:
1. Every acceptance criterion in the ticket is met.
2. Tests listed in the ticket exist and pass; `cargo test --workspace` is green.
3. `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` pass.
4. No benchmark regression on any hot path touched.
5. `context/progress-tracker.md` is updated (status, date, short note).
6. Anything important learned (decisions, gotchas, follow-ups) is added to `BRAIN.md`.
7. The change is one logical commit referencing the ticket ID.

## Safety rules for this repo
- Never run destructive commands (`rm -rf`, `git reset --hard`, force-push) without explicit approval.
- Never delete or overwrite files outside the scope of the ticket.
- Never commit secrets, tokens or personal paths.
- Never weaken a security rule in `docs/SECURITY.md` to make a test pass.
- Never disable, skip or delete a failing test to get to green. Fix the cause or report it.

## Reporting back
End every session with a short report:
- Ticket ID and what was done.
- Files changed (grouped by crate).
- Tests added and their results.
- Anything skipped, uncertain or needing a human decision.
- Suggested next ticket.

## Git conventions
- Branch per ticket: `t07-status-line`. Small commits. Message format: `T07: add status line`.
- Do not merge or push to `main` without human approval.
