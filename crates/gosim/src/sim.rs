use flecs_ecs::prelude::*;

pub struct SimTimings {
    /// Time elapsed since last update
    pub dt: Decimal,
}
