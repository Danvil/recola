use crate::{op_mark_completed, OpStatePending, OpsModule};
use flecs_ecs::prelude::*;

#[derive(Component)]
pub struct InventoryModule;

/// Marker for individual items which can be stored in a container
#[derive(Component)]
pub struct ItemTag;

// /// Amount of an item stored in a container for countable items. All instances of countable items
// /// are considered identical.
// #[derive(Component)]
// pub struct Amount {
//     pub amount: u32,
// }

/// Marker for containers which can store items
#[derive(Component)]
pub struct ContainerTag;

/// Relation to indicate in which container an item is placed.
#[derive(Component)]
pub struct ContainedBy;

/// Relation to indicate which container an entity uses as inventory
#[derive(Component)]
pub struct HasInventory;

/// Transfers an item between containers
#[derive(Component, Debug, Clone)]
pub struct TransferItemOp {
    /// Item to move to another container
    pub item: Entity,

    /// Container to which the item is moved
    pub container: Entity,
}

/// Removes the container property from an entity and transfers all its items to another container
#[derive(Component, Debug, Clone)]
pub struct DestroyContainerOp {
    /// Container which is destroyed
    pub container: Entity,

    /// Container which receives items
    pub receiver: Entity,
}

/// Removes the item property from an entity and thus also removes it from its container
#[derive(Component)]
pub struct DestroyItemOp {
    /// Item which is destroyed
    pub item: Entity,
}

impl Module for InventoryModule {
    fn module(world: &World) {
        world.module::<InventoryModule>("inventory");

        world.import::<OpsModule>();

        world.component::<ItemTag>();
        world.component::<ContainerTag>();
        world
            .component::<ContainedBy>()
            .add_trait::<flecs::Exclusive>();
        world.component::<HasInventory>();
        world
            .component::<HasInventory>()
            .add_trait::<flecs::Exclusive>();
        world.component::<TransferItemOp>();
        world.component::<DestroyContainerOp>();
        world.component::<DestroyItemOp>();

        // Process operations to transfer items
        world
            .system::<(&TransferItemOp,)>()
            .with(OpStatePending)
            .each_entity(|e, (op,)| {
                let world = e.world();
                let item = world.entity_from_id(op.item);

                item.add((ContainedBy, op.container));

                op_mark_completed(e);
            });

        // Process operations to destroy a container
        world
            .system::<(&DestroyContainerOp,)>()
            .with(OpStatePending)
            .each_entity(move |e, (op,)| {
                let world = e.world();
                let container = world.entity_from_id(op.container);

                world
                    .query::<()>()
                    .with(ItemTag)
                    .with((ContainedBy, op.container))
                    .build()
                    .each_entity(|item, _| {
                        item.add((ContainedBy, op.receiver));
                    });

                container.remove(ContainerTag);

                op_mark_completed(e);
            });

        // Process operations to destroy an item
        world
            .system::<(&DestroyItemOp,)>()
            .with(OpStatePending)
            .each_entity(|e, (op,)| {
                let world = e.world();
                let item = world.entity_from_id(op.item);

                item.remove(ContainedBy);
                item.remove(ItemTag);

                op_mark_completed(e);
            });
    }
}
