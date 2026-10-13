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
