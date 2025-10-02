use crate::{
    BloodMocca, BodyPart, BodyPartMocca, HeartJunctions, HeartMocca, LungsJunctions, LungsMocca,
    PipeConnectionHelper, TissueBuilder, create_blood_vessels, create_heart, create_lungs,
    ecs::prelude::*, utils::EntityBuilder,
};
use flowsim::PortTag;
use gems::volume_from_liters;

#[derive(Component)]
pub struct AgentMocca;

/// Marker for the player entity
#[derive(Component)]
pub struct PlayerTag;

impl Mocca for AgentMocca {
    fn load(mut dep: MoccaDeps) {
        dep.depends_on::<BodyPartMocca>();
        dep.depends_on::<BloodMocca>();
        dep.depends_on::<HeartMocca>();
        dep.depends_on::<LungsMocca>();
    }

    fn register_components(world: &mut World) {
        world.register_component::<PlayerTag>();
    }

    fn start(_world: &mut World) -> Self {
        Self
    }
}

pub fn create_human(mut human: EntityWorldMut) -> EntityWorldMut {
    let mut con = PipeConnectionHelper::default();

    fn part_f<'a>(world: &'a mut World, name: &'static str) -> EntityWorldMut<'a> {
        // TODO make .child_of(human);
        let entity = world.spawn((Name::from_str(name),));
        world.entity(entity).unwrap()
    }

    let heart = part_f(human.world_mut(), "heart").and_set(BodyPart::Heart);
    let HeartJunctions {
        red_in,
        red_out,
        blue_in,
        blue_out,
    } = create_heart(heart, &mut con);

    // lungs
    {
        let lungs = part_f(human.world_mut(), "lungs").and_set(BodyPart::Lungs);
        let LungsJunctions {
            pulmonary_in,
            pulmonary_out,
            bronchial_in,
            bronchial_out,
        } = create_lungs(lungs, &mut con);
        con.join_junctions(human.world_mut(), blue_out, pulmonary_in);
        con.join_junctions(human.world_mut(), red_in, pulmonary_out);
        con.join_junctions(human.world_mut(), red_out, bronchial_in);
        con.join_junctions(human.world_mut(), blue_in, bronchial_out);
    }

    // torso
    {
        let torso_artery = create_blood_vessels(
            part_f(human.world_mut(), "torso artery"),
            volume_from_liters(0.100),
        )
        .id();

        let torso = create_blood_vessels(
            part_f(human.world_mut(), "torso"),
            volume_from_liters(0.500),
        );
        let torso = TissueBuilder {
            volume: volume_from_liters(10.),
        }
        .build(torso)
        .and_set(BodyPart::Torso)
        .id();

        let torso_vein = create_blood_vessels(
            part_f(human.world_mut(), "torso vein"),
            volume_from_liters(0.400),
        )
        .id();

        con.connect_to_junction((torso_artery, PortTag::A), red_out);
        con.connect_chain(human.world_mut(), &[torso_artery, torso, torso_vein]);
        con.connect_to_junction((torso_vein, PortTag::B), blue_in);
    }

    human
}
