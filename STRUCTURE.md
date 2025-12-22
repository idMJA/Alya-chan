# 🎵 Alya-chan Bot Structure

## Project Organization

```
src/
├── main.rs                 # Bot initialization and event loop
├── types/                  # Type definitions and traits
│   ├── command.rs         # SlashCommand trait & SlashCommandContext
│   ├── context.rs         # BotContext for shared state
│   ├── event.rs           # EventHandler trait & EventContext
│   ├── component.rs       # ComponentHandler trait & ComponentContext
│   ├── error.rs           # BotError & BotResult types
│   └── mod.rs
├── commands/              # Slash commands organized by category
│   ├── utility/           # Utility commands
│   │   ├── ping.rs       # /ping command
│   │   ├── userinfo.rs   # /userinfo command
│   │   ├── help.rs       # /help command (auto-discovery)
│   │   └── mod.rs
│   ├── fun/              # Fun commands (empty, ready for expansion)
│   │   └── mod.rs
│   ├── moderation/       # Moderation commands (empty, ready for expansion)
│   │   └── mod.rs
│   └── mod.rs
├── events/               # Event handlers (not managed, raw handlers)
│   ├── ready.rs         # Bot ready event
│   ├── message_create.rs # Message create event
│   ├── guild_create.rs   # Guild create event
│   └── mod.rs
├── components/          # Component interaction handlers
│   ├── buttons/         # Button handlers
│   │   └── mod.rs
│   ├── modals/          # Modal handlers
│   │   └── mod.rs
│   ├── select_menus/    # Select menu handlers
│   │   └── mod.rs
│   └── mod.rs
├── handlers/            # Managers for various systems
│   ├── command_manager.rs      # Manages slash commands
│   ├── event_manager.rs        # Manages event handlers
│   ├── component_manager.rs    # Manages component interactions
│   └── mod.rs
├── utils/               # Utility functions
│   ├── logger.rs        # Logger initialization
│   ├── format.rs        # Formatting utilities
│   └── mod.rs
└── Cargo.toml          # Dependencies
```

## 📝 How to Add New Commands

Commands are organized **by category (folder name)**. The category is determined automatically from the folder name.

### Example: Adding a "ban" command to moderation

1. **Create file**: `src/commands/moderation/ban.rs`

```rust
use async_trait::async_trait;
use crate::types::{SlashCommand, SlashCommandContext, BotResult};

pub struct BanCommand;

#[async_trait]
impl SlashCommand for BanCommand {
    fn name(&self) -> &str {
        "ban"
    }
    
    fn description(&self) -> &str {
        "Ban a member from the server"
    }
    
    fn category(&self) -> &str {
        "moderation"
    }
    
    async fn execute(&self, ctx: &SlashCommandContext) -> BotResult<()> {
        // Implementation here
        Ok(())
    }
}
```

2. **Export in** `src/commands/moderation/mod.rs`:

```rust
pub mod ban;
pub use ban::BanCommand;
```

3. **Update** `src/main.rs` to register the command:

```rust
use commands::moderation::BanCommand;

// In main()
cmd_mgr.register(Arc::new(BanCommand));
```

## 🎛️ Component Handlers

Components are organized by type:

- **buttons/** - Button click handlers
- **modals/** - Modal submission handlers  
- **select_menus/** - Select menu handlers

### Example: Adding a button handler

1. **Create file**: `src/components/buttons/confirm_button.rs`

2. **Export in** `src/components/buttons/mod.rs`

3. **Register in** main.rs via ComponentManager

## 🔧 Managers (in handlers/)

- **CommandManager**: Registers and routes slash commands by name/category
- **EventManager**: Registers and dispatches Discord events
- **ComponentManager**: Routes component interactions based on custom_id patterns

---

**Key Benefits:**
✅ Commands organized by semantic category (utility, fun, moderation)
✅ Components organized by type (buttons, modals, select_menus)
✅ Managers centralized in handlers folder for clean architecture
✅ Easy to extend - just add new folders/files, update mod.rs exports
✅ Auto-discovery of commands in help system
