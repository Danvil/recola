use crate::{
    create_blood_vessel, create_heart, create_lungs, create_tissue, BloodModule, BodyPart,
    BodyPartModule, FluidFlowLink, HeartModule, HeartSlots, LungsModule, LungsSlots,
};
use flecs_ecs::prelude::*;

#[derive(Component)]
pub struct AgentModule;

/// Marker for the player entity
#[derive(Component)]
pub struct PlayerTag;

impl Module for AgentModule {
    fn module(world: &World) {
        world.module::<AgentModule>("agent");

        world.import::<BloodModule>();
        world.import::<BodyPartModule>();
        world.import::<HeartModule>();
        world.import::<LungsModule>();

        world.component::<PlayerTag>();
    }
}

pub fn create_human(human: EntityView) -> EntityView {
    let world = human.world();

    let part_f = |name| world.entity_named(name).child_of(human);

    let HeartSlots {
        heart,
        red_in,
        red_out,
        blue_in,
        blue_out,
    } = create_heart(&world, part_f("heart"));
    heart.set(BodyPart::Heart);

    // lungs
    {
        let LungsSlots {
            lungs,
            alveoli,
            bronchial_in,
            bronchial_out,
        } = create_lungs(&world, part_f("lungs"));
        lungs.set(BodyPart::Lungs);

        // create pulmonary cycle (oxygen enrichment)
        blue_out.add((FluidFlowLink, alveoli));
        alveoli.add((FluidFlowLink, red_in));

        // create bronchial cycle (nutrients)
        red_out.add((FluidFlowLink, bronchial_in));
        bronchial_out.add((FluidFlowLink, blue_in));
    }

    // torso
    {
        let torso_artery = create_blood_vessel(&world, part_f("torso artery"), 0.100);

        let torso = create_blood_vessel(&world, part_f("torso"), 0.500);
        create_tissue(torso).set(BodyPart::Torso);

        let torso_vein = create_blood_vessel(&world, part_f("torso vein"), 0.400);

        red_out.add((FluidFlowLink, torso_artery));
        torso_artery.add((FluidFlowLink, torso));
        torso.add((FluidFlowLink, torso_vein));
        torso_vein.add((FluidFlowLink, blue_in));
    }

    human
}
