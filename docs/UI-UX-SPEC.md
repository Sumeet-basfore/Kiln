# Kiln — UI/UX Specification (Terminal)

This is the terminal equivalent of a frontend spec. All keybindings below are **proposed defaults** and are remappable in config. Where a key is not listed, follow Vim behavior.

## 1. Principles
1. Feels like Vim: modal, keyboard-first, no surprises for Vim users.
2. Nothing blocks: slow operations show progress and can be cancelled.
3. Always tell the user what mode they are in and what Kiln is doing.
4. Works in 80x24 and scales up; degrades cleanly in 16-color terminals, tmux and SSH.
5. The human is always in control of agent changes.

## 2. Screen layout
```text
+--------------------------------------------------------------+
| tabline: [1 main.rs] [2 lib.rs*] [3 agent: claude]           |
+--------+-----------------------------------+-----------------+
| file   | editor window(s)                  | optional right  |
| tree   |  gutter | text                    | pane: outline / |
| (opt.) |                                   | changed files   |
+--------+-----------------------------------+-----------------+
| agent / terminal pane (toggleable, bottom or side)           |
+--------------------------------------------------------------+
| statusline                                                   |
| message / command line                                       |
+--------------------------------------------------------------+
```
- **Tabline:** shown only with 2+ tabs. Marks modified (`*`) and conflicted (`!`) buffers; agent tabs show the agent name.
- **File tree:** left sidebar, toggled with `Space e`. Hidden by default in narrow terminals (under 100 columns).
- **Editor windows:** splits arranged in a tree. Each window has a gutter and the text area.
- **Agent/terminal pane:** toggled with `Space a t`; bottom by default, side when the terminal is wide (over 160 columns) or by config.
- **Right pane:** symbol outline (`Space o`) or changed-files list (`Space a c`), one at a time.
- **Status line:** one line, always visible. **Message line:** transient messages, the `:` command line, and search input.

## 3. Gutter
Left to right: git sign (added, modified, removed), diagnostic sign (error, warning, info, hint), line number (absolute, relative or hybrid by config; default hybrid), optional fold marker. Agent-edited lines use a distinct sign while the agent session is active or a review is pending. Gutter width is stable (does not jump while typing).

## 4. Status line
Segments, left to right, dropping from the right when narrow:
`MODE | file name [+ modified] [! conflict] [RO] | git branch | diagnostics (E:2 W:1) | LSP status | agent status | encoding · line ending | line:col | percent`
- Mode is a colored label: NORMAL, INSERT, VISUAL, V-LINE, V-BLOCK, COMMAND, SEARCH, REVIEW (diff review), PICK (finder/palette).
- Agent status: `agent:name running`, `idle`, `waiting`, or `exited(code)`. Hidden when no agent session exists.
- LSP status: spinner while starting or indexing; a warning sign when a server failed.

## 5. Modes and core keys
Standard Vim keys apply in Normal, Insert, Visual and Command modes (subset defined by decision D4). Additions:

| Key | Mode | Action |
|---|---|---|
| `Space` | Normal | Leader key |
| `Space f` | Normal | Fuzzy file finder |
| `Space b` | Normal | Buffer switcher |
| `Space /` | Normal | Project-wide text search |
| `Space e` | Normal | Toggle file tree |
| `Space o` | Normal | Symbol outline |
| `Space p` | Normal | Command palette |
| `Space ?` | Normal | Help and keybinding cheat sheet |
| `K` | Normal | Hover information |
| `gd` / `gr` | Normal | Go to definition / references |
| `Space r` | Normal | Rename symbol |
| `Space c` | Normal | Code actions |
| `[d` / `]d` | Normal | Previous / next diagnostic |
| `[g` / `]g` | Normal | Previous / next git hunk |
| `Ctrl-w` + `h j k l` | Normal | Move between windows |
| `Esc` | Any | Back to Normal mode / close overlay |

After a prefix key (`Space`, `g`, `[`, `]`, `Ctrl-w`), a **which-key hint** appears after 400 ms (configurable) listing the possible continuations.

## 6. Overlays (pickers and popups)
- **Fuzzy finder / buffer switcher / command palette / project search:** centered overlay with an input line and a results list, with a preview of the selected item when space allows. `Ctrl-n`/`Ctrl-p` or arrows move, `Enter` opens, `Esc` closes. Results stream in; the count and a spinner show progress. Project search is cancellable by typing or `Esc`.
- **Completion menu:** appears under the cursor during Insert mode; `Ctrl-n`/`Ctrl-p` move, `Tab` or `Enter` accepts, `Esc` dismisses. Max visible items and max total items are capped.
- **Hover popup:** near the cursor, scrollable with `Ctrl-d`/`Ctrl-u`, closes on any other key.
- **Confirmation prompts:** small centered box with explicit choices and a safe default highlighted (for example conflict resolution, trust prompt, recovery prompt).
- Overlays never resize the underlying layout and always restore the previous mode on close.

## 7. Agent workflow UI
### 7.1 Keys (proposed defaults)
| Key | Action |
|---|---|
| `Space a o` | Open agent launcher (choose preset) |
| `Space a t` | Toggle agent pane focus/visibility |
| `Space a n` | Next agent session |
| `Space a s` | Send selection / current line (with file path and range) to the agent |
| `Space a p` | Open prompt box, type an instruction, send it with the selection as context |
| `Space a d` | Send current file's diagnostics to the agent |
| `Space a c` | Open changed-files panel |
| `Space a r` | Open diff review |
| `Space a u` | Roll back (confirmation required) |
| `Ctrl-\ Ctrl-n` | From the agent pane: return to the editor |

### 7.2 Agent pane
- A full terminal emulator view. While focused, keystrokes go to the agent. The pane title shows the preset name and status.
- `Ctrl-\ Ctrl-n` leaves the pane; the pane keeps running in the background.
- Scrollback is capped (default 10,000 lines) and scrollable with the mouse or copy mode.
- When the process exits, the pane shows `process exited (code)` and keeps its scrollback.

### 7.3 Changed-files panel
List of files touched since the session started: status icon (modified, created, deleted), path, number of hunks, and review state (unreviewed, partly reviewed, accepted, rejected). `Enter` opens that file's diff review; `x` marks a file for rollback (confirmation required).

### 7.4 Diff review (mode: REVIEW)
- Inline or side-by-side view, switchable (`Tab`); default inline on narrow terminals, side-by-side when wide.
- Hunks are highlighted with a word-level diff inside changed lines. The current hunk is outlined.
- Keys: `]h` / `[h` next/previous hunk, `]f` / `[f` next/previous file, `a` accept, `x` reject (reverts to baseline as an undoable edit), `s` skip, `A` accept all in file, `X` reject all in file (confirmation), `b` toggle baseline between session snapshot and git HEAD, `q` leave review.
- A footer shows progress: `file 2/5 · hunk 3/7 · 12 accepted · 3 rejected`.
- Rejecting a hunk in a buffer with unsaved user edits is allowed only through the conflict prompt.

### 7.5 Conflict prompt
When an agent changes a file with unsaved user edits: a prompt with `Keep mine`, `Take theirs`, `View diff` (default), and `Decide later`. The buffer is marked `!` until resolved.

## 8. First run and help
- On first launch, show a one-screen welcome: three keys to learn (`Space ?`, `Space f`, `Space p`), where the config lives, and how to open the tutorial.
- `:help` opens a built-in help buffer (read-only) with topics: basics, keys, LSP, agents, config, troubleshooting. `:tutor` opens an interactive Vim tutorial buffer.

## 9. Messages, errors and empty states
- Messages are one line, specific and actionable: what happened, which file, what to try. Example: `config.toml:12: unknown key "tabwidth" (did you mean "tab_width")? using defaults`.
- Severity levels: info (neutral), warning (yellow), error (red). Errors that need action open a prompt; others appear on the message line and in a message history (`:messages`).
- Empty states: finder with no results shows `No matches`; agent pane with no preset configured shows how to add one; changed-files panel with no changes shows `No changes since session start`.
- Long operations (indexing, search, large file load) show a spinner and a count; `Esc` cancels where possible.

## 10. Themes and colors
- Themes are TOML files mapping scopes to colors and modifiers. Required scopes include: `ui.background`, `ui.text`, `ui.cursor`, `ui.selection`, `ui.statusline`, `ui.gutter`, `ui.popup`, `ui.menu`, `diagnostic.error|warning|info|hint`, `diff.plus|minus|delta`, `agent.running|idle|waiting`, and all syntax scopes (`keyword`, `string`, `comment`, `function`, `type`, `variable`, `constant`, `operator`, ...).
- Ship at least one dark and one light theme.
- Detect color support at startup (true color, 256, 16) and downgrade automatically; in 16-color mode rely on modifiers (bold, underline, reverse) so meaning does not depend on exact hues.
- Never encode meaning by color alone: errors also use a sign and underline; modified buffers also use `*`.
- Colors must keep a readable contrast ratio for text on its background in both shipped themes.

## 11. Mouse and clipboard
- Mouse is optional (default on, toggleable): click to place cursor, drag to select, scroll windows, click tabs and file tree entries, click gutter signs for details.
- Clipboard: yank and paste use the system clipboard when available (registers `+` and `*`), with a fallback for SSH (terminal clipboard sequences only if the user enables them; see SECURITY.md section 6).

## 12. Accessibility and terminals
- Fully usable with the keyboard only. All actions reachable through the command palette.
- Respects `NO_COLOR` and a reduced-motion setting (no spinners that animate when set).
- Handles terminal resize, focus events and tmux correctly. Works with wide characters and combining marks.
- Minimum supported size: 80x24. Below that, show a clear message instead of a broken layout.

## 13. Configurable UI options (initial list)
Line numbers mode · relative numbers · tab width · soft wrap · scrolloff · mouse · theme · file tree position and width · agent pane position and size · which-key delay · diff view default (inline or side-by-side) · agent scrollback · show tabline (auto, always, never).
