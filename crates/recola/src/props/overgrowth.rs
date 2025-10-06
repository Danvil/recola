use crate::props::laser_pointer::*;
use candy::MaterialDirty;
use candy_time::{CandyTimeMocca, SimClock};
use candy_utils::{Material, PbrMaterial};
use excess::prelude::*;
use magi_color::{LinearColor, SRgbU8Color};
use simplecs::prelude::*;

#[derive(Component)]
pub struct InitOvergrowthTask {
    pub change_mat_entity: Entity,
}

#[derive(Component)]
pub struct Overgrowth {
    pub burn_progress: f32,
    pub change_mat_entity: Entity,
}

const OVERGROWTH_BURN_DURATION: f32 = 3.33;

/// Owergrowth which can be burned away
pub struct OvergrowthMocca;

impl Mocca for OvergrowthMocca {
    fn load(mut deps: MoccaDeps) {
        deps.depends_on::<LaserPointerMocca>();
        deps.depends_on::<CandyTimeMocca>();
    }

    fn start(_world: &mut World) -> Self {
        Self
    }

    fn register_components(world: &mut World) {
        world.register_component::<InitOvergrowthTask>();
        world.register_component::<Overgrowth>();
    }

    fn step(&mut self, world: &mut World) {
        world.run(init_overgrowth);
        world.run(burn_overgrowth);
    }
}

fn init_overgrowth(mut cmd: Commands, query_task: Query<(Entity, &InitOvergrowthTask)>) {
    for (entity, task) in query_task.iter() {
        cmd.entity(entity).remove::<InitOvergrowthTask>();
        cmd.entity(entity)
            .and_set(Overgrowth {
                burn_progress: 0.,
                change_mat_entity: task.change_mat_entity,
            })
            .and_set(BeamDetector { latch: false });
    }
}

fn burn_overgrowth(
    mut cmd: Commands,
    time: Singleton<SimClock>,
    mut query: Query<(Entity, &mut Overgrowth, &BeamHit)>,
) {
    let color_fresh: LinearColor = SRgbU8Color::from_rgb(64, 87, 22).to_linear();
    let color_burnt: LinearColor = SRgbU8Color::from_rgb(219, 153, 53).to_linear();

    let dt = time.sim_dt_f32();
    for (entity, overgrowth, hit) in query.iter_mut() {
        if hit.as_bool() {
            overgrowth.burn_progress += dt;
            let q = overgrowth.burn_progress / OVERGROWTH_BURN_DURATION;

            if q >= 1. {
                cmd.despawn_recursive(entity);
            }

            let color = color_fresh.mix(q, color_burnt);
            let mat = PbrMaterial::diffuse_white().with_base_color(color);

            cmd.entity(overgrowth.change_mat_entity)
                .and_set(Material::Pbr(mat))
                .and_set(MaterialDirty);
        }
    }
}
