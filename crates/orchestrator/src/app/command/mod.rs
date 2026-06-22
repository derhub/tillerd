//! Command-library CQS operations.
//!
//! Commands mutate and return nothing; queries read and return their `Out`.
//! Prebuilt commands are immutable: `RenameCommand`/`EditCommand`/`DiscardCommand`
//! are rejected by the prebuilt-immutable guard. `DuplicateCommand` produces an
//! editable `Custom` copy of any command, including a `Prebuilt`.
//!
//! `SeedCommands` is idempotent: it upserts the built-in prebuilt commands, leaving
//! custom rows untouched. The migration seeds the same rows; calling it again is a
//! no-op.

pub mod discard_command;
pub mod duplicate_command;
pub mod edit_command;
pub mod get_command_by_id;
pub mod list_commands;
pub mod new_command;
pub mod pin_command;
pub mod rename_command;
pub mod seed_commands;
pub mod unpin_command;

#[cfg(test)]
pub(crate) mod test_util;

pub use discard_command::DiscardCommand;
pub use duplicate_command::DuplicateCommand;
pub use edit_command::EditCommand;
pub use get_command_by_id::GetCommandById;
pub use list_commands::ListCommands;
pub use new_command::NewCommand;
pub use pin_command::PinCommand;
pub use rename_command::RenameCommand;
pub use seed_commands::SeedCommands;
pub use unpin_command::UnpinCommand;

// ── guard ─────────────────────────────────────────────────────────────────────

pub(crate) fn guard_not_prebuilt(
    cmd: &crate::entities::command::Command,
) -> crate::shared::Result<()> {
    use crate::entities::command::CommandOrigin;
    use crate::shared::Error;
    if cmd.origin == CommandOrigin::Prebuilt {
        Err(Error::PrebuiltImmutable { kind: "command" })
    } else {
        Ok(())
    }
}

// ── seed helper ───────────────────────────────────────────────────────────────

pub(crate) async fn seed_prebuilt(cx: &crate::context::Ctx) -> crate::shared::Result<()> {
    // The schema migration seeds login-shell; any prebuilt that should exist
    // post-migration but pre-seed is re-applied here idempotently.
    const PREBUILT: &[(&str, &str, &[&str])] = &[(
        "00000000-0000-0000-0000-000000000101",
        "login-shell",
        &["-l"],
    )];

    for (id, name, args) in PREBUILT {
        let args_json = serde_json::to_string(*args)?;
        sqlx::query(
            "INSERT OR IGNORE INTO command (id, name, origin, cli, args_json)
             VALUES (?, ?, 'prebuilt', '/bin/bash', ?)",
        )
        .bind(id)
        .bind(name)
        .bind(&args_json)
        .execute(cx.db())
        .await?;
    }
    Ok(())
}
