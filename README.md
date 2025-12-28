# Alya-chan Discord Bot

Discord multipurpose bot yang dibuat dengan Rust dan Twilight.

## Struktur Project

```
src/
├── main.rs                 # Entry point dan bot initialization
├── types/                  # Type definitions dan traits
│   ├── mod.rs
│   ├── context.rs         # BotContext
│   ├── command.rs         # Command trait
│   ├── event.rs           # EventHandler trait
│   ├── component.rs       # ComponentHandler trait
│   └── error.rs           # Error types
├── commands/              # Command implementations
│   ├── mod.rs
│   ├── manager.rs         # CommandManager
│   ├── ping.rs            # Ping command
│   ├── help.rs            # Help command
│   └── userinfo.rs        # User info command
├── events/                # Event handler implementations
│   ├── mod.rs
│   ├── manager.rs         # EventManager
│   ├── ready.rs           # Ready event
│   ├── message_create.rs  # Message create event
│   └── guild_create.rs    # Guild create event
├── components/            # Component handler implementations
│   ├── mod.rs
│   └── manager.rs         # ComponentManager
└── utils/                 # Utility functions
    ├── mod.rs
    ├── format.rs          # String formatting utilities
    └── logger.rs          # Logger setup
```

## Setup

1. Clone repository
2. Copy `.env.example` ke `.env` dan isi `DISCORD_TOKEN`
3. Run bot:
   ```bash
   cargo run
   ```

Optional configuration
----------------------
Required configuration
----------------------

A `config.toml` file in the project root is required and must contain the following required keys:

- `[color].primary` (hex or number)
- `[info].banner` (valid http(s) URL)
- `[emoji]` section with required keys: `pencil`, `info`, `music`, `list`, `folder`, `warn`, `question` (see `emoji.toml` example)

If any of the above are missing or invalid, the application will refuse to start and print an error message explaining which keys are missing or invalid.

Example `config.toml` minimum:

```toml
[color]
primary = "#5865f2"

[info]
banner = "https://i.ibb.co/hrpKCdy/e1da98e96fdfc12635909f99725d971e.png"
```

You should also provide `emoji.toml` (recommended) or the `[emoji]` table inside `config.toml` with the required emoji keys.

## Menambahkan Command Baru

Buat file baru di `src/commands/`, contoh `src/commands/say.rs`:

```rust
use async_trait::async_trait;
use crate::types::{Command, CommandContext, BotResult, BotError};

pub struct SayCommand;

#[async_trait]
impl Command for SayCommand {
    fn name(&self) -> &str {
        "say"
    }
    
    fn description(&self) -> &str {
        "Make the bot say something"
    }
    
    fn category(&self) -> &str {
        "fun"
    }
    
    fn usage(&self) -> &str {
        "!say <message>"
    }
    
    async fn execute(&self, ctx: &CommandContext) -> BotResult<()> {
        if ctx.args.is_empty() {
            return Err(BotError::InvalidArguments(
                "Please provide a message".to_string()
            ));
        }
        
        let message = ctx.args.join(" ");
        
        ctx.bot
            .http
            .create_message(ctx.message.channel_id)
            .content(&message)
            .await?;
        
        Ok(())
    }
}
```

Tambahkan ke `src/commands/mod.rs`:
```rust
pub mod say;
```

Register di `src/main.rs`:
```rust
use commands::say::SayCommand;
// ...
command_manager.register(Arc::new(SayCommand));
```

## Menambahkan Event Handler Baru

Buat file baru di `src/events/`, contoh `src/events/member_join.rs`:

```rust
use async_trait::async_trait;
use crate::types::{EventHandler, EventContext, BotResult};
use twilight_model::gateway::event::Event;

pub struct MemberJoinHandler;

#[async_trait]
impl EventHandler for MemberJoinHandler {
    fn name(&self) -> &str {
        "member_join"
    }
    
    async fn handle(&self, ctx: &EventContext) -> BotResult<()> {
        if let Event::MemberAdd(member) = &ctx.event {
            tracing::info!(
                "New member joined: {}#{}",
                member.user.name,
                member.user.discriminator
            );
            
            // Send welcome message, dll
        }
        
        Ok(())
    }
}
```

Tambahkan ke `src/events/mod.rs` dan register di `src/main.rs`.

## Menambahkan Component Handler

Untuk button, select menu, dll:

```rust
use async_trait::async_trait;
use crate::types::{ComponentHandler, ComponentContext, BotResult};

pub struct RoleButtonHandler;

#[async_trait]
impl ComponentHandler for RoleButtonHandler {
    fn custom_id_pattern(&self) -> &str {
        "role_*"  // Matches role_admin, role_mod, dll
    }
    
    async fn handle(&self, ctx: &ComponentContext) -> BotResult<()> {
        // Handle button click
        Ok(())
    }
}
```

## Features

- ✅ Command system dengan trait-based architecture
- ✅ Event handling system
- ✅ Component handling (buttons, select menus)
- ✅ Cache system
- ✅ Error handling
- ✅ Logging
- ✅ Extensible dan mudah di-maintain

## Dependencies

- `twilight-gateway` - Gateway connection
- `twilight-http` - HTTP API client
- `twilight-model` - Discord models
- `twilight-cache-inmemory` - In-memory cache
- `twilight-util` - Utility functions
- `twilight-standby` - Event waiting
- `tokio` - Async runtime
- `async-trait` - Async traits
- `tracing` - Logging
- `anyhow` - Error handling
