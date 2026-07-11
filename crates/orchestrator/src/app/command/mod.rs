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

mod view;

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
pub use view::CommandView;

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

/// Resolve a launch item's command reference into a concrete spawn command
/// (launch-execution spec: "an item's command is resolved before launch"). A
/// library reference reads the stored cli/args/env; inline is used as given.
/// Shared by `LaunchSession` and `SpawnSurface` so both resolve identically.
pub(crate) async fn resolve(
    cx: &crate::context::Ctx,
    cmd_ref: &crate::entities::launch_spec::CommandRef,
) -> crate::shared::Result<crate::infra::daemon_pty_api::SpawnCommand> {
    use crate::entities::command::CommandId;
    use crate::entities::launch_spec::CommandRef;
    use crate::infra::daemon_pty_api::SpawnCommand;
    use crate::infra::CommandRepo;
    use crate::shared::Error;

    match cmd_ref {
        CommandRef::LibraryRef { library_ref } => {
            let cmd = CommandRepo::get(cx.db(), &CommandId::from_string(library_ref.clone()))
                .await?
                .ok_or_else(|| Error::CommandNotFound(library_ref.clone()))?;
            Ok(SpawnCommand {
                exe: cmd.cli,
                args: cmd.args,
                env: cmd.env.into_iter().collect(),
            })
        }
        CommandRef::Inline { executable, args } => Ok(SpawnCommand {
            exe: executable.clone(),
            args: args.clone(),
            env: Default::default(),
        }),
    }
}

#[cfg(test)]
mod resolve_tests {
    use super::resolve;
    use crate::entities::command::{Command, CommandId, CommandOrigin};
    use crate::entities::launch_spec::CommandRef;
    use crate::infra::CommandRepo;
    use crate::shared::Error;

    // Scenario: Library reference resolves
    #[tokio::test]
    async fn library_reference_resolves_to_the_stored_cli_args_env() {
        let cx = crate::boot::test_ctx().await.unwrap();
        let mut env = std::collections::HashMap::new();
        env.insert("FOO".to_owned(), "bar".to_owned());
        let cmd = Command {
            id: CommandId::mint(),
            name: "htop".to_owned(),
            origin: CommandOrigin::Custom,
            cli: "/usr/bin/htop".to_owned(),
            args: vec!["--sort-key".to_owned(), "cpu".to_owned()],
            env,
            pinned: false,
        };
        CommandRepo::create(cx.db(), &cmd).await.unwrap();

        let resolved = resolve(
            &cx,
            &CommandRef::LibraryRef {
                library_ref: cmd.id.as_str().to_owned(),
            },
        )
        .await
        .unwrap();

        assert_eq!(resolved.exe, "/usr/bin/htop");
        assert_eq!(
            resolved.args,
            vec!["--sort-key".to_owned(), "cpu".to_owned()]
        );
        assert_eq!(resolved.env.get("FOO"), Some(&"bar".to_owned()));
    }

    // Scenario: Inline command is used as given
    #[tokio::test]
    async fn inline_command_is_used_as_given() {
        let cx = crate::boot::test_ctx().await.unwrap();

        let resolved = resolve(
            &cx,
            &CommandRef::Inline {
                executable: "/bin/sh".to_owned(),
                args: vec!["-c".to_owned(), "echo hi".to_owned()],
            },
        )
        .await
        .unwrap();

        assert_eq!(resolved.exe, "/bin/sh");
        assert_eq!(resolved.args, vec!["-c".to_owned(), "echo hi".to_owned()]);
        assert!(resolved.env.is_empty());
    }

    // Scenario: Unknown reference fails the item
    #[tokio::test]
    async fn unknown_library_reference_is_a_typed_not_found_error() {
        let cx = crate::boot::test_ctx().await.unwrap();

        let result = resolve(
            &cx,
            &CommandRef::LibraryRef {
                library_ref: "does-not-exist".to_owned(),
            },
        )
        .await;

        assert!(matches!(result, Err(Error::CommandNotFound(_))));
    }
}
