use crate::{FlowSimMocca, LogMocca, TimeMocca};
use flecs_ecs::prelude::World;
use mocca::{Mocca, MoccaDeps};

pub struct GosSimMocca;

impl Mocca for GosSimMocca {
    fn start(_: &World) -> Self {
        Self
    }

    fn load(mut dep: MoccaDeps) {
        dep.dep::<LogMocca>();
        dep.dep::<TimeMocca>();
        dep.dep::<FlowSimMocca>();
    }
}
