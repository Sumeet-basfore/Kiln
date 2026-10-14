//! `kiln-bin`: terminal entrypoint for Kiln.

use std::io::stdout;
use std::path::PathBuf;
use std::time::Duration;

use kiln_bin::{apply_theme_file, EditorSession, SaveRequest, SessionAction};
use kiln_input::{KeyCode, KeyEvent, KeyModifiers, Mode};
use kiln_services::file_io;
use kiln_ui::crossterm::event::{self, Event, KeyCode as CKeyCode, KeyModifiers as CKeyModifiers};
use kiln_ui::ratatui::backend::CrosstermBackend;
use kiln_ui::ratatui::Terminal;
use kiln_ui::{
    BufferView, Gutter, LineNumberMode, RatatuiRenderer, Renderer, StatusLine, StatusLineState,
    TerminalGuard, Theme,
};

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

#[derive(Debug, PartialEq, Eq)]
struct CliArgs {
    path: Option<PathBuf>,
    headless: bool,
    keys: Option<String>,
}

fn parse_args(args: &[String]) -> Result<CliArgs, String> {
    let mut headless = false;
    let mut keys = None;
    let mut path = None;
    let mut index = 1;

    while index < args.len() {
        match args[index].as_str() {
            "--headless" => headless = true,
            "--keys" => {
                index += 1;
                keys = Some(
                    args.get(index)
                        .ok_or("--keys requires a key script")?
                        .clone(),
                );
            }
            "--version" | "-V" => {}
            option if option.starts_with('-') => return Err(format!("unknown option: {option}")),
            value if path.is_none() => path = Some(PathBuf::from(value)),
            value => return Err(format!("unexpected argument: {value}")),
        }
        index += 1;
    }

    if headless && keys.is_none() {
        return Err("--headless requires --keys <script>".to_string());
    }
    Ok(CliArgs {
        path,
        headless,
        keys,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args
        .iter()
        .skip(1)
        .any(|arg| arg == "--version" || arg == "-V")
    {
        println!("kiln {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let cli = parse_args(&args).map_err(std::io::Error::other)?;
    let Some(path) = cli.path else {
        return Err("usage: kiln [--headless --keys <script>] <file>".into());
    };

    if cli.headless {
        let keys = cli.keys.as_deref().unwrap_or_default();
        let session = kiln_bin::run_headless(path, keys).map_err(std::io::Error::other)?;
        if let Some(message) = session.message() {
            eprintln!("{message}");
        }
        if session.exit_requested() {
            return Ok(());
        }
        if session.buffer().is_dirty() {
            return Err("headless script ended with unsaved changes".into());
        }
        return Ok(());
    }

    run_interactive(path)
}

fn run_interactive(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()?;
    let loaded_config = kiln_config::load_global();
    let config_path = kiln_config::global_config_path();
    let mut session = EditorSession::empty(Some(path.clone()));
    let mut config_warnings = session.apply_config(loaded_config);
    let mut load_result = None;
    let load_path = path.clone();
    let load_task = tokio::task::spawn_blocking(move || {
        file_io::read_file(&load_path).map(|bytes| kiln_bin::decode_loaded_file(load_path, &bytes))
    });
    match runtime.block_on(load_task) {
        Ok(Ok(file)) => {
            session = EditorSession::from_loaded(file);
        }
        Ok(Err(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            load_result = Some("New file".to_string());
        }
        Ok(Err(error)) => return Err(std::io::Error::other(format!("Open failed: {error}")).into()),
        Err(error) => return Err(std::io::Error::other(error.to_string()).into()),
    }
    if let Some(message) = load_result {
        eprintln!("{message}");
    }

    let mut theme = Theme::dark();
    if let Some(theme_path) = session.config().theme.file.as_ref() {
        let theme_path = PathBuf::from(theme_path);
        let theme_path = if theme_path.is_absolute() {
            theme_path
        } else {
            config_path
                .as_ref()
                .and_then(|path| path.parent())
                .map_or(theme_path.clone(), |parent| parent.join(theme_path))
        };
        let path_for_worker = theme_path.clone();
        match runtime.block_on(tokio::task::spawn_blocking(move || {
            kiln_config::load_theme_file(path_for_worker)
        })) {
            Ok(Ok(theme_file)) => apply_theme_file(&mut theme, &theme_file),
            Ok(Err(diagnostic)) => config_warnings.push(diagnostic.to_string()),
            Err(error) => config_warnings.push(format!("could not load theme: {error}")),
        }
    }
    for warning in config_warnings {
        eprintln!("{warning}");
    }
    let _guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(stdout());
    let terminal = Terminal::new(backend)?;
    let mut renderer = RatatuiRenderer::new(terminal)?;
    let mut buffer_view = BufferView {
        scrolloff_rows: session.config().editor.scrolloff,
        ..BufferView::default()
    };
    if !session.config().editor.line_numbers {
        buffer_view.gutter = Gutter {
            mode: LineNumberMode::None,
            show_signs: false,
        };
    }
    let mut pending_save: Option<tokio::task::JoinHandle<(SaveRequest, std::io::Result<()>)>> =
        None;

    while !session.exit_requested() {
        let (_, height) = renderer.size();
        let view_height = height.saturating_sub(1);
        renderer.clear();
        buffer_view.render(&mut renderer, session.buffer(), view_height, &theme);

        let head = session.buffer().selections().primary().head;
        let line_col = session
            .buffer()
            .text()
            .char_to_line_col(head)
            .unwrap_or(kiln_core::LineCol::new(0, 0));
        let total_lines = session.buffer().text().len_lines().max(1);
        let percent = if line_col.line == 0 {
            "Top".to_string()
        } else if line_col.line + 1 >= total_lines {
            "Bot".to_string()
        } else {
            format!("{}%", ((line_col.line + 1) * 100) / total_lines)
        };
        let (mode_name, mode_style) = match session.mode() {
            Mode::Normal => ("NORMAL", theme.mode_normal),
            Mode::Insert => ("INSERT", theme.mode_insert),
            Mode::Visual(_) => ("VISUAL", theme.mode_visual),
            Mode::Command => ("COMMAND", theme.mode_command),
            Mode::Search => ("SEARCH", theme.mode_command),
        };
        let path_name = session
            .buffer()
            .path()
            .and_then(|path| path.file_name())
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("[No Name]");
        let encoding = match session.buffer().encoding() {
            kiln_text::Encoding::Utf8 | kiln_text::Encoding::Utf8Bom => "UTF-8",
            kiln_text::Encoding::Utf16Le => "UTF-16LE",
            kiln_text::Encoding::Utf16Be => "UTF-16BE",
            kiln_text::Encoding::Latin1 => "LATIN-1",
        };
        let ending = match session.buffer().line_ending() {
            kiln_text::LineEnding::Lf => "LF",
            kiln_text::LineEnding::Crlf => "CRLF",
            kiln_text::LineEnding::Cr => "CR",
        };
        let status_state = StatusLineState {
            mode_name,
            mode_style,
            buffer_name: path_name,
            is_modified: session.buffer().is_dirty(),
            is_readonly: session.buffer().is_read_only(),
            line: line_col.line + 1,
            col: line_col.col + 1,
            percent: &percent,
            encoding,
            line_ending: ending,
        };
        StatusLine::render(&mut renderer, view_height, &status_state, &theme);
        if session.mode() == Mode::Command {
            renderer.draw_str(
                0,
                view_height,
                &format!(":{}", session.command_line()),
                theme.ui_text,
            );
        } else if let Some(message) = session.message() {
            renderer.draw_str(0, view_height, message, theme.ui_text);
        }
        renderer.flush()?;

        if let Some(task) = pending_save.as_mut() {
            if task.is_finished() {
                let task = pending_save.take();
                if let Some(task) = task {
                    if let Ok((request, result)) = runtime.block_on(task) {
                        session.apply_save_result(&request, result);
                    }
                }
                continue;
            }
        }

        if pending_save.is_none() && event::poll(Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key_event) => {
                    if key_event.modifiers.contains(CKeyModifiers::CONTROL)
                        && (key_event.code == CKeyCode::Char('q')
                            || key_event.code == CKeyCode::Char('c'))
                    {
                        if session.buffer().is_dirty() {
                            eprintln!("Unsaved changes; use :w, :wq, or :q! to quit");
                        } else {
                            break;
                        }
                        continue;
                    }
                    if let Some(key) = crossterm_to_kiln_key(key_event) {
                        match session.handle_key(key) {
                            SessionAction::Save(request) => {
                                let save_request = request.clone();
                                pending_save = Some(tokio::task::spawn_blocking(move || {
                                    let result = kiln_bin::encode_save_request(&save_request)
                                        .and_then(|bytes| {
                                            file_io::save_file(&save_request.path, &bytes)
                                        });
                                    (save_request, result)
                                }));
                            }
                            SessionAction::Exit | SessionAction::Continue => {}
                        }
                    }
                }
                Event::Resize(_, _) => {}
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
    fn cli_parses_headless_and_file_path() {
        let args = vec![
            "kiln".to_string(),
            "--headless".to_string(),
            "file.txt".to_string(),
            "--keys".to_string(),
            "iHi<Esc>:wq<Enter>".to_string(),
        ];
        assert_eq!(
            parse_args(&args).expect("args parse"),
            CliArgs {
                path: Some(PathBuf::from("file.txt")),
                headless: true,
                keys: Some("iHi<Esc>:wq<Enter>".to_string()),
            }
        );
    }

    #[test]
    fn test_crossterm_key_conversion() {
        let ev = event::KeyEvent::new(CKeyCode::Char('k'), CKeyModifiers::NONE);
        let converted = crossterm_to_kiln_key(ev).expect("character key converts");
        assert_eq!(converted.code, KeyCode::Char('k'));
        assert!(converted.modifiers.is_empty());
        let ev_ctrl = event::KeyEvent::new(CKeyCode::Char('c'), CKeyModifiers::CONTROL);
        let converted_ctrl = crossterm_to_kiln_key(ev_ctrl).expect("modified key converts");
        assert_eq!(converted_ctrl.code, KeyCode::Char('c'));
        assert!(converted_ctrl.modifiers.ctrl);
    }
}
