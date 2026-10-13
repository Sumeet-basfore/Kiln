//! Built-in editing and cursor movement commands for `kiln-core`.

use crate::command::{
    CommandArg, CommandContext, CommandMeta, CommandRegistry, CommandResult, MoveDirection,
};
use crate::error::CoreError;
use crate::event::EditorEvent;
use crate::selection::{Range, Selections};
use crate::transaction::Transaction;

/// Registers all core built-in edit commands into the provided registry.
pub fn register_builtins(registry: &mut CommandRegistry) {
    registry.register(CommandMeta {
        name: "insert_text",
        description: "Insert text at current cursor positions",
        handler: cmd_insert_text,
    });
    registry.register(CommandMeta {
        name: "delete_char",
        description: "Delete character at cursor or delete non-empty selections",
        handler: cmd_delete_char,
    });
    registry.register(CommandMeta {
        name: "backspace",
        description: "Delete character before cursor or delete non-empty selections",
        handler: cmd_backspace,
    });
    registry.register(CommandMeta {
        name: "newline",
        description: "Insert newline at cursor positions",
        handler: cmd_newline,
    });
    registry.register(CommandMeta {
        name: "move_cursor",
        description: "Move cursor in the specified direction",
        handler: cmd_move_cursor,
    });
    registry.register(CommandMeta {
        name: "undo",
        description: "Undo the last change",
        handler: cmd_undo,
    });
    registry.register(CommandMeta {
        name: "redo",
        description: "Redo the previously undone change",
        handler: cmd_redo,
    });
    registry.register(CommandMeta {
        name: "vim_x",
        description: "Delete character under cursor and yank to register",
        handler: cmd_vim_x,
    });
    registry.register(CommandMeta {
        name: "vim_paste_after",
        description: "Paste register content after cursor",
        handler: cmd_vim_paste_after,
    });
    registry.register(CommandMeta {
        name: "vim_paste_before",
        description: "Paste register content before cursor",
        handler: cmd_vim_paste_before,
    });
    registry.register(CommandMeta {
        name: "vim_open_below",
        description: "Open line below cursor",
        handler: cmd_vim_open_below,
    });
    registry.register(CommandMeta {
        name: "vim_open_above",
        description: "Open line above cursor",
        handler: cmd_vim_open_above,
    });
    registry.register(CommandMeta {
        name: "vim_append",
        description: "Append after cursor",
        handler: cmd_vim_append,
    });
    registry.register(CommandMeta {
        name: "vim_append_line_end",
        description: "Append at end of line",
        handler: cmd_vim_append_line_end,
    });
    registry.register(CommandMeta {
        name: "vim_insert",
        description: "Insert at cursor",
        handler: cmd_vim_insert,
    });
    registry.register(CommandMeta {
        name: "vim_insert_line_start",
        description: "Insert at first non-blank character of line",
        handler: cmd_vim_insert_line_start,
    });
    registry.register(CommandMeta {
        name: "vim_join_lines",
        description: "Join current line with next line",
        handler: cmd_vim_join_lines,
    });
}

/// Command: `insert_text`
///
/// Inserts text at each selection. If a selection is non-empty, it is replaced.
pub fn cmd_insert_text(
    ctx: &mut CommandContext<'_>,
    arg: &CommandArg,
) -> Result<CommandResult, CoreError> {
    let text_to_insert = match arg {
        CommandArg::Text(s) => s.as_str(),
        CommandArg::None => "",
        _ => {
            return Err(CoreError::InvalidArgument {
                message: "insert_text requires Text or None argument",
            })
        }
    };

    if text_to_insert.is_empty() {
        return Ok(CommandResult {
            affected_buffer: Some(ctx.buffer.id()),
            count_applied: 0,
        });
    }

    let doc_len = ctx.buffer.text().len_chars();
    let ranges = ctx.buffer.selections().ranges();

    let edits: Vec<(std::ops::Range<usize>, &str)> = ranges
        .iter()
        .map(|r| (r.from()..r.to(), text_to_insert))
        .collect();

    let tx = Transaction::from_edits(doc_len, &edits)?;
    let buffer_id = ctx.buffer.id();
    let res = ctx.buffer.apply(tx)?;

    ctx.event_bus.publish(EditorEvent::BufferChanged {
        buffer_id,
        old_version: res.old_version,
        new_version: res.new_version,
        transaction: res.transaction,
    });

    Ok(CommandResult {
        affected_buffer: Some(buffer_id),
        count_applied: 1,
    })
}

/// Command: `delete_char`
///
/// If selection is non-empty, deletes the selection.
/// If selection is empty (point), deletes the character at the cursor position (like 'x' or 'Delete').
pub fn cmd_delete_char(
    ctx: &mut CommandContext<'_>,
    arg: &CommandArg,
) -> Result<CommandResult, CoreError> {
    let count = match arg {
        CommandArg::Count(c) => *c,
        CommandArg::None => 1,
        _ => {
            return Err(CoreError::InvalidArgument {
                message: "delete_char accepts Count or None argument",
            })
        }
    };

    if count == 0 {
        return Ok(CommandResult {
            affected_buffer: Some(ctx.buffer.id()),
            count_applied: 0,
        });
    }

    let doc_len = ctx.buffer.text().len_chars();
    if doc_len == 0 {
        return Ok(CommandResult {
            affected_buffer: Some(ctx.buffer.id()),
            count_applied: 0,
        });
    }

    let ranges = ctx.buffer.selections().ranges();
    let mut edits: Vec<(std::ops::Range<usize>, &str)> = Vec::new();

    for r in ranges {
        if !r.is_empty() {
            edits.push((r.from()..r.to(), ""));
        } else {
            let start = r.head;
            if start < doc_len {
                let end = (start + count).min(doc_len);
                edits.push((start..end, ""));
            }
        }
    }

    if edits.is_empty() {
        return Ok(CommandResult {
            affected_buffer: Some(ctx.buffer.id()),
            count_applied: 0,
        });
    }

    let tx = Transaction::from_edits(doc_len, &edits)?;
    let buffer_id = ctx.buffer.id();
    let res = ctx.buffer.apply(tx)?;

    ctx.event_bus.publish(EditorEvent::BufferChanged {
        buffer_id,
        old_version: res.old_version,
        new_version: res.new_version,
        transaction: res.transaction,
    });

    Ok(CommandResult {
        affected_buffer: Some(buffer_id),
        count_applied: 1,
    })
}

/// Command: `backspace`
///
/// If selection is non-empty, deletes the selection.
/// If selection is empty (point), deletes the character before the cursor position.
pub fn cmd_backspace(
    ctx: &mut CommandContext<'_>,
    arg: &CommandArg,
) -> Result<CommandResult, CoreError> {
    let count = match arg {
        CommandArg::Count(c) => *c,
        CommandArg::None => 1,
        _ => {
            return Err(CoreError::InvalidArgument {
                message: "backspace accepts Count or None argument",
            })
        }
    };

    if count == 0 {
        return Ok(CommandResult {
            affected_buffer: Some(ctx.buffer.id()),
            count_applied: 0,
        });
    }

    let doc_len = ctx.buffer.text().len_chars();
    if doc_len == 0 {
        return Ok(CommandResult {
            affected_buffer: Some(ctx.buffer.id()),
            count_applied: 0,
        });
    }

    let ranges = ctx.buffer.selections().ranges();
    let mut edits: Vec<(std::ops::Range<usize>, &str)> = Vec::new();

    for r in ranges {
        if !r.is_empty() {
            edits.push((r.from()..r.to(), ""));
        } else {
            let head = r.head;
            if head > 0 {
                let start = head.saturating_sub(count);
                edits.push((start..head, ""));
            }
        }
    }

    if edits.is_empty() {
        return Ok(CommandResult {
            affected_buffer: Some(ctx.buffer.id()),
            count_applied: 0,
        });
    }

    let tx = Transaction::from_edits(doc_len, &edits)?;
    let buffer_id = ctx.buffer.id();
    let res = ctx.buffer.apply(tx)?;

    ctx.event_bus.publish(EditorEvent::BufferChanged {
        buffer_id,
        old_version: res.old_version,
        new_version: res.new_version,
        transaction: res.transaction,
    });

    Ok(CommandResult {
        affected_buffer: Some(buffer_id),
        count_applied: 1,
    })
}

/// Command: `newline`
///
/// Inserts a newline `\n` at the cursor position.
pub fn cmd_newline(
    ctx: &mut CommandContext<'_>,
    _arg: &CommandArg,
) -> Result<CommandResult, CoreError> {
    cmd_insert_text(ctx, &CommandArg::Text("\n".to_string()))
}

/// Command: `move_cursor`
///
/// Moves cursor in the requested direction. Non-mutating to text; does not emit BufferChanged.
pub fn cmd_move_cursor(
    ctx: &mut CommandContext<'_>,
    arg: &CommandArg,
) -> Result<CommandResult, CoreError> {
    let (dir, count) = match arg {
        CommandArg::Direction(d) => (*d, 1),
        _ => {
            return Err(CoreError::InvalidArgument {
                message: "move_cursor requires Direction argument",
            })
        }
    };

    let text = ctx.buffer.text();
    let total_chars = text.len_chars();
    let current_selections = ctx.buffer.selections();

    let mut new_ranges = Vec::with_capacity(current_selections.ranges().len());

    for range in current_selections.ranges() {
        let head = range.head;
        let new_head = match dir {
            MoveDirection::Left => head.saturating_sub(count),
            MoveDirection::Right => (head + count).min(total_chars),
            MoveDirection::LineStart => {
                if let Ok(lc) = text.char_to_line_col(head) {
                    text.line_col_to_char(kiln_text::LineCol::new(lc.line, 0))
                        .unwrap_or(0)
                } else {
                    0
                }
            }
            MoveDirection::LineEnd => {
                if let Ok(lc) = text.char_to_line_col(head) {
                    if let Ok(line_slice) = text.line(lc.line) {
                        let line_start = text
                            .line_col_to_char(kiln_text::LineCol::new(lc.line, 0))
                            .unwrap_or(0);
                        let mut line_len = line_slice.chars().count();
                        if line_slice.ends_with('\n') {
                            line_len = line_len.saturating_sub(1);
                        }
                        (line_start + line_len).min(total_chars)
                    } else {
                        head
                    }
                } else {
                    head
                }
            }
            MoveDirection::Up => {
                if let Ok(lc) = text.char_to_line_col(head) {
                    let target_line = lc.line.saturating_sub(count);
                    move_to_line_col(text, target_line, lc.col, total_chars)
                } else {
                    head
                }
            }
            MoveDirection::Down => {
                if let Ok(lc) = text.char_to_line_col(head) {
                    let num_lines = text.len_lines();
                    let target_line = (lc.line + count).min(num_lines.saturating_sub(1));
                    move_to_line_col(text, target_line, lc.col, total_chars)
                } else {
                    head
                }
            }
        };

        // For standard cursor movement, new selection collapses to point
        new_ranges.push(Range::point(new_head));
    }

    let primary = current_selections.primary_index();
    let new_selections = Selections::new(new_ranges, primary)?;
    ctx.buffer.set_selections(new_selections);

    Ok(CommandResult {
        affected_buffer: Some(ctx.buffer.id()),
        count_applied: 1,
    })
}

fn move_to_line_col(
    text: &kiln_text::Text,
    line_idx: usize,
    target_col: usize,
    total_chars: usize,
) -> usize {
    let line_start = text
        .line_col_to_char(kiln_text::LineCol::new(line_idx, 0))
        .unwrap_or(0);
    if let Ok(line_slice) = text.line(line_idx) {
        let mut line_len = line_slice.chars().count();
        if line_slice.ends_with('\n') {
            line_len = line_len.saturating_sub(1);
        }
        let col = target_col.min(line_len);
        (line_start + col).min(total_chars)
    } else {
        line_start.min(total_chars)
    }
}

/// Command: `undo`
///
/// Undoes the last committed edit in history and publishes `BufferChanged`.
pub fn cmd_undo(
    ctx: &mut CommandContext<'_>,
    _arg: &CommandArg,
) -> Result<CommandResult, CoreError> {
    let buffer_id = ctx.buffer.id();
    if let Some(res) = ctx.buffer.undo()? {
        ctx.event_bus.publish(EditorEvent::BufferChanged {
            buffer_id,
            old_version: res.old_version,
            new_version: res.new_version,
            transaction: res.transaction,
        });
        Ok(CommandResult {
            affected_buffer: Some(buffer_id),
            count_applied: 1,
        })
    } else {
        Ok(CommandResult {
            affected_buffer: Some(buffer_id),
            count_applied: 0,
        })
    }
}

/// Command: `redo`
///
/// Redoes the previously undone edit in history and publishes `BufferChanged`.
pub fn cmd_redo(
    ctx: &mut CommandContext<'_>,
    _arg: &CommandArg,
) -> Result<CommandResult, CoreError> {
    let buffer_id = ctx.buffer.id();
    if let Some(res) = ctx.buffer.redo()? {
        ctx.event_bus.publish(EditorEvent::BufferChanged {
            buffer_id,
            old_version: res.old_version,
            new_version: res.new_version,
            transaction: res.transaction,
        });
        Ok(CommandResult {
            affected_buffer: Some(buffer_id),
            count_applied: 1,
        })
    } else {
        Ok(CommandResult {
            affected_buffer: Some(buffer_id),
            count_applied: 0,
        })
    }
}

/// Command: `vim_x`
///
/// Deletes the character under the cursor and yanks it to the unnamed register.
pub fn cmd_vim_x(
    ctx: &mut CommandContext<'_>,
    arg: &CommandArg,
) -> Result<CommandResult, CoreError> {
    let count = match arg {
        CommandArg::Count(c) => *c,
        CommandArg::None => 1,
        _ => {
            return Err(CoreError::InvalidArgument {
                message: "vim_x accepts Count or None",
            })
        }
    };

    let doc_len = ctx.buffer.text().len_chars();
    if doc_len == 0 {
        return Ok(CommandResult::default());
    }

    let head = ctx.buffer.selections().primary().head;
    if head >= doc_len {
        return Ok(CommandResult::default());
    }

    let end = (head + count).min(doc_len);
    if let Ok(deleted) = ctx.buffer.text().slice(head..end) {
        ctx.registers.set(
            None,
            crate::register::RegisterValue::character(deleted.to_string()),
        );
    }

    let tx = Transaction::delete_range(doc_len, head..end)?;
    let buffer_id = ctx.buffer.id();
    let res = ctx.buffer.apply(tx)?;

    ctx.event_bus.publish(EditorEvent::BufferChanged {
        buffer_id,
        old_version: res.old_version,
        new_version: res.new_version,
        transaction: res.transaction,
    });

    Ok(CommandResult {
        affected_buffer: Some(buffer_id),
        count_applied: 1,
    })
}

/// Command: `vim_paste_after`
///
/// Pastes register content after cursor (or on the next line if linewise).
pub fn cmd_vim_paste_after(
    ctx: &mut CommandContext<'_>,
    _arg: &CommandArg,
) -> Result<CommandResult, CoreError> {
    let reg_val = match ctx.registers.get(None) {
        Some(v) => v.clone(),
        None => return Ok(CommandResult::default()),
    };

    let doc_len = ctx.buffer.text().len_chars();
    let head = ctx.buffer.selections().primary().head;

    let (insert_pos, insert_text) = if reg_val.is_linewise {
        let line_idx = ctx
            .buffer
            .text()
            .char_to_line_col(head)
            .map(|lc| lc.line)
            .unwrap_or(0);
        let next_line_start = ctx
            .buffer
            .text()
            .line_col_to_char(kiln_text::LineCol::new(line_idx + 1, 0))
            .unwrap_or(doc_len);
        let text = if reg_val.text.ends_with('\n') {
            reg_val.text
        } else {
            format!("{}\n", reg_val.text)
        };
        (next_line_start, text)
    } else {
        let pos = (head + 1).min(doc_len);
        (pos, reg_val.text)
    };

    let tx = Transaction::insert_at(doc_len, insert_pos, &insert_text)?;
    let buffer_id = ctx.buffer.id();
    let res = ctx.buffer.apply(tx)?;

    ctx.event_bus.publish(EditorEvent::BufferChanged {
        buffer_id,
        old_version: res.old_version,
        new_version: res.new_version,
        transaction: res.transaction,
    });

    Ok(CommandResult {
        affected_buffer: Some(buffer_id),
        count_applied: 1,
    })
}

/// Command: `vim_paste_before`
///
/// Pastes register content before cursor (or on the current line if linewise).
pub fn cmd_vim_paste_before(
    ctx: &mut CommandContext<'_>,
    _arg: &CommandArg,
) -> Result<CommandResult, CoreError> {
    let reg_val = match ctx.registers.get(None) {
        Some(v) => v.clone(),
        None => return Ok(CommandResult::default()),
    };

    let doc_len = ctx.buffer.text().len_chars();
    let head = ctx.buffer.selections().primary().head;

    let (insert_pos, insert_text) = if reg_val.is_linewise {
        let line_idx = ctx
            .buffer
            .text()
            .char_to_line_col(head)
            .map(|lc| lc.line)
            .unwrap_or(0);
        let line_start = ctx
            .buffer
            .text()
            .line_col_to_char(kiln_text::LineCol::new(line_idx, 0))
            .unwrap_or(0);
        let text = if reg_val.text.ends_with('\n') {
            reg_val.text
        } else {
            format!("{}\n", reg_val.text)
        };
        (line_start, text)
    } else {
        (head, reg_val.text)
    };

    let tx = Transaction::insert_at(doc_len, insert_pos, &insert_text)?;
    let buffer_id = ctx.buffer.id();
    let res = ctx.buffer.apply(tx)?;

    ctx.event_bus.publish(EditorEvent::BufferChanged {
        buffer_id,
        old_version: res.old_version,
        new_version: res.new_version,
        transaction: res.transaction,
    });

    Ok(CommandResult {
        affected_buffer: Some(buffer_id),
        count_applied: 1,
    })
}

/// Command: `vim_open_below`
///
/// Inserts a newline at the end of the current line and positions cursor there.
pub fn cmd_vim_open_below(
    ctx: &mut CommandContext<'_>,
    _arg: &CommandArg,
) -> Result<CommandResult, CoreError> {
    let doc_len = ctx.buffer.text().len_chars();
    let head = ctx.buffer.selections().primary().head;
    let line_idx = ctx
        .buffer
        .text()
        .char_to_line_col(head)
        .map(|lc| lc.line)
        .unwrap_or(0);

    let line_str = ctx.buffer.text().line(line_idx).unwrap_or_default();
    let has_newline = line_str.ends_with('\n');
    let line_start = ctx
        .buffer
        .text()
        .line_col_to_char(kiln_text::LineCol::new(line_idx, 0))
        .unwrap_or(0);
    let line_end = line_start + line_str.chars().count();

    let (insert_pos, cursor_pos) = if has_newline {
        (line_end, line_end)
    } else {
        (line_end, line_end + 1)
    };

    let mut tx = Transaction::insert_at(doc_len, insert_pos, "\n")?;
    tx.selections_after = Some(Selections::point(cursor_pos));

    let buffer_id = ctx.buffer.id();
    let res = ctx.buffer.apply(tx)?;

    ctx.event_bus.publish(EditorEvent::BufferChanged {
        buffer_id,
        old_version: res.old_version,
        new_version: res.new_version,
        transaction: res.transaction,
    });

    Ok(CommandResult {
        affected_buffer: Some(buffer_id),
        count_applied: 1,
    })
}

/// Command: `vim_open_above`
///
/// Inserts a newline at the start of the current line and positions cursor there.
pub fn cmd_vim_open_above(
    ctx: &mut CommandContext<'_>,
    _arg: &CommandArg,
) -> Result<CommandResult, CoreError> {
    let doc_len = ctx.buffer.text().len_chars();
    let head = ctx.buffer.selections().primary().head;
    let line_idx = ctx
        .buffer
        .text()
        .char_to_line_col(head)
        .map(|lc| lc.line)
        .unwrap_or(0);

    let line_start = ctx
        .buffer
        .text()
        .line_col_to_char(kiln_text::LineCol::new(line_idx, 0))
        .unwrap_or(0);

    let mut tx = Transaction::insert_at(doc_len, line_start, "\n")?;
    tx.selections_after = Some(Selections::point(line_start));

    let buffer_id = ctx.buffer.id();
    let res = ctx.buffer.apply(tx)?;

    ctx.event_bus.publish(EditorEvent::BufferChanged {
        buffer_id,
        old_version: res.old_version,
        new_version: res.new_version,
        transaction: res.transaction,
    });

    Ok(CommandResult {
        affected_buffer: Some(buffer_id),
        count_applied: 1,
    })
}

/// Command: `vim_append`
///
/// Moves cursor 1 grapheme right (for entering insert mode).
pub fn cmd_vim_append(
    ctx: &mut CommandContext<'_>,
    _arg: &CommandArg,
) -> Result<CommandResult, CoreError> {
    let doc_len = ctx.buffer.text().len_chars();
    let head = ctx.buffer.selections().primary().head;
    let new_head = (head + 1).min(doc_len);
    ctx.buffer.set_selections(Selections::point(new_head));
    Ok(CommandResult::default())
}

/// Command: `vim_append_line_end`
///
/// Moves cursor to the end of the line (for entering insert mode).
pub fn cmd_vim_append_line_end(
    ctx: &mut CommandContext<'_>,
    _arg: &CommandArg,
) -> Result<CommandResult, CoreError> {
    let head = ctx.buffer.selections().primary().head;
    let end = crate::motion::MotionEvaluator::line_end(ctx.buffer.text(), head);
    ctx.buffer.set_selections(Selections::point(end));
    Ok(CommandResult::default())
}

/// Command: `vim_insert`
///
/// Prepares for insert mode at the current cursor position.
pub fn cmd_vim_insert(
    _ctx: &mut CommandContext<'_>,
    _arg: &CommandArg,
) -> Result<CommandResult, CoreError> {
    Ok(CommandResult::default())
}

/// Command: `vim_insert_line_start`
///
/// Moves cursor to the first non-blank character of the line.
pub fn cmd_vim_insert_line_start(
    ctx: &mut CommandContext<'_>,
    _arg: &CommandArg,
) -> Result<CommandResult, CoreError> {
    let head = ctx.buffer.selections().primary().head;
    let first = crate::motion::MotionEvaluator::first_non_blank(ctx.buffer.text(), head);
    ctx.buffer.set_selections(Selections::point(first));
    Ok(CommandResult::default())
}

/// Command: `vim_join_lines`
///
/// Joins the current line with the next line (replaces `\n` and leading whitespace with single space).
pub fn cmd_vim_join_lines(
    ctx: &mut CommandContext<'_>,
    _arg: &CommandArg,
) -> Result<CommandResult, CoreError> {
    let doc_len = ctx.buffer.text().len_chars();
    let head = ctx.buffer.selections().primary().head;
    let line_idx = ctx
        .buffer
        .text()
        .char_to_line_col(head)
        .map(|lc| lc.line)
        .unwrap_or(0);

    if line_idx + 1 >= ctx.buffer.text().len_lines() {
        return Ok(CommandResult::default());
    }

    let line_slice = ctx.buffer.text().line(line_idx)?;
    if !line_slice.ends_with('\n') {
        return Ok(CommandResult::default());
    }

    let line_start = ctx
        .buffer
        .text()
        .line_col_to_char(kiln_text::LineCol::new(line_idx, 0))?;
    let newline_pos = line_start + line_slice.chars().count() - 1;

    let next_line_slice = ctx.buffer.text().line(line_idx + 1)?;
    let mut leading_ws = 0;
    for ch in next_line_slice.chars() {
        if ch.is_whitespace() && ch != '\n' {
            leading_ws += 1;
        } else {
            break;
        }
    }

    let del_start = newline_pos;
    let del_end = newline_pos + 1 + leading_ws;

    let tx = Transaction::replace_range(doc_len, del_start..del_end, " ")?;
    let buffer_id = ctx.buffer.id();
    let res = ctx.buffer.apply(tx)?;

    ctx.event_bus.publish(EditorEvent::BufferChanged {
        buffer_id,
        old_version: res.old_version,
        new_version: res.new_version,
        transaction: res.transaction,
    });

    Ok(CommandResult {
        affected_buffer: Some(buffer_id),
        count_applied: 1,
    })
}
