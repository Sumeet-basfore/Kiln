# CLAUDE.md — Kiln

@AGENTS.md

The shared instructions are in `AGENTS.md` (imported above). Everything there applies. The notes below are specific to Claude Code.

## Claude Code specifics
- Start each session by reading the files listed in `AGENTS.md`, then restate the ticket's acceptance criteria in your own words before editing code.
- Use plan mode for any ticket that touches more than one crate, adds a dependency, or changes a public API. Show the plan and wait for approval.
- Prefer small, reviewable diffs. Run `cargo fmt`, `cargo clippy` and `cargo test` after each meaningful change, not only at the end.
- Use subagents only for read-only research (for example comparing candidate crates); do not let a subagent edit files outside the ticket's scope.
- Do not create new top-level files or directories without asking.
- Before finishing, run the full checks from `AGENTS.md`, update `context/progress-tracker.md` and `BRAIN.md`, and produce the final report in the format given in `AGENTS.md`.
- If a command would be destructive (`rm -rf`, `git reset --hard`, force push, deleting files outside the ticket), stop and ask.
