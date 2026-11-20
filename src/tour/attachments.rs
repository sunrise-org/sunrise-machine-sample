//! Crouch with an empty hand and inspect an entity to record a saved visit.
use mod_api::*;

pub fn register(registrar: &mut dyn Registrar) -> Result<AttachmentKey<u32>, String> {
    registrar.register_attachment_type(
        AttachmentDescriptor::new("machine_sample:inspection_visits", || 0)
            .serialize(AttachmentCodec { encode, decode }),
    )
}

fn encode(visits: &u32) -> Result<Compound, AttachmentError> {
    let mut saved = Compound::default();
    saved.insert("visits".into(), Tag::Long(i64::from(*visits)));
    Ok(saved)
}

fn decode(saved: &Compound) -> Result<u32, AttachmentError> {
    let Some(Tag::Long(visits)) = saved.get(&"visits".into()) else {
        return Err(AttachmentError::Decode);
    };
    u32::try_from(*visits).map_err(|_| AttachmentError::Decode)
}

pub fn inspect(
    host: &mut dyn ServerHost,
    event: &EntityInteractEvent<'_>,
    key: &AttachmentKey<u32>,
) -> Option<InteractionResult> {
    let player = host.players().acting_player()?;
    if player.is_secondary_use_active(host) != Some(true) {
        return None;
    }
    let main = ItemPlace::PlayerHand {
        player: player.uuid(),
        hand: Hand::Main,
    };
    if !inspection_hands_empty(main.amount(host) != 0, event.stack) {
        return None;
    }
    let id = u32::try_from(event.entity).ok()?;
    // The event's target belongs to the host's current level. No entity, world
    // borrow or holder identity is retained beyond this callback.
    let holder = Holder::Entity(id);
    let answer = key.get_existing(host, holder).and_then(|visits| {
        let visits = visits.unwrap_or_default().saturating_add(1);
        key.set(host, holder, visits)?;
        Ok(visits)
    });
    match answer {
        Ok(visits) => {
            let _ = player.action_bar(
                host,
                Text::literal(format!("Inspected {}: visit {visits}", event.kind)),
            );
            // A consuming answer ends the hand loop, so the off hand is not asked;
            // the server swings the hand.
            Some(InteractionResult::SuccessServer)
        }
        Err(AttachmentError::MissingHolder) => None,
        Err(_) => {
            let _ = player.action_bar(
                host,
                Text::literal("Inspection data could not be read; left unchanged."),
            );
            None
        }
    }
}

fn inspection_hands_empty(main_occupied: bool, used: Option<&ItemStack>) -> bool {
    !main_occupied && used.is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mod_api::prelude::ItemsHost;

    #[test]
    fn an_empty_off_hand_does_not_bypass_an_occupied_main_hand() {
        let mut host = mod_api::testing::FakeHost::new();
        let id = Identifier::parse("minecraft:stick").unwrap();
        host.registries.resolve_items([id.clone()]).unwrap();
        let stick = ItemStack::of(host.item_key(&id).unwrap());
        assert!(inspection_hands_empty(false, None));
        assert!(!inspection_hands_empty(true, None));
        assert!(!inspection_hands_empty(false, Some(&stick)));
        assert!(!inspection_hands_empty(true, Some(&stick)));
    }

    #[test]
    fn persisted_visit_count_round_trips_without_truncation() {
        for visits in [0, 1, u32::MAX] {
            assert_eq!(decode(&encode(&visits).unwrap()), Ok(visits));
        }
    }

    #[test]
    fn malformed_saved_count_is_refused() {
        assert_eq!(decode(&Compound::default()), Err(AttachmentError::Decode));
        for value in [
            Tag::Int(1),
            Tag::Long(-1),
            Tag::Long(i64::from(u32::MAX) + 1),
        ] {
            let mut saved = Compound::default();
            saved.insert("visits".into(), value);
            assert_eq!(decode(&saved), Err(AttachmentError::Decode));
        }
    }
}
