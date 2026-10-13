//! `kiln-bin`: main binary entrypoint for Kiln.

use std::io::stdout;
use std::time::Duration;

use kiln_core::{
    Buffer, BufferIdAllocator, CommandArg, CommandContext, CommandRegistry, EventBus, Registers,
    Text,
};
use kiln_input::{InputEngine, InputResult, KeyCode, KeyEvent, KeyModifiers, Mode};
use kiln_ui::crossterm::event::{self, Event, KeyCode as CKeyCode, KeyModifiers as CKeyModifiers};
use kiln_ui::ratatui::backend::CrosstermBackend;
use kiln_ui::ratatui::Terminal;
use kiln_ui::{
    BufferView, RatatuiRenderer, Renderer, StatusLine, StatusLineState, TerminalGuard, Theme,
};

/// Converts a crossterm key event to a terminal-independent `kiln_input::KeyEvent`.
fn crossterm_to_kiln_key(key: event::KeyEvent) -> Option<KeyEvent> {
    let modifiers = KeyModifiers {
        ctrl: key.modifiers.contains(CKeyModifiers::CONTROL),
        alt: key.modifiers.contains(CKeyModifiers::ALT),
        shift: key.modifiers.contains(CKeyModifiers::SHIFT),
    };

    let code = match key.code {
        CKeyCode::Char(c) => KeyCode::Char(c),
        CKeyCode::Enter => KeyCode::Enter,
        CKeyCode::Tab => KeyCode::Tab,
        CKeyCode::Backspace => KeyCode::Backspace,
        CKeyCode::Esc => KeyCode::Esc,
        CKeyCode::Left => KeyCode::Left,
        CKeyCode::Right => KeyCode::Right,
        CKeyCode::Up => KeyCode::Up,
        CKeyCode::Down => KeyCode::Down,
        CKeyCode::Home => KeyCode::Home,
        CKeyCode::End => KeyCode::End,
        CKeyCode::PageUp => KeyCode::PageUp,
        CKeyCode::PageDown => KeyCode::PageDown,
        CKeyCode::Delete => KeyCode::Delete,
        CKeyCode::F(n) => KeyCode::F(n),
        CKeyCode::Null => KeyCode::Null,
        _ => return None,
    };

    Some(KeyEvent { code, modifiers })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && (args[1] == "--version" || args[1] == "-V") {
        println!("kiln {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // Initialize terminal and panic hook
    let _guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(stdout());
    let terminal = Terminal::new(backend)?;
    let mut renderer = RatatuiRenderer::new(terminal)?;

    let mut allocator = BufferIdAllocator::new();
    let initial_text = "Welcome to Kiln.\nModal text editor with supervised AI agents.\nType :q or Ctrl-q to exit.";
    let mut buffer = Buffer::new(allocator.allocate(), Text::from(initial_text));

    let mut event_bus = EventBus::new();
    let mut registry = CommandRegistry::new();
    kiln_core::commands::register_builtins(&mut registry);

    let mut registers = Registers::new();
    let mut input_engine = InputEngine::new();
    let mut buffer_view = BufferView::default();
    let theme = Theme::dark();

    let mut should_exit = false;

    while !should_exit {
        let (_width, height) = renderer.size();
        let view_height = height.saturating_sub(1);

        renderer.clear();
        buffer_view.render(&mut renderer, &buffer, view_height, &theme);

        // Compute cursor and statusline state
        let primary_head = buffer.selections().primary().head;
        let line_col = buffer
            .text()
            .char_to_line_col(primary_head)
            .unwrap_or(kiln_core::LineCol::new(0, 0));
        let total_lines = buffer.text().len_lines().max(1);

        let percent_str = if line_col.line == 0 {
            "Top".to_string()
        } else if line_col.line + 1 >= total_lines {
            "Bot".to_string()
        } else {
            format!("{}%", ((line_col.line + 1) * 100) / total_lines)
        };

        let (mode_name, mode_style) = match input_engine.mode() {
            Mode::Normal => ("NORMAL", theme.mode_normal),
            Mode::Insert => ("INSERT", theme.mode_insert),
            Mode::Visual(_) => ("VISUAL", theme.mode_visual),
            Mode::Command => ("COMMAND", theme.mode_command),
            Mode::Search => ("SEARCH", theme.mode_command),
        };

        let status_state = StatusLineState {
            mode_name,
            mode_style,
            buffer_name: "[No Name]",
            is_modified: buffer.is_dirty(),
            is_readonly: buffer.is_read_only(),
            line: line_col.line + 1,
            col: line_col.col + 1,
            percent: &percent_str,
            encoding: "UTF-8",
            line_ending: "LF",
        };

        StatusLine::render(&mut renderer, view_height, &status_state, &theme);
        renderer.flush()?;

        // Wait up to ~16 ms for next input event
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key_event) => {
                    // Quick exit: Ctrl-q or Ctrl-c
                    if key_event.modifiers.contains(CKeyModifiers::CONTROL)
                        && (key_event.code == CKeyCode::Char('q')
                            || key_event.code == CKeyCode::Char('c'))
                    {
                        should_exit = true;
                        continue;
                    }

                    if let Some(kiln_key) = crossterm_to_kiln_key(key_event) {
                        match input_engine.handle_key(kiln_key) {
                            InputResult::ExecuteCommand { name, arg } => {
                                if name == "quit" || name == "vim_quit" {
                                    should_exit = true;
                                } else {
                                    let mut ctx = CommandContext {
                                        buffer: &mut buffer,
                                        event_bus: &mut event_bus,
                                        registers: &mut registers,
                                    };
                                    let _ = registry.execute(name, &mut ctx, &arg);
                                }
                            }
                            InputResult::InsertChar(c) => {
                                let mut ctx = CommandContext {
                                    buffer: &mut buffer,
                                    event_bus: &mut event_bus,
                                    registers: &mut registers,
                                };
                                let _ = registry.execute(
                                    "insert_text",
                                    &mut ctx,
                                    &CommandArg::Text(c.to_string()),
                                );
                            }
                            InputResult::SwitchMode(_) => {}
                            InputResult::VimAction(_) => {}
                            InputResult::Pending | InputResult::Ignored => {}
                        }
                    }
                }
                Event::Resize(_w, _h) => {
                    // Handled on next frame render
                }
                _ => {}
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bin_version() {
        assert_eq!(env!("CARGO_PKG_VERSION"), "0.0.1");
    }

    #[test]
    fn test_crossterm_key_conversion() {
        let ev = event::KeyEvent::new(CKeyCode::Char('k'), CKeyModifiers::NONE);
        let converted = crossterm_to_kiln_key(ev).unwrap();
        assert_eq!(converted.code, KeyCode::Char('k'));
        assert!(converted.modifiers.is_empty());

        let ev_ctrl = event::KeyEvent::new(CKeyCode::Char('c'), CKeyModifiers::CONTROL);
        let converted_ctrl = crossterm_to_kiln_key(ev_ctrl).unwrap();
        assert_eq!(converted_ctrl.code, KeyCode::Char('c'));
        assert!(converted_ctrl.modifiers.ctrl);
    }
}
