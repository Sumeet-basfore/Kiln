//! Shared editor session and headless driver.

use std::io;
use std::path::PathBuf;

use kiln_core::{
    Buffer, BufferIdAllocator, CommandArg, CommandContext, CommandRegistry, EditorEvent, EventBus,
    Registers, Text,
};
use kiln_input::{InputEngine, InputResult, KeyCode, KeyEvent, KeymapAction, Mode, VisualKind};
use kiln_services::file_io;
use kiln_text::{
    denormalize_line_endings, detect_and_decode, encode, is_binary, Encoding, LineEnding,
};
use kiln_ui::{Color, Modifier, Style, Theme};

/// Applies configured keybinding overrides and returns warnings for rejected entries.
pub fn apply_keybinding_overrides(
    input: &mut InputEngine,
    config: &kiln_config::KeybindingsConfig,
    registry: &CommandRegistry,
) -> Vec<String> {
    let mut warnings = Vec::new();
    for (mode_name, bindings) in &config.modes {
        let mode = match mode_name.as_str() {
            "normal" => Mode::Normal,
            "insert" => Mode::Insert,
            "visual" => Mode::Visual(VisualKind::Char),
            "visual_line" => Mode::Visual(VisualKind::Line),
            "command" => Mode::Command,
            "search" => Mode::Search,
            _ => {
                warnings.push(format!("unknown keybinding mode `{mode_name}`"));
                continue;
            }
        };
        for (notation, command) in bindings {
            let Some(keys) = parse_config_key_sequence(notation) else {
                warnings.push(format!("invalid key notation `{notation}`"));
                continue;
            };
            if registry.get(command).is_none() {
                warnings.push(format!("unknown command `{command}` for key `{notation}`"));
                continue;
            }
            input.keymaps_mut().get_trie_mut(mode).insert(
                &keys,
                KeymapAction::Command {
                    name: match command.as_str() {
                        "undo" => "undo",
                        "redo" => "redo",
                        "vim_paste_after" => "vim_paste_after",
                        "vim_paste_before" => "vim_paste_before",
                        "vim_open_below" => "vim_open_below",
                        "vim_open_above" => "vim_open_above",
                        "vim_append" => "vim_append",
                        "vim_append_line_end" => "vim_append_line_end",
                        "vim_insert" => "vim_insert",
                        "vim_insert_line_start" => "vim_insert_line_start",
                        "vim_join_lines" => "vim_join_lines",
                        _ => {
                            warnings.push(format!("command `{command}` cannot be bound by config"));
                            continue;
                        }
                    },
                    arg: CommandArg::None,
                },
            );
        }
    }
    warnings
}

/// Applies a parsed custom theme over a built-in theme's unspecified styles.
pub fn apply_theme_file(theme: &mut Theme, file: &kiln_config::ThemeFile) {
    theme.name.clone_from(&file.name);
    for (scope, style) in &file.scopes {
        let mut resolved = Style::default();
        if let Some(color) = &style.foreground {
            resolved = resolved.fg(convert_theme_color(color));
        }
        if let Some(color) = &style.background {
            resolved = resolved.bg(convert_theme_color(color));
        }
        for modifier in &style.modifiers {
            resolved = match modifier.as_str() {
                "bold" => resolved.add_modifier(Modifier::BOLD),
                "italic" => resolved.add_modifier(Modifier::ITALIC),
                "underline" => resolved.add_modifier(Modifier::UNDERLINED),
                "reverse" => resolved.add_modifier(Modifier::REVERSED),
                _ => resolved,
            };
        }
        match scope.as_str() {
            "ui.text" => theme.ui_text = resolved,
            "ui.background" => theme.ui_background = resolved,
            "ui.cursor" => theme.ui_cursor = resolved,
            "ui.selection" => theme.ui_selection = resolved,
            "ui.gutter" => theme.ui_gutter = resolved,
            "ui.gutter.current" => theme.ui_gutter_current = resolved,
            "ui.statusline" => theme.ui_statusline = resolved,
            "ui.statusline.inactive" => theme.ui_statusline_nc = resolved,
            "mode.normal" => theme.mode_normal = resolved,
            "mode.insert" => theme.mode_insert = resolved,
            "mode.visual" => theme.mode_visual = resolved,
            "mode.command" => theme.mode_command = resolved,
            "status.modified" => theme.status_modified = resolved,
            "status.readonly" => theme.status_readonly = resolved,
            _ => {}
        }
    }
}

fn convert_theme_color(color: &kiln_config::ThemeColor) -> Color {
    match color {
        kiln_config::ThemeColor::Rgb([red, green, blue]) => Color::Rgb(*red, *green, *blue),
        kiln_config::ThemeColor::Name(name) => match name.as_str() {
            "black" => Color::Black,
            "red" => Color::Red,
            "green" => Color::Green,
            "yellow" => Color::Yellow,
            "blue" => Color::Blue,
            "magenta" => Color::Magenta,
            "cyan" => Color::Cyan,
            "white" => Color::White,
            "gray" | "grey" => Color::Gray,
            "dark_gray" | "dark_grey" => Color::DarkGray,
            "light_red" => Color::LightRed,
            "light_green" => Color::LightGreen,
            "light_yellow" => Color::LightYellow,
            "light_blue" => Color::LightBlue,
            "light_magenta" => Color::LightMagenta,
            "light_cyan" => Color::LightCyan,
            "reset" => Color::Reset,
            _ => Color::Reset,
        },
    }
}

/// A decoded file and the metadata required to preserve its original format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedFile {
    /// Canonical path used to load the file.
    pub path: PathBuf,
    /// Decoded text normalized to LF.
    pub text: Text,
    /// Detected source encoding.
    pub encoding: Encoding,
    /// Detected dominant line ending.
    pub line_ending: LineEnding,
    /// Whether content looked binary and must remain read-only.
    pub is_binary: bool,
    /// Whether decoding replaced invalid data.
    pub is_lossy: bool,
}

/// Decodes raw bytes and attaches file metadata.
pub fn decode_loaded_file(path: PathBuf, bytes: &[u8]) -> LoadedFile {
    let is_binary = is_binary(bytes);
    let decoded = detect_and_decode(bytes);
    LoadedFile {
        path: std::fs::canonicalize(&path).unwrap_or(path),
        text: Text::from(decoded.text),
        encoding: decoded.encoding,
        line_ending: decoded.line_ending.dominant,
        is_binary,
        is_lossy: decoded.is_lossy,
    }
}

/// Encodes a text snapshot using the buffer's source format.
pub fn encode_save_request(request: &SaveRequest) -> io::Result<Vec<u8>> {
    let normalized = denormalize_line_endings(&request.text.to_string(), request.line_ending);
    let (bytes, lossy) = encode(&normalized, request.encoding);
    if lossy {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "save would lose characters in {:?} encoding",
                request.encoding
            ),
        ));
    }
    Ok(bytes)
}

/// A file snapshot requested by a `:w` or `:wq` command.
#[derive(Debug, Clone)]
pub struct SaveRequest {
    /// Destination path.
    pub path: PathBuf,
    /// Text snapshot to encode and write.
    pub text: Text,
    /// Encoding to preserve.
    pub encoding: Encoding,
    /// Line-ending style to preserve.
    pub line_ending: LineEnding,
    /// Whether a successful save should end the session.
    pub quit_after_save: bool,
    version: u64,
}

/// Result of applying one terminal-independent key.
#[derive(Debug, Clone)]
pub enum SessionAction {
    /// No application-level action is required.
    Continue,
    /// Start this save asynchronously and report its completion to the session.
    Save(SaveRequest),
    /// Exit without saving.
    Exit,
}

/// A single-buffer editor session shared by terminal and headless execution.
pub struct EditorSession {
    buffer: Buffer,
    event_bus: EventBus,
    registry: CommandRegistry,
    registers: Registers,
    input: InputEngine,
    message: Option<String>,
    exit_requested: bool,
    config: kiln_config::Config,
}

impl EditorSession {
    /// Creates an empty, clean session associated with an optional path.
    #[must_use]
    pub fn empty(path: Option<PathBuf>) -> Self {
        let mut allocator = BufferIdAllocator::new();
        let mut buffer = Buffer::new(allocator.allocate(), Text::new());
        if let Some(path) = path {
            buffer = buffer.with_path(path);
        }
        let mut session = Self {
            buffer,
            event_bus: EventBus::new(),
            registry: CommandRegistry::with_builtins(),
            registers: Registers::new(),
            input: InputEngine::new(),
            message: None,
            exit_requested: false,
            config: kiln_config::Config::default(),
        };
        let _ = session.buffer.mark_saved();
        session
    }

    /// Creates a session from decoded file contents.
    #[must_use]
    pub fn from_loaded(file: LoadedFile) -> Self {
        let mut allocator = BufferIdAllocator::new();
        let mut buffer = Buffer::new(allocator.allocate(), file.text)
            .with_path(file.path)
            .with_encoding(file.encoding)
            .with_line_ending(file.line_ending);
        buffer.set_read_only(file.is_binary);
        let _ = buffer.mark_saved();
        let mut session = Self::empty(None);
        session.buffer = buffer;
        if file.is_binary {
            session.message = Some("Binary file opened read-only".to_string());
        } else if file.is_lossy {
            session.message =
                Some("Invalid text sequences were replaced while decoding".to_string());
        }
        session
    }

    /// Applies global config and returns warnings for invalid keybindings.
    pub fn apply_config(&mut self, loaded: kiln_config::LoadedConfig) -> Vec<String> {
        self.config = loaded.config;
        let mut warnings: Vec<String> =
            loaded.diagnostics.iter().map(ToString::to_string).collect();
        warnings.extend(apply_keybinding_overrides(
            &mut self.input,
            &self.config.keybindings,
            &self.registry,
        ));
        warnings
    }

    /// Returns the effective global configuration.
    #[must_use]
    pub fn config(&self) -> &kiln_config::Config {
        &self.config
    }

    /// Returns the configured keybinding action for one mode and key.
    #[must_use]
    pub fn keybinding(&self, mode: Mode, key: KeyEvent) -> Option<KeymapAction> {
        match self.input.keymaps().get_trie(mode).lookup(&[key]) {
            kiln_input::KeymapLookup::Exact(action) => Some(action.clone()),
            kiln_input::KeymapLookup::Partial | kiln_input::KeymapLookup::None => None,
        }
    }

    /// Returns the active buffer.
    #[must_use]
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// Returns the active input mode.
    #[must_use]
    pub fn mode(&self) -> Mode {
        self.input.mode()
    }

    /// Returns the command line currently being edited.
    #[must_use]
    pub fn command_line(&self) -> &str {
        self.input.command_line()
    }

    /// Returns the most recent user-facing message, if any.
    #[must_use]
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// Returns whether the session has requested exit.
    #[must_use]
    pub fn exit_requested(&self) -> bool {
        self.exit_requested
    }

    /// Applies one key event through the shared input and core command path.
    pub fn handle_key(&mut self, key: KeyEvent) -> SessionAction {
        self.message = None;
        match self.input.handle_key(key) {
            InputResult::ExecuteCommand { name, arg } => self.execute_core_command(name, &arg),
            InputResult::InsertChar(character) => {
                self.execute_core_command("insert_text", &CommandArg::Text(character.to_string()))
            }
            InputResult::CommandLineChanged(_) => SessionAction::Continue,
            InputResult::SubmitCommand(command) => self.execute_ex_command(&command),
            InputResult::SwitchMode(_)
            | InputResult::VimAction(_)
            | InputResult::Pending
            | InputResult::Ignored => SessionAction::Continue,
        }
    }

    /// Applies a completed asynchronous load to the session.
    pub fn apply_loaded(&mut self, result: io::Result<LoadedFile>) {
        match result {
            Ok(file) => *self = Self::from_loaded(file),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                self.message = Some("New file".to_string());
                let _ = self.buffer.mark_saved();
            }
            Err(error) => self.message = Some(format!("Open failed: {error}")),
        }
    }

    /// Applies the completion of an asynchronous save.
    pub fn apply_save_result(&mut self, request: &SaveRequest, result: io::Result<()>) -> bool {
        match result {
            Ok(()) => {
                if self.buffer.version() == request.version {
                    if let Err(error) = self.buffer.mark_saved() {
                        self.message = Some(format!(
                            "Save completed but buffer state could not be marked clean: {error}"
                        ));
                        return false;
                    }
                    self.event_bus.publish(EditorEvent::BufferSaved {
                        buffer_id: self.buffer.id(),
                    });
                }
                if request.quit_after_save && !self.buffer.is_dirty() {
                    self.exit_requested = true;
                } else if self.buffer.is_dirty() {
                    self.message = Some("Saved snapshot; newer edits remain unsaved".to_string());
                } else {
                    self.message = Some("File saved".to_string());
                }
                self.exit_requested
            }
            Err(error) => {
                self.message = Some(format!("Save failed: {error}"));
                false
            }
        }
    }

    /// Requests a normal quit, refusing to discard a dirty buffer.
    pub fn request_quit(&mut self) -> SessionAction {
        if self.buffer.is_dirty() {
            self.message = Some("Unsaved changes; use :w, :wq, or :q!".to_string());
            SessionAction::Continue
        } else {
            self.exit_requested = true;
            SessionAction::Exit
        }
    }

    fn execute_core_command(&mut self, name: &'static str, arg: &CommandArg) -> SessionAction {
        let mut context = CommandContext {
            buffer: &mut self.buffer,
            event_bus: &mut self.event_bus,
            registers: &mut self.registers,
        };
        if let Err(error) = self.registry.execute(name, &mut context, arg) {
            self.message = Some(error.to_string());
        }
        SessionAction::Continue
    }

    fn execute_ex_command(&mut self, command: &str) -> SessionAction {
        match command.trim() {
            "w" | "write" => self.save(false),
            "wq" | "writequit" => self.save(true),
            "q" | "quit" => self.request_quit(),
            "q!" | "quit!" => {
                self.exit_requested = true;
                SessionAction::Exit
            }
            "" => SessionAction::Continue,
            other => {
                self.message = Some(format!("Unknown command: :{other}"));
                SessionAction::Continue
            }
        }
    }

    fn save(&mut self, quit_after_save: bool) -> SessionAction {
        if self.buffer.is_read_only() {
            self.message = Some("Cannot save a read-only buffer".to_string());
            return SessionAction::Continue;
        }
        let Some(path) = self.buffer.path().map(PathBuf::from) else {
            self.message = Some("No file name; open a path before saving".to_string());
            return SessionAction::Continue;
        };
        SessionAction::Save(SaveRequest {
            path,
            text: self.buffer.text().clone(),
            encoding: self.buffer.encoding(),
            line_ending: self.buffer.line_ending(),
            quit_after_save,
            version: self.buffer.version(),
        })
    }
}

/// Opens a file and drives the same input/core path without a terminal.
pub fn run_headless(path: PathBuf, script: &str) -> Result<EditorSession, String> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .build()
        .map_err(|error| error.to_string())?
        .block_on(run_headless_async(path, script))
}

async fn run_headless_async(path: PathBuf, script: &str) -> Result<EditorSession, String> {
    let load_path = path.clone();
    let loaded = tokio::task::spawn_blocking(move || file_io::read_file(load_path))
        .await
        .map_err(|error| error.to_string())?;
    let mut session = match loaded {
        Ok(bytes) => EditorSession::from_loaded(decode_loaded_file(path.clone(), &bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => EditorSession::empty(Some(path)),
        Err(error) => return Err(error.to_string()),
    };
    for key in parse_key_script(script)? {
        let action = session.handle_key(key);
        if let SessionAction::Save(request) = action {
            let save_request = request.clone();
            let result = tokio::task::spawn_blocking(move || {
                encode_save_request(&save_request)
                    .and_then(|bytes| file_io::save_file(&save_request.path, &bytes))
            })
            .await
            .map_err(|error| error.to_string())?;
            session.apply_save_result(&request, result);
        }
        if session.exit_requested() {
            break;
        }
    }
    Ok(session)
}

fn parse_config_key_sequence(sequence: &str) -> Option<Vec<KeyEvent>> {
    let mut keys = Vec::new();
    let mut chars = sequence.trim().char_indices().peekable();
    while let Some((start, character)) = chars.next() {
        if character.is_whitespace() {
            continue;
        }
        if character == '<' {
            let remaining = &sequence[start..];
            let (relative_end, _) = remaining
                .char_indices()
                .find(|(_, current)| *current == '>')?;
            let token_end = start + relative_end + 1;
            keys.push(KeyEvent::parse(&sequence[start..token_end])?);
            while chars.peek().is_some_and(|(index, _)| *index < token_end) {
                chars.next();
            }
        } else {
            keys.push(KeyEvent::char(character));
        }
    }
    (!keys.is_empty()).then_some(keys)
}

/// Parses literal characters and `<Key>` tokens into terminal-independent key events.
pub fn parse_key_script(script: &str) -> Result<Vec<KeyEvent>, String> {
    let mut keys = Vec::new();
    let mut chars = script.char_indices().peekable();
    while let Some((start, character)) = chars.next() {
        if character == '<' {
            let Some((end, _)) = script[start..].char_indices().find(|(_, c)| *c == '>') else {
                return Err("unterminated key token in --keys script".to_string());
            };
            let token_end = start + end + 1;
            let token = &script[start..token_end];
            let key = KeyEvent::parse(token)
                .ok_or_else(|| format!("invalid key token {token:?} in --keys script"))?;
            keys.push(key);
            while chars.peek().is_some_and(|(index, _)| *index < token_end) {
                chars.next();
            }
        } else {
            keys.push(KeyEvent::new(KeyCode::Char(character)));
        }
    }
    Ok(keys)
}
