//! A saved machine tank, filled or drained with the player's held bucket.
use mod_api::resource_util::move_resources;
use mod_api::*;

pub fn tanks() -> ModFluidTanks {
    let mut tanks = ModFluidTanks::default();
    tanks.tanks = vec![4_000];
    tanks.faces = std::array::from_fn(|_| vec![0]);
    tanks.inputs = vec![0];
    tanks.outputs = vec![0];
    tanks
}

pub fn register(event: &mut RegisterCommandsEvent<'_>, machine: Option<BlockKey>) {
    for (verb, fill) in [("fill", true), ("drain", false)] {
        event.commands.push(
            Command::op(&format!("machine_sample {verb} <tank:block_pos>"), 2).executes(
                move |host, source, command| transfer(host, source, command, machine, fill),
            ),
        );
    }
}

fn transfer(
    host: &mut dyn ServerHost,
    source: &CommandSource,
    command: &CommandInvocation,
    machine: Option<BlockKey>,
    fill: bool,
) -> Result<i32, Box<Text>> {
    let CommandSource::Player { player, .. } = source else {
        return Err(Text::literal("Hold a bucket and run this as a player.").into());
    };
    let Some(CommandArgumentValue::BlockPos(pos)) = command.argument("tank") else {
        return Err(Text::literal("Expected the position of a sample machine.").into());
    };
    let Some(level) = player.entity().level(host) else {
        return Err(Text::literal("The player is no longer in a level.").into());
    };
    if !level
        .block(host, *pos)
        .is_some_and(|block| Some(block) == machine)
    {
        return Err(Text::literal("Target a loaded sample machine.").into());
    }
    let tank = level.capability(host, &FLUIDS, *pos, None);
    let held = ItemPlace::PlayerHand {
        player: player.uuid(),
        hand: Hand::Main,
    };
    let bucket = held.capability(host, &ITEM_FLUIDS);
    let (Some(tank), Some(bucket)) = (tank, bucket) else {
        return Err(Text::literal("No fluid handler in the machine or held item.").into());
    };
    let (from, to) = if fill {
        (&bucket, &tank)
    } else {
        (&tank, &bucket)
    };
    let amount = host
        .transaction(|host, tx| move_resources(host, from, to, |_| true, 1_000, Some(tx)))
        .map_err(|_| Text::literal("Transfer refused; no fluid moved."))?;
    let _ = host
        .commands()
        .feedback(Text::literal(format!("Transferred {amount} mB.")));
    Ok(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tank_has_four_bucket_capacity_and_all_faces() {
        let tank = tanks();
        assert_eq!(tank.tanks, [4_000]);
        assert!(tank.faces.iter().all(|face| face == &[0]));
        assert_eq!(tank.inputs, [0]);
        assert_eq!(tank.outputs, [0]);
    }
}
