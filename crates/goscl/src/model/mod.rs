use flecs_ecs::prelude::*;
// use gosim::WaterfrontModule;

pub struct GosClientModel {
    world: World,
}

impl GosClientModel {
    pub fn new() -> Self {
        let world = World::new();

        // world.import::<WaterfrontModule>();

        Self { world }
    }

    pub fn on_tick(&mut self) {
        self.world.progress();
    }

    pub fn world(&self) -> &World {
        &self.world
    }
}
