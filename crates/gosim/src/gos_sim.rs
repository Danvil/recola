use crate::{AgentMocca, create_human, ecs::prelude::*};

pub struct GosSimMocca;

impl Mocca for GosSimMocca {
    fn load(mut dep: MoccaDeps) {
        dep.depends_on::<AgentMocca>();
    }

    fn start(world: &mut World) -> Self {
        let entity = world.spawn((Name::from_str("Bob"),));
        let entity = world.entity(entity).unwrap();
        create_human(entity);

        Self
    }
}
