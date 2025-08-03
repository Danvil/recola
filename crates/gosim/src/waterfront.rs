use crate::{
    create_human, AgentModule, Air, ContainedBy, ContainerTag, HasInventory, InventoryModule,
    ItemTag, MapModule, OpsModule, PlayerTag, Tile, TimeModule, Weight, WeightModule,
};
use flecs_ecs::prelude::*;

#[derive(Component)]
pub struct WaterfrontModule;

impl Module for WaterfrontModule {
    fn module(world: &World) {
        world.module::<WaterfrontModule>("waterfront");

        world.import::<TimeModule>();
        world.import::<AgentModule>();
        world.import::<InventoryModule>();
        world.import::<MapModule>();
        world.import::<OpsModule>();
        world.import::<WeightModule>();

        let tile_bar = world.entity_named("Bar").add(Tile).set(Air {
            oxygen_percent: 0.15,
            pollution: 150.,
        });

        let _tile_junkyard = world.entity_named("Junkyard").add(Tile).set(Air {
            oxygen_percent: 0.10,
            pollution: 50.,
        });

        let _tile_wasteland = world.entity_named("Wasteland").add(Tile).set(Air {
            oxygen_percent: 0.05,
            pollution: 350.,
        });

        let player = create_human(world.entity_named("Charn"))
            .add(PlayerTag)
            .add((flecs::ChildOf, tile_bar));

        let player_backback = world.entity_named("Charn's Backpack").add(ContainerTag);
        player.add((HasInventory, player_backback));

        world
            .entity_named("Old Dollar")
            .add(ItemTag)
            .set(Weight::new(0.01))
            .add((ContainedBy, player_backback));
        world
            .entity_named("Empty Water Bottle")
            .add(ItemTag)
            .set(Weight::new(1.))
            .add((ContainedBy, player_backback));
        world
            .entity_named("Empty Chips Bag")
            .add(ItemTag)
            .set(Weight::new(0.1))
            .add((ContainedBy, player_backback));

        // let _dan = create_human(world.entity_named("Dan the Drinker")).add((LocatedIn,
        // tile_bar));

        // let jack =
        //     create_human(world.entity_named("Jack The Junker")).add((LocatedIn, tile_junkyard));

        // let jack_inv = world.entity_named("Jack's Backpack").add(ContainerTag);
        // jack.add((HasInventory, jack_inv));

        // let _jacks_rebreather = world
        //     .entity_named("Jack's Rebreather")
        //     .add(ItemTag)
        //     .set(Rebreather {
        //         charge: 1000),
        //         oxygen_absorption_mod: Modifier::new_add(0.25),
        //         pollution_absorption_mod: Modifier::new_add(-0.50),
        //     })
        //     .set(Charge { amount: 1000) })
        //     .add((ContainedBy, jack_inv));

        // let _jack = world
        //     .entity_named("Wang The Waster")
        //     .is_a(HumanPrefab)
        //     .add((LocatedIn, tile_wasteland))
        //     .set(CurrentBloodOxygen(50));
    }
}
