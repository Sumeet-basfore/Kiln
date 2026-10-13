//! Command registry and execution infrastructure for `kiln-core`.

use std::collections::BTreeMap;

use crate::buffer::{Buffer, BufferId};
use crate::error::CoreError;
use crate::event::EventBus;

/// Direction for cursor movement commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveDirection {
    /// Move cursor left by one or more columns.
    Left,
    /// Move cursor right by one or more columns.
    Right,
    /// Move cursor up by one or more lines.
    Up,
    /// Move cursor down by one or more lines.
    Down,
    /// Move cursor to the start of the current line.
    LineStart,
    /// Move cursor to the end of the current line.
    LineEnd,
}

/// Strongly-typed argument passed to a command.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CommandArg {
    /// No argument provided.
    #[default]
    None,
    /// Text argument (e.g. string to insert).
    Text(String),
    /// Repetition count argument.
    Count(usize),
    /// Direction argument for motions and cursor navigation.
    Direction(MoveDirection),
}

/// Execution context provided to a command handler.
pub struct CommandContext<'a> {
    /// The buffer being operated on.
    pub buffer: &'a mut Buffer,
    /// The event bus for publishing state changes.
    pub event_bus: &'a mut EventBus,
    /// The register store for yanking, pasting, and deletions.
    pub registers: &'a mut crate::register::Registers,
}

/// Outcome of executing a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResult {
    /// The buffer affected by the command, if any.
    pub affected_buffer: Option<BufferId>,
    /// Number of operations or repetitions successfully applied.
    pub count_applied: usize,
}

impl Default for CommandResult {
    fn default() -> Self {
        Self {
            affected_buffer: None,
            count_applied: 1,
        }
    }
}

/// Function signature for command handlers.
pub type CommandFn = fn(&mut CommandContext<'_>, &CommandArg) -> Result<CommandResult, CoreError>;

/// Metadata describing a registered command.
#[derive(Clone)]
pub struct CommandMeta {
    /// Canonical command identifier (e.g. "insert_text").
    pub name: &'static str,
    /// Short human-readable description for palette and help.
    pub description: &'static str,
    /// The handler function.
    pub handler: CommandFn,
}

/// Registry mapping command names to handlers and metadata.
#[derive(Default)]
pub struct CommandRegistry {
    commands: BTreeMap<&'static str, CommandMeta>,
}

impl CommandRegistry {
    /// Creates a new, empty command registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            commands: BTreeMap::new(),
        }
    }

    /// Creates a command registry pre-populated with all built-in edit commands.
    #[must_use]
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        crate::commands::register_builtins(&mut registry);
        registry
    }

    /// Registers a command in the registry.
    ///
    /// If a command with the same name already exists, it is replaced.
    pub fn register(&mut self, meta: CommandMeta) {
        self.commands.insert(meta.name, meta);
    }

    /// Looks up metadata for a command by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&CommandMeta> {
        self.commands.get(name)
    }

    /// Executes a registered command by name with the given context and argument.
    ///
    /// # Errors
    /// Returns `CoreError::CommandNotFound` if no command matches `name`, or any
    /// error returned by the command handler itself.
    pub fn execute(
        &self,
        name: &str,
        ctx: &mut CommandContext<'_>,
        arg: &CommandArg,
    ) -> Result<CommandResult, CoreError> {
        let meta = self
            .commands
            .get(name)
            .ok_or_else(|| CoreError::CommandNotFound {
                name: name.to_string(),
            })?;
        (meta.handler)(ctx, arg)
    }

    /// Returns a list of all registered command metadata in sorted order.
    #[must_use]
    pub fn list(&self) -> Vec<&CommandMeta> {
        self.commands.values().collect()
    }
}
