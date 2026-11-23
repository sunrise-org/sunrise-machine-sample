//! An operator announcement command using parsed input and server services.
use mod_api::*;

pub fn register(event: &mut RegisterCommandsEvent<'_>) {
    event.commands.push(
        Command::op("machine_sample announce <message:text>", 2).executes(
            |host, source, command| {
                let message = announcement(command, source)?;
                host.messages()
                    .broadcast(Text::literal(message))
                    .map_err(|_| Text::literal("Announcement could not be delivered."))?;
                // The command service routes feedback for console, functions, command blocks and players.
                let _ = host
                    .commands()
                    .feedback(Text::literal("Announcement delivered."));
                Ok(1)
            },
        ),
    );
}

fn announcement(command: &CommandInvocation, source: &CommandSource) -> Result<String, Box<Text>> {
    let Some(CommandArgumentValue::String(message)) = command.argument("message") else {
        return Err(Text::literal("Expected an announcement message.").into());
    };
    let origin = match source {
        CommandSource::Player { .. } => "player",
        CommandSource::Console { .. } => "console",
        CommandSource::CommandBlock { .. } => "command block",
        CommandSource::CommandBlockMinecart { .. } => "command minecart",
        CommandSource::Function { .. } => "function",
        _ => "server",
    };
    Ok(format!("[Machine Sample / {origin}] {message}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(message: Option<&str>) -> CommandInvocation {
        CommandInvocation::new(
            vec!["machine_sample".into(), "announce".into()],
            message
                .map(|message| {
                    CommandArgument::new(
                        "message".into(),
                        CommandArgumentValue::String(message.into()),
                    )
                })
                .into_iter()
                .collect(),
            DimensionKey::OVERWORLD,
            [0.0; 3],
        )
    }

    #[test]
    fn announces_parsed_text_and_typed_origin() {
        let source = CommandSource::Console {
            permission: PermissionLevel::All,
        };
        assert_eq!(
            announcement(&command(Some("Machine ready")), &source).unwrap(),
            "[Machine Sample / console] Machine ready"
        );
        assert!(announcement(&command(None), &source).is_err());
        let function = CommandSource::Function {
            id: Identifier::parse("machine_sample:alarm").unwrap(),
            permission: PermissionLevel::All,
        };
        assert_eq!(
            announcement(&command(Some("Ready")), &function).unwrap(),
            "[Machine Sample / function] Ready"
        );
    }
}
