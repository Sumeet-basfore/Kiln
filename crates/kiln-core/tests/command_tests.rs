use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use kiln_core::*;
use kiln_text::Text;

#[test]
fn test_command_registry_registration_and_lookup() {
    let mut registry = CommandRegistry::new();
    assert!(registry.list().is_empty());

    registry.register(CommandMeta {
        name: "test_cmd",
        description: "A test command",
        handler: |_ctx, _arg| Ok(CommandResult::default()),
    });

    assert_eq!(registry.list().len(), 1);
    let meta = registry.get("test_cmd").expect("command should be found");
    assert_eq!(meta.name, "test_cmd");
    assert_eq!(meta.description, "A test command");

    assert!(registry.get("unknown_cmd").is_none());
}

#[test]
fn test_command_registry_unknown_command_returns_error() {
    let registry = CommandRegistry::with_builtins();
    let mut allocator = BufferIdAllocator::new();
    let mut buffer = Buffer::new(allocator.allocate(), Text::from("Hello"));
    let mut event_bus = EventBus::new();
    let mut registers = Registers::new();
    let mut ctx = CommandContext {
        buffer: &mut buffer,
        event_bus: &mut event_bus,
        registers: &mut registers,
    };

    let err = registry
        .execute("non_existent_command", &mut ctx, &CommandArg::None)
        .unwrap_err();

    assert!(matches!(err, CoreError::CommandNotFound { name } if name == "non_existent_command"));
}

#[test]
fn test_event_bus_subscription_and_publishing() {
    let mut bus = EventBus::new();
    let counter = Arc::new(AtomicUsize::new(0));

    let counter_clone = Arc::clone(&counter);
    let _sub1 = bus.subscribe(move |ev| {
        if matches!(ev, EditorEvent::BufferChanged { .. }) {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }
    });

    let counter_clone2 = Arc::clone(&counter);
    let sub2 = bus.subscribe(move |_| {
        counter_clone2.fetch_add(10, Ordering::SeqCst);
    });

    let dummy_tx = Transaction::insert_at(5, 5, "!").unwrap();
    bus.publish(EditorEvent::BufferChanged {
        buffer_id: BufferId::from_raw(1),
        old_version: 1,
        new_version: 2,
        transaction: dummy_tx,
    });

    assert_eq!(counter.load(Ordering::SeqCst), 11);
    assert_eq!(bus.events().len(), 1);

    // Unsubscribe sub2
    assert!(bus.unsubscribe(sub2));
    assert!(!bus.unsubscribe(sub2)); // second unsubscribe returns false

    bus.publish(EditorEvent::BufferSaved {
        buffer_id: BufferId::from_raw(1),
    });

    // sub1 ignored BufferSaved, so counter only increased from sub2 which is gone
    assert_eq!(counter.load(Ordering::SeqCst), 11);
    assert_eq!(bus.events().len(), 2);

    let drained = bus.drain_events();
    assert_eq!(drained.len(), 2);
    assert!(bus.events().is_empty());
}

#[test]
fn test_cmd_insert_text_and_event_emission() {
    let registry = CommandRegistry::with_builtins();
    let mut allocator = BufferIdAllocator::new();
    let mut buffer = Buffer::new(allocator.allocate(), Text::from("Hello"));
    let mut event_bus = EventBus::new();

    // Position cursor at end (pos 5)
    buffer.set_selections(Selections::point(5));

    let mut registers = Registers::new();
    let mut ctx = CommandContext {
        buffer: &mut buffer,
        event_bus: &mut event_bus,
        registers: &mut registers,
    };

    let res = registry
        .execute(
            "insert_text",
            &mut ctx,
            &CommandArg::Text(" World".to_string()),
        )
        .unwrap();

    assert_eq!(res.affected_buffer, Some(buffer.id()));
    assert_eq!(buffer.text().to_string(), "Hello World");
    assert_eq!(buffer.selections().ranges(), &[Range::point(11)]);

    // Check event bus published exactly once
    assert_eq!(event_bus.events().len(), 1);
    match &event_bus.events()[0] {
        EditorEvent::BufferChanged {
            buffer_id,
            old_version,
            new_version,
            ..
        } => {
            assert_eq!(*buffer_id, buffer.id());
            assert_eq!(*old_version, 1);
            assert_eq!(*new_version, 2);
        }
        _ => panic!("Expected BufferChanged event"),
    }
}

#[test]
fn test_cmd_insert_text_multi_cursor_and_selection_replace() {
    let registry = CommandRegistry::with_builtins();
    let mut allocator = BufferIdAllocator::new();
    let mut buffer = Buffer::new(allocator.allocate(), Text::from("foo bar baz"));
    let mut event_bus = EventBus::new();

    // Select "bar" [4..7] and "baz" [8..11]
    let sel = Selections::new(vec![Range::new(4, 7), Range::new(8, 11)], 0).unwrap();
    buffer.set_selections(sel);

    let mut registers = Registers::new();
    let mut ctx = CommandContext {
        buffer: &mut buffer,
        event_bus: &mut event_bus,
        registers: &mut registers,
    };

    registry
        .execute("insert_text", &mut ctx, &CommandArg::Text("X".to_string()))
        .unwrap();

    assert_eq!(buffer.text().to_string(), "foo X X");
    assert_eq!(event_bus.events().len(), 1);
}

#[test]
fn test_cmd_delete_char_and_backspace_boundaries() {
    let registry = CommandRegistry::with_builtins();
    let mut allocator = BufferIdAllocator::new();
    let mut buffer = Buffer::new(allocator.allocate(), Text::from("ABC"));
    let mut event_bus = EventBus::new();

    // 1. delete_char at start: deletes 'A'
    buffer.set_selections(Selections::point(0));
    {
        let mut registers = Registers::new();
        let mut ctx = CommandContext {
            buffer: &mut buffer,
            event_bus: &mut event_bus,
            registers: &mut registers,
        };
        registry
            .execute("delete_char", &mut ctx, &CommandArg::None)
            .unwrap();
    }
    assert_eq!(buffer.text().to_string(), "BC");

    // 2. delete_char beyond end of doc is a no-op and does not panic
    buffer.set_selections(Selections::point(2));
    {
        let mut registers = Registers::new();
        let mut ctx = CommandContext {
            buffer: &mut buffer,
            event_bus: &mut event_bus,
            registers: &mut registers,
        };
        let res = registry
            .execute("delete_char", &mut ctx, &CommandArg::None)
            .unwrap();
        assert_eq!(res.count_applied, 0);
    }
    assert_eq!(buffer.text().to_string(), "BC");

    // 3. backspace at end: deletes 'C'
    {
        let mut registers = Registers::new();
        let mut ctx = CommandContext {
            buffer: &mut buffer,
            event_bus: &mut event_bus,
            registers: &mut registers,
        };
        registry
            .execute("backspace", &mut ctx, &CommandArg::None)
            .unwrap();
    }
    assert_eq!(buffer.text().to_string(), "B");

    // 4. backspace at start (pos 0) is a no-op and does not panic
    buffer.set_selections(Selections::point(0));
    {
        let mut registers = Registers::new();
        let mut ctx = CommandContext {
            buffer: &mut buffer,
            event_bus: &mut event_bus,
            registers: &mut registers,
        };
        let res = registry
            .execute("backspace", &mut ctx, &CommandArg::None)
            .unwrap();
        assert_eq!(res.count_applied, 0);
    }
    assert_eq!(buffer.text().to_string(), "B");
}

#[test]
fn test_cmd_newline_and_move_cursor() {
    let registry = CommandRegistry::with_builtins();
    let mut allocator = BufferIdAllocator::new();
    let mut buffer = Buffer::new(allocator.allocate(), Text::from("Line 1\nLine 2"));
    let mut event_bus = EventBus::new();

    buffer.set_selections(Selections::point(0));

    // Move cursor right by 3
    {
        let mut registers = Registers::new();
        let mut ctx = CommandContext {
            buffer: &mut buffer,
            event_bus: &mut event_bus,
            registers: &mut registers,
        };
        registry
            .execute(
                "move_cursor",
                &mut ctx,
                &CommandArg::Direction(MoveDirection::Right),
            )
            .unwrap();
    }
    assert_eq!(buffer.selections().ranges(), &[Range::point(1)]);

    // Move to LineEnd
    {
        let mut registers = Registers::new();
        let mut ctx = CommandContext {
            buffer: &mut buffer,
            event_bus: &mut event_bus,
            registers: &mut registers,
        };
        registry
            .execute(
                "move_cursor",
                &mut ctx,
                &CommandArg::Direction(MoveDirection::LineEnd),
            )
            .unwrap();
    }
    // "Line 1" has 6 chars before newline; cursor moves to pos 6
    assert_eq!(buffer.selections().ranges(), &[Range::point(6)]);

    // Insert newline
    {
        let mut registers = Registers::new();
        let mut ctx = CommandContext {
            buffer: &mut buffer,
            event_bus: &mut event_bus,
            registers: &mut registers,
        };
        registry
            .execute("newline", &mut ctx, &CommandArg::None)
            .unwrap();
    }
    assert_eq!(buffer.text().to_string(), "Line 1\n\nLine 2");
    assert_eq!(buffer.selections().ranges(), &[Range::point(7)]);

    // Move Down
    {
        let mut registers = Registers::new();
        let mut ctx = CommandContext {
            buffer: &mut buffer,
            event_bus: &mut event_bus,
            registers: &mut registers,
        };
        registry
            .execute(
                "move_cursor",
                &mut ctx,
                &CommandArg::Direction(MoveDirection::Down),
            )
            .unwrap();
    }
    // Moved down to Line 2
    assert_eq!(buffer.selections().ranges(), &[Range::point(8)]);

    // Non-mutating move_cursor must NOT emit BufferChanged events
    let change_events: Vec<_> = event_bus
        .events()
        .iter()
        .filter(|e| matches!(e, EditorEvent::BufferChanged { .. }))
        .collect();
    // Only the 'newline' command produced a change event
    assert_eq!(change_events.len(), 1);
}

#[test]
fn test_cmd_undo_and_redo() {
    let registry = CommandRegistry::with_builtins();
    let mut allocator = BufferIdAllocator::new();
    let mut buffer = Buffer::new(allocator.allocate(), Text::from("Start"));
    let mut event_bus = EventBus::new();

    buffer.set_selections(Selections::point(5));

    // Edit 1: Insert " Edit"
    {
        let mut registers = Registers::new();
        let mut ctx = CommandContext {
            buffer: &mut buffer,
            event_bus: &mut event_bus,
            registers: &mut registers,
        };
        registry
            .execute(
                "insert_text",
                &mut ctx,
                &CommandArg::Text(" Edit".to_string()),
            )
            .unwrap();
    }
    assert_eq!(buffer.text().to_string(), "Start Edit");

    // Execute Undo
    {
        let mut registers = Registers::new();
        let mut ctx = CommandContext {
            buffer: &mut buffer,
            event_bus: &mut event_bus,
            registers: &mut registers,
        };
        registry
            .execute("undo", &mut ctx, &CommandArg::None)
            .unwrap();
    }
    assert_eq!(buffer.text().to_string(), "Start");

    // Execute Redo
    {
        let mut registers = Registers::new();
        let mut ctx = CommandContext {
            buffer: &mut buffer,
            event_bus: &mut event_bus,
            registers: &mut registers,
        };
        registry
            .execute("redo", &mut ctx, &CommandArg::None)
            .unwrap();
    }
    assert_eq!(buffer.text().to_string(), "Start Edit");

    // Redo again when at leaf is a no-op
    {
        let mut registers = Registers::new();
        let mut ctx = CommandContext {
            buffer: &mut buffer,
            event_bus: &mut event_bus,
            registers: &mut registers,
        };
        let res = registry
            .execute("redo", &mut ctx, &CommandArg::None)
            .unwrap();
        assert_eq!(res.count_applied, 0);
    }
}
