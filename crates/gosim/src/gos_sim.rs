use crate::{create_human, ecs::prelude::*, AgentMocca};

pub struct GosSimMocca;

impl Mocca for GosSimMocca {
    fn load(mut dep: MoccaDeps) {
        dep.depends_on::<AgentMocca>();
    }

    fn start(world: &mut World) -> Self {
        create_human(world.spawn_empty().with_name("Bob"));

        Self
    }
}
