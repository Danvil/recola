use crate::ecs::prelude::*;

#[derive(Component)]
pub struct HungerModule;

/// Satiation
#[derive(Component, Debug, Clone)]
pub struct Satiation {
    /// First level of satiation. If empty hunger debuffs apply and secondary level empties.
    pub primary: f64,

    /// Second level of satiation. If empty the agent starves to death. Will recharge from primary
    /// level if possible.
    pub secondary: f64,
}

/// Eats some food
#[derive(Component, Debug, Clone)]
pub struct EatOp {
    pub food_value: f64,
}

const SECONDARY_TO_PRIMARY_SATIATION_CONVERSION_RATE: f64 = 2.0;
const SECONDARY_SATIATION_RECHARGE: f64 = 0.1;
const HUNGER_RATE: f64 = 0.1;

impl Module for HungerModule {
    fn module(world: &World) {
        world.module::<HungerModule>("hunger");

        world.component::<Satiation>();
        world.component::<EatOp>();

        // Update hunger levels
        world
            .system::<(&mut Satiation,)>()
            .each_entity(|e, (sat,)| {
                if sat.primary < HUNGER_RATE {
                    let missing = HUNGER_RATE - sat.primary;
                    sat.primary = 0.;
                    let sec_sub = missing / SECONDARY_TO_PRIMARY_SATIATION_CONVERSION_RATE;
                    if sat.secondary < sec_sub {
                        sat.secondary = 0.;
                    } else {
                        sat.secondary -= sec_sub;
                    }
                }

                if sat.secondary <= 100. {
                    let missing = (100. - sat.secondary).min(SECONDARY_SATIATION_RECHARGE);
                    let prim_sub_max = missing * SECONDARY_TO_PRIMARY_SATIATION_CONVERSION_RATE;
                    let prim_sub_actual = prim_sub_max.min(sat.primary);
                    sat.primary -= prim_sub_actual;
                    sat.secondary +=
                        prim_sub_actual / SECONDARY_TO_PRIMARY_SATIATION_CONVERSION_RATE;
                }
            });
    }
}
