use proptest::prelude::*;

use kiln_core::*;
use kiln_text::Text;

proptest! {
    #[test]
    fn prop_fuzz_random_command_sequences(
        initial in "[a-zA-Z0-9 \n]{0,20}",
        commands in prop::collection::vec(0..=6u8, 1..=40)
    ) {
        let registry = CommandRegistry::with_builtins();
        let mut allocator = BufferIdAllocator::new();
        let mut buffer = Buffer::new(allocator.allocate(), Text::from(initial.as_str()));
        let mut event_bus = EventBus::new();
        let mut registers = Registers::new();

        for cmd_code in commands {
            let mut ctx = CommandContext {
                buffer: &mut buffer,
                event_bus: &mut event_bus,
                registers: &mut registers,
            };

            let res = match cmd_code {
                0 => registry.execute("insert_text", &mut ctx, &CommandArg::Text("A".to_string())),
                1 => registry.execute("delete_char", &mut ctx, &CommandArg::None),
                2 => registry.execute("backspace", &mut ctx, &CommandArg::None),
                3 => registry.execute("newline", &mut ctx, &CommandArg::None),
                4 => registry.execute("move_cursor", &mut ctx, &CommandArg::Direction(MoveDirection::Left)),
                5 => registry.execute("move_cursor", &mut ctx, &CommandArg::Direction(MoveDirection::Right)),
                6 => registry.execute("undo", &mut ctx, &CommandArg::None),
                _ => Ok(CommandResult::default()),
            };

            // Invariant 1: Commands never panic and always return typed Result
            prop_assert!(res.is_ok());

            // Invariant 2: Selections always remain valid character offsets within 0..=len
            let len = buffer.text().len_chars();
            for r in buffer.selections().ranges() {
                prop_assert!(r.anchor <= len);
                prop_assert!(r.head <= len);
            }
        }
    }
}
