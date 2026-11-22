// Shared by the standalone tests and the native bundle.
mod machine;
mod processing;
mod tour;

use mod_api::*;

#[derive(Default)]
struct Sample {
    recipes: std::sync::Arc<processing::ProcessingRecipes>,
    machine: std::sync::OnceLock<BlockKey>,
    inspect_mapping: std::sync::OnceLock<ModKeyMappingRef>,
    visits: std::sync::RwLock<Option<AttachmentKey<u32>>>,
}

#[mod_api::sunrise_mod]
impl Hooks for Sample {
    fn server_tick_rate_changed(
        &self,
        host: &mut dyn ServerHost,
        _: &mut ServerTickRateChangedEvent,
    ) {
        let _ = host
            .messages()
            .broadcast(Text::literal("Machine Sample: tick-rate update observed."));
    }

    fn server_about_to_start(&self, _: &mut dyn ServerHost, _: &mut ServerAboutToStartEvent) {
        self.recipes.clear();
    }

    fn datapack_sync(&self, host: &mut dyn ServerHost, _: &mut DatapackSyncEvent<'_>) {
        self.recipes.publish(host);
    }

    fn server_stopped(&self, _: &mut dyn ServerHost, _: &mut ServerStoppedEvent) {
        self.recipes.clear();
    }

    fn entity_interact(
        &self,
        host: HostSide<'_>,
        event: &mut EntityInteractEvent<'_>,
    ) -> Option<InteractionResult> {
        // The visit count is the server's: the client predicts nothing here.
        match host {
            HostSide::Server(host) => {
                let visits = self.visits.read().ok()?;
                tour::attachments::inspect(host, event, visits.as_ref()?)
            }
            HostSide::Client(_) => None,
        }
    }

    fn add_server_reload_listeners(
        &self,
        _: &dyn ReloadContext,
        event: &mut AddServerReloadListenersEvent<'_>,
    ) {
        tour::reload::server(event);
    }

    fn add_client_reload_listeners(
        &self,
        _: &dyn ClientReloadListenerContext,
        event: &mut AddClientReloadListenersEvent<'_>,
    ) {
        tour::reload::client(event);
    }

    fn client_setup(&self, host: &mut dyn ClientSetupContext, _: &mut ClientSetupEvent) {
        if let Ok(mapping) = host.mod_key_mapping("machine_sample", "inspect") {
            let _ = self.inspect_mapping.set(mapping);
        }
    }

    fn input_key(&self, host: &mut dyn ClientHost, event: &mut InputKeyEvent<'_>) {
        if let Some(mapping) = self.inspect_mapping.get().copied() {
            tour::client::input(host, event, mapping);
        }
    }

    fn register_commands(&self, _: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        tour::services::register(event);
        tour::fluids::register(event, self.machine.get().copied());
    }

    fn register(&self, registrar: &mut dyn Registrar, _: &RegistrationContext) {
        if let Ok(visits) = tour::attachments::register(registrar) {
            *self.visits.write().unwrap() = Some(visits);
        }
        registrar.register_recipe_type(PROCESSING_RECIPE_TYPE);
        registrar.register_raw_recipe_serializer(PROCESSING_SERIALIZER, PROCESSING_RECIPE_TYPE);
        let machine_block = registrar.block_key(MACHINE);
        registrar.block_container(&machine_block, container());
        registrar.block_fluids(&machine_block, tour::fluids::tanks());
        registrar.block_energy(&machine_block, tour::energy::buffer());
        let _ = self.machine.set(machine_block);
        let pipe = registrar.block_key(PIPE);
        tour::client::register(registrar);
        let automatic_input = tour::policy::register(registrar);
        let machine = registrar
            .block_entity_type("machine")
            .blocks(&[MACHINE])
            .build();
        registrar
            .block_entity_part(
                &machine,
                machine::Machine {
                    pipe,
                    automatic_input,
                    recipes: self.recipes.clone(),
                },
            )
            .expect("native block entity type");
        registrar.menu_part(MACHINE_MENU, machine::MachineMenu);
        registrar
            .register_menu_screen(MACHINE_MENU, std::sync::Arc::new(machine::screen))
            .expect("machine screen factory");
    }
}

fn container() -> ModContainer {
    ModContainer {
        slots: 5,
        faces: std::array::from_fn(|_| vec![0, 1]),
        inputs: vec![0],
        outputs: vec![1],
        insert_rules: Vec::new(),
        menu: Some(MACHINE_MENU.into()),
    }
}

pub const MACHINE: &str = "machine_sample:machine";
/// The machine's own menu type, with five backing slots.
pub const MACHINE_MENU: &str = "machine_sample:machine_menu";
/// The recipe type the machine queries with `recipes_of_type`.
pub const PROCESSING_RECIPE_TYPE: &str = "machine_sample:processing";
/// The `type` of the recipe files, a serializer id independent of the recipe type.
pub const PROCESSING_SERIALIZER: &str = "machine_sample:processing_json";
pub const PIPE: &str = "machine_sample:pipe";
pub const DURATION: i32 = 40;
pub const DEPOSIT: i32 = 0;

pub fn deposit_count(carried: u32, input: u32, limit: u32, compatible: bool) -> u32 {
    if compatible {
        carried.min(limit.saturating_sub(input))
    } else {
        0
    }
}

pub fn advance(progress: i32, can_process: bool, duration: i32) -> (i32, bool) {
    if !can_process {
        return (0, false);
    }
    let next = progress.clamp(0, duration - 1) + 1;
    if next == duration {
        (0, true)
    } else {
        (next, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deposit_respects_capacity_and_compatibility() {
        assert_eq!(deposit_count(0, 0, 64, true), 0);
        assert_eq!(deposit_count(32, 0, 64, true), 32);
        assert_eq!(deposit_count(32, 50, 64, true), 14);
        assert_eq!(deposit_count(32, 64, 64, true), 0);
        assert_eq!(deposit_count(32, 70, 64, true), 0);
        assert_eq!(deposit_count(32, 5, 64, false), 0);
    }
    #[test]
    fn completes_after_forty_ticks_and_resets_when_blocked() {
        let mut progress = 0;
        for _ in 0..39 {
            let (next, done) = advance(progress, true, DURATION);
            assert!(!done);
            progress = next;
        }
        assert_eq!(advance(progress, true, DURATION), (0, true));
        assert_eq!(advance(progress, false, DURATION), (0, false));
        assert_eq!(advance(79, true, 80), (0, true));
        assert_eq!(advance(0, true, 1), (0, true));
    }
}
