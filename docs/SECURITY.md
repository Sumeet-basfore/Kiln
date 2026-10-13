# Kiln — Security and Access

Authority: this document overrides the TDD and PRD where they conflict. Agents must never weaken a rule here to make a test pass.

## 1. Security goals
1. Kiln never runs code, agents or language servers the user did not intend to run.
2. Kiln never silently overwrites or deletes user work.
3. Kiln never sends project content anywhere on its own.
4. A malicious project (config, files, symlinks) cannot take control of the user's machine through Kiln.
5. Failures degrade safely: the editor stays usable and data is preserved.

## 2. Trust boundaries
| Boundary | Trusted? | Notes |
|---|---|---|
| User's global config (`~/.config/kiln/config.toml`) | Yes | User-authored |
| Project config (`.kiln.toml`) | **No, until the user trusts the project** | Can define commands, so it is a code-execution vector |
| Files in the project | No | Content is data; never executed by Kiln |
| Language servers | Semi | Run as the user, started from a known table or trusted config |
| Agent CLIs | Semi | Run as the user; started only by explicit user action |
| Agent output in the PTY | No | Treated as terminal data; escape sequences are sanitized (see 6) |
| Plugins (v2, WASM) | No | Sandboxed, capability-based |
| Network | Not used by default | See section 8 |

## 3. Project trust model
- A `.kiln.toml` may define: language server commands, agent presets, environment variables, formatters. These can execute programs, so they are **ignored until the project is trusted**.
- On first open of a project with a `.kiln.toml`, show a prompt listing exactly which commands the file would enable. Options: Trust, Trust once, Don't trust (default).
- Trust is stored per canonical project path plus a hash of the `.kiln.toml` contents. If the file changes, trust is **revoked** and the prompt is shown again.
- Untrusted projects still open normally with built-in defaults and the global config.
- Never auto-trust based on git remote, directory name or file location.

## 4. Process execution rules
- Kiln starts processes only for: language servers, agent presets, user-run commands, and formatters, all from the global config, the built-in language table, or a trusted project config.
- Commands are spawned with an explicit program and argument list. **Never build a shell command string from file content, filenames or agent output.** If a shell is required, the user's command is passed as a single configured string and never concatenated with untrusted data.
- Environment: pass through the user's environment; project config may add variables but may not remove security-relevant ones. Never log environment values.
- Agent sessions start only on an explicit user command. No auto-start, no restart-on-crash for agents.
- Language server auto-start is allowed only for servers in the built-in table or in trusted config; a missing server produces a message, never a download or install.

## 5. File access and data safety
- Atomic saves (temp file in the same directory, flush, rename). Preserve permissions; never follow a symlink out of the project to write unless the user opened that path explicitly.
- When an external change hits a buffer with unsaved edits, **never overwrite either side silently**: show the conflict prompt.
- Agent changes: never auto-accept hunks into a buffer with unsaved user edits. Rejecting a hunk is an undoable edit.
- Snapshots (rollback) are stored under the Kiln state directory with permissions `0700` (owner only on Unix) and exclude ignored paths and very large files by default.
- Swap, undo and session files are created with owner-only permissions. Do not store them inside the project unless configured.
- Recovery files and snapshots never contain more than what the user's buffers and agent-touched files already contain.
- Deleting snapshots/recovery data is allowed only for Kiln's own files after the session ends or retention expires; never delete files outside the Kiln state directory.

## 6. Terminal and escape-sequence safety
- Output in the agent pane and any text displayed from files, LSP messages, diagnostics, hover content and filenames is **sanitized**: control characters and unsupported escape sequences are stripped or shown as visible placeholders, except where the terminal emulator interprets them inside the agent pane's own virtual screen.
- The agent pane emulator must not be able to: change the host terminal's title to arbitrary text without limits, write to the clipboard (OSC 52) unless the user enables it, trigger host-level hyperlinks that execute, or reconfigure the host terminal. Unsupported sequences are dropped.
- Pasted text is bracketed. Sending context to an agent uses bracketed paste so multi-line content cannot execute as separate commands.
- Never write text from an untrusted source to the host terminal without sanitizing it first.

## 7. Language server and LSP rules
- LSP responses are untrusted data. Validate ranges and URIs; reject edits that target files outside the workspace unless the user confirms (for example rename across files shows a list of files first).
- Workspace edits from LSP are applied as normal undoable transactions.
- Servers receive only the document content of files the user opened or the workspace configured; never other files from disk on Kiln's behalf.
- Request timeouts and output size limits apply to every server; a misbehaving server is stopped, not trusted.

## 8. Network and privacy
- Kiln itself makes **no network connections** in v1, apart from what the user's own language servers and agents do. No telemetry. No update checks without an explicit opt-in setting.
- On-demand grammar downloads (if implemented): opt-in only, from a pinned source, with checksum verification, never executed code beyond the grammar loading step. If this cannot be made safe, grammars stay bundled.
- Kiln does not read or store agent conversations. Only the bytes the user explicitly sends reach the agent's PTY.
- Logs are local, contain no file contents or secrets, and are size-limited and rotated.

## 9. Agent safety model
- The user is always in control: start, stop, review, accept, reject, roll back.
- Kiln does not grant an agent any more access than the user's own shell would; it does not sandbox agents in v1. This must be stated in the documentation.
- Kiln provides detection and recovery (changed-files panel, snapshots, diff review), not prevention. Do not claim otherwise.
- Agent presets are config data. Presets from untrusted project config are disabled until trust is granted.
- Sending context (selection, path, diagnostics) is always an explicit key press; there is no background sharing.

## 10. Agent protocol server (v2) requirements
- Disabled by default. Enabled per project with an explicit setting.
- Local-only socket (Unix socket or Windows named pipe) in a user-owned directory with owner-only permissions. No TCP listening.
- Token authentication per client; tokens are random, short-lived or rotated, never logged.
- Explicit permission list per client (read buffers, read diagnostics, open file, propose edits). No permission to run commands or write files directly; edits go through the review flow.
- Rate limits and size limits on every request.

## 11. Plugin system (v2) requirements
- WASM sandbox with capability-based permissions declared in a manifest and approved by the user: filesystem scope, network hosts, process spawning.
- Default: no filesystem, no network, no processes.
- Plugins run with CPU and memory limits and cannot block the editor thread.
- Plugin install from a Git URL requires a pinned version/commit and shows the requested capabilities before enabling.

## 12. Supply chain and build security
- Pin the Rust toolchain. Commit `Cargo.lock`.
- CI runs `cargo audit` (or `cargo deny`) for known vulnerabilities and licence checks. Failing audit blocks release.
- Review every new dependency: maintenance, size, licence, unsafe usage. Minimal features.
- Release binaries are built in CI from tagged commits; publish checksums (and signatures when available).
- Bundled grammars and queries are vendored with recorded source versions.

## 13. Denial of service and robustness
- Limits: maximum completion items, search results held in memory, undo history size, agent scrollback lines, diagnostics per file, swap file size.
- Huge files and very long lines never freeze the UI; binary files open read-only.
- Parsing and search run off the editor thread and are cancellable.
- Fuzz the input parser, config loader and buffer operations (see TDD section 14).

## 14. Security checklist per ticket
An agent must check, for every ticket, whether it:
- Spawns a process (then follow section 4).
- Reads or writes files outside the open buffers (then follow section 5).
- Displays text from outside the editor (then follow section 6).
- Handles LSP or agent input (then follow sections 6 and 7).
- Adds a dependency (then follow section 12).
If yes, state in the ticket report how the rules were applied.

## 15. Reporting vulnerabilities
Add a `SECURITY.md` to the repository root before 1.0 with a private reporting contact and response expectations. (Decision needed: contact method.)
