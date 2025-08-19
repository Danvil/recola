use crate::ecs::prelude::*;

#[derive(Component, Debug, Clone)]
pub struct BodyTox {
    pub amount: f64,
}
