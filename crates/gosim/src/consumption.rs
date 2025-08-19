use crate::{ecs::prelude::*, op_mark_completed, OpStatePending};

#[derive(Component)]
pub struct ConsumptionModule;

/// Marker for entities which can be consumed
#[derive(Component)]
pub struct Consumable;

/// Charge
#[derive(Component)]
pub struct Charge {
    pub amount: f64,
}

/// Consumes an entity by transfering charge
#[derive(Component, Debug, Clone)]
pub struct TransferChargeOp {
    /// Entity which provides the charge
    pub source: Entity,

    /// Entity which receives the charge
    pub target: Entity,
}

impl Module for ConsumptionModule {
    fn module(world: &World) {
        world.module::<ConsumptionModule>("consumption");

        world.component::<Consumable>();
        world.component::<Charge>();
        world.component::<TransferChargeOp>();

        // Transfer charge between items
        world
            .system::<(&TransferChargeOp,)>()
            .with(OpStatePending)
            .each_entity(move |e, (op,)| {
                let world = e.world();

                let source = world.entity_from_id(op.source);
                let target = world.entity_from_id(op.target);

                source.get::<&Charge>(|src| {
                    target.get::<&mut Charge>(|dst| {
                        dst.amount += src.amount;
                    })
                });

                op_mark_completed(e);
            });
    }
}
