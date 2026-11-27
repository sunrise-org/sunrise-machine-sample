use mod_api::*;

use crate::{DEPOSIT, DURATION, advance, deposit_count};

pub struct Machine {
    pub pipe: BlockKey,
    pub automatic_input: BoolGameRuleKey,
    pub(crate) recipes: std::sync::Arc<crate::processing::ProcessingRecipes>,
}

#[mod_api::sunrise_part]
impl BlockPart for Machine {}

#[mod_api::sunrise_part]
impl BlockEntityPart for Machine {
    fn block_entity_tick(&self, event: &mut BlockEntityTickEvent<'_>) {
        let tick = &mut event.subject;
        let Some((handle, pos, level, empty)) = tick.with_view(|machine| {
            (
                machine.handle(),
                machine.position(),
                machine.level(),
                machine.item_slot(0).is_none(),
            )
        }) else {
            return;
        };

        let mut changed = false;
        if empty && crate::tour::policy::automatic_input(tick.host(), self.automatic_input) {
            changed |= tick
                .transaction(|tick, tx| {
                    let host = tick.host();
                    let source = level
                        .capability(
                            host,
                            &ITEMS,
                            [pos[0] - 1, pos[1], pos[2]],
                            Some(Direction::East),
                        )
                        .ok_or(())?;
                    let resource = source.resource(host, 0).ok_or(())?;
                    if source.extract_at(host, tx, 0, &resource, 1) != Ok(1) {
                        return Err(());
                    }
                    let stack = resource.to_stack(1).ok_or(())?;
                    tick.with_view(|mut machine| {
                        if machine.item_slot(0).is_some()
                            || machine.set_item_slot(0, Some(stack)) != Ok(true)
                        {
                            return Err(());
                        }
                        Ok(())
                    })
                    .ok_or(())?
                })
                .is_ok();
        }

        let Some((input, output, progress)) = tick.with_view(|machine| {
            let progress = match machine.nbt().get(&"progress".into()) {
                Some(Tag::Int(n)) => *n,
                _ => 0,
            };
            (
                machine.item_slot(0).and_then(|stack| stack.to_owned()),
                machine.item_slot(1).and_then(|stack| stack.to_owned()),
                progress,
            )
        }) else {
            return;
        };
        let host = tick.host();
        let duration = input.as_ref().map_or(DURATION, |input| {
            crate::tour::reload::duration(host, input.item())
        });
        let result = input
            .as_ref()
            .and_then(|input| self.recipes.result(host, input));
        let result = result.and_then(|result| {
            let limit = host.max_stack_size(result.item())?;
            if let Some(mut output) = output {
                let count = output.count().checked_add(result.count())?;
                if count > limit || !output.is_stackable_with(&result) {
                    return None;
                }
                output.set_count(count).ok()?;
                Some(output)
            } else {
                (result.count() <= limit).then_some(result)
            }
        });
        let (progress, done) = advance(progress, result.is_some(), duration);
        if let Some(output) = result.filter(|_| done) {
            changed |= tick
                .transaction(|tick, _| -> Result<(), ()> {
                    tick.with_view(|mut machine| -> Result<(), ()> {
                        machine
                            .set_item_slot(0, input.and_then(|stack| stack.shrink(1)))
                            .map_err(|_| ())?;
                        machine.set_item_slot(1, Some(output)).map_err(|_| ())?;
                        let energy =
                            crate::tour::energy::charged(machine.nbt().get(&"energy".into()));
                        machine.set_nbt("energy".into(), Some(energy));
                        machine.set_nbt("progress".into(), Some(Tag::Int(progress)));
                        machine.set_data_slots(&[progress, duration]);
                        Ok(())
                    })
                    .ok_or(())?
                })
                .is_ok();
        } else {
            // Progress alone marks nothing changed: vanilla's furnace calls `setChanged` only
            // when it finishes an item, burns fuel or lets progress decay
            // (`AbstractFurnaceBlockEntity.serverTick`), not for each cook-timer step.
            tick.with_view(|mut machine| {
                machine.set_nbt("progress".into(), Some(Tag::Int(progress)));
                machine.set_data_slots(&[progress, duration]);
            });
        }

        if let Some(Some(output)) =
            tick.with_view(|machine| machine.item_slot(1).and_then(|stack| stack.to_owned()))
        {
            changed |= tick
                .transaction(|tick, tx| {
                    let host = tick.host();
                    if level.block(host, [pos[0] + 1, pos[1], pos[2]]) != Some(self.pipe) {
                        return Err(());
                    }
                    let target = level
                        .capability(
                            host,
                            &ITEMS,
                            [pos[0] + 2, pos[1], pos[2]],
                            Some(Direction::West),
                        )
                        .ok_or(())?;
                    if target.insert(host, tx, &ItemResource::of(&output), 1) != Ok(1) {
                        return Err(());
                    }
                    tick.with_view(|mut machine| {
                        let current = machine
                            .item_slot(1)
                            .and_then(|stack| stack.to_owned())
                            .ok_or(())?;
                        if current != output {
                            return Err(());
                        }
                        machine.set_item_slot(1, output.shrink(1)).map_err(|_| ())?;
                        Ok(())
                    })
                    .ok_or(())?
                })
                .is_ok();
        }
        if changed {
            let _ = handle.set_changed(tick.host());
        }
    }
}

/// Slot rules and the Deposit action for the machine's menu.
pub struct MachineMenu;

#[mod_api::sunrise_part]
impl MenuPart for MachineMenu {
    fn menu_slot_rules(&self, _: &dyn Host, event: &mut MenuSlotRulesEvent<'_>) {
        if event.container == MenuContainer::Main && matches!(event.slot, 2..=4) {
            event.may_place = false;
            event.may_pickup = false;
        }
    }

    fn menu_button_valid(&self, _: &dyn Host, event: &mut MenuButtonValidEvent<'_>) -> Flow {
        if event.block == Some(crate::MACHINE) && event.id == DEPOSIT {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn menu_button(&self, event: &mut MenuButtonEvent<'_>) {
        if event.block != Some(crate::MACHINE) || event.context.id != DEPOSIT {
            return;
        }
        // Move from the live cursor and commit the two writes together.
        let subject = &mut event.subject;
        let host = subject.host();
        let Some(menu) = host
            .player(event.context.player)
            .and_then(|player| player.container_menu(host))
        else {
            return;
        };
        let cursor = ItemPlace::PlayerCursor { menu };
        let Some(Some(stack)) = cursor.with_stack(host, |stack| stack.project()) else {
            return;
        };
        let Some(limit) = host.max_stack_size(stack.item()) else {
            return;
        };
        // A refused commit leaves the cursor and machine inventory unchanged.
        let _: Result<(), TransactionError<()>> = subject.transaction(|subject, tx| {
            let count = subject
                .with_view(|mut machine| {
                    let input = match machine.item_slot(0) {
                        Some(view) => Some(view.to_owned()?),
                        None => None,
                    };
                    let input_count = input.as_ref().map_or(0, ItemStack::count);
                    let count = deposit_count(
                        stack.count(),
                        input_count,
                        limit,
                        input
                            .as_ref()
                            .is_none_or(|item| item.is_stackable_with(&stack)),
                    );
                    if count == 0 {
                        return None;
                    }
                    let mut updated = input.unwrap_or_else(|| stack.clone());
                    updated.set_count(input_count + count).ok()?;
                    (machine.set_item_slot(0, Some(updated)) == Ok(true)).then_some(count)
                })
                .flatten()
                .ok_or(())?;
            let took = cursor
                .extract(subject.host(), tx, &ItemResource::of(&stack), count)
                .map_err(|_| ())?;
            if took != count {
                return Err(());
            }
            Ok(())
        });
    }
}

/// Build the presentation for a newly opened machine menu.
pub fn screen(_: &dyn ClientHost, _: MenuScreenOpen<'_>) -> Option<MenuScreen> {
    // The final three backing slots start inactive; the MenuPart rejects their use.
    let layout = MenuScreenLayout {
        width: 176.0,
        height: 166.0,
        slots: vec![
            Some(ScreenSlot {
                position: [44.0, 35.0],
                no_item_icon: None,
            }),
            Some(ScreenSlot {
                position: [116.0, 35.0],
                no_item_icon: None,
            }),
        ],
        inventory_x: 8.0,
        inventory_y: 84.0,
        ..Default::default()
    };
    Some(
        MenuScreen::new(layout)
            .with_draw(|_, frame, draw| {
                crate::tour::reload::draw(draw, frame.origin);
                crate::tour::client::overlay(draw, frame.origin);
                let [x, y] = frame.origin;
                draw.fill([x + 66.0, y + 55.0, 44.0, 16.0], 0xff555555);
                draw.text("Deposit", [x + 68.0, y + 59.0], 0xffffffff, false);
                let duration = frame.data.get(1).copied().unwrap_or(DURATION).max(1);
                let progress = frame
                    .data
                    .first()
                    .copied()
                    .unwrap_or_default()
                    .clamp(0, duration);
                draw.fill([x + 72.0, y + 40.0, 32.0, 6.0], 0xff555555);
                draw.fill(
                    [
                        x + 72.0,
                        y + 40.0,
                        32.0 * progress as f32 / duration as f32,
                        6.0,
                    ],
                    0xff55bb55,
                );
            })
            .with_click(|_, frame, right| {
                (!right
                    && (66.0..110.0).contains(&frame.at[0])
                    && (55.0..71.0).contains(&frame.at[1]))
                .then_some(DEPOSIT)
            }),
    )
}
