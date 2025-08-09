use crate::{create_human, AgentMocca};
use flecs_ecs::prelude::World;
use mocca::{Mocca, MoccaDeps};

pub struct GosSimMocca;

impl Mocca for GosSimMocca {
    fn load(mut dep: MoccaDeps) {
        dep.dep::<AgentMocca>();
    }

    fn start(world: &World) -> Self {
        create_human(world.entity_named("Bob"));

        Self
    }
}
