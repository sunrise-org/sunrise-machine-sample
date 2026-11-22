//! An operator announcement command using parsed input and server services.
use mod_api::*;

pub fn register(event: &mut RegisterCommandsEvent<'_>) {
    event.commands.push(
        Command::op("machine_sample announce <message:text>", 2).executes(
            |host, source, command| {
                let message = announcement(command, source)?;
                let feedback = if host.messages().broadcast(Text::literal(message)).is_ok() {
                    "Announcement delivered."
                } else {
                    "Announcement could not be delivered."
                };
                // The command service routes feedback for console, functions, command blocks and players.
                let _ = host.commands().feedback(Text::literal(feedback));
                Some(1)
            },
        ),
    );
}

fn announcement(command: &CommandInvocation, source: &CommandSource) -> Option<String> {
    let CommandArgumentValue::String(message) = command.argument("message")? else {
        return None;
    };
    let origin = match source {
        CommandSource::Player { .. } => "player",
        CommandSource::Console { .. } => "console",
        CommandSource::CommandBlock { .. } => "command block",
        CommandSource::CommandBlockMinecart { .. } => "command minecart",
        CommandSource::Function { .. } => "function",
        _ => "server",
    };
    Some(format!("[Machine Sample / {origin}] {message}"))
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
            announcement(&command(Some("Machine ready")), &source),
            Some("[Machine Sample / console] Machine ready".into())
        );
        assert_eq!(announcement(&command(None), &source), None);
        let function = CommandSource::Function {
            id: Identifier::parse("machine_sample:alarm").unwrap(),
            permission: PermissionLevel::All,
        };
        assert_eq!(
            announcement(&command(Some("Ready")), &function),
            Some("[Machine Sample / function] Ready".into())
        );
    }
}
