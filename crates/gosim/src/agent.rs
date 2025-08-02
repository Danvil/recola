use crate::{
    create_blood_vessel, create_heart, create_lungs, utils::EntityBuilder, volume_from_liters,
    BloodModule, BodyPart, BodyPartModule, HeartJunctions, HeartModule, LungsJunctions,
    LungsModule, PipeConnectionHelper, PortTag, TissueBuilder,
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

    let mut con = PipeConnectionHelper::default();

    let part_f = |name| world.entity_named(name).child_of(human);

    let heart = part_f("heart");
    let HeartJunctions {
        red_in,
        red_out,
        blue_in,
        blue_out,
    } = create_heart(&world, heart, &mut con);
    heart.set(BodyPart::Heart);

    // lungs
    {
        let lungs = part_f("lungs");
        let LungsJunctions {
            pulmonary_in,
            pulmonary_out,
            bronchial_in,
            bronchial_out,
        } = create_lungs(&world, lungs, &mut con);
        lungs.set(BodyPart::Lungs);
        con.join_junctions(&world, blue_out, pulmonary_in);
        con.join_junctions(&world, red_in, pulmonary_out);
        con.join_junctions(&world, red_out, bronchial_in);
        con.join_junctions(&world, blue_in, bronchial_out);
    }

    // torso
    {
        let torso_artery =
            create_blood_vessel(&world, part_f("torso artery"), volume_from_liters(0.100));

        let torso = create_blood_vessel(&world, part_f("torso"), volume_from_liters(0.500));
        TissueBuilder {
            volume: volume_from_liters(10.),
        }
        .build(&world, torso)
        .set(BodyPart::Torso);

        let torso_vein =
            create_blood_vessel(&world, part_f("torso vein"), volume_from_liters(0.400));

        con.connect_to_junction((torso_artery, PortTag::A), red_out);
        con.connect_chain(&[torso_artery, torso, torso_vein]);
        con.connect_to_junction((torso_vein, PortTag::B), blue_in);
    }

    con.write_dot(&world, "tmp/human.dot").ok();

    human
}
