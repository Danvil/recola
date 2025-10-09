use crate::props::laser_pointer::*;
use atom::prelude::*;
use candy::material::*;
use candy::prims::*;
use candy::rng::*;
use candy::scene_tree::*;
use candy::time::*;
use glam::Vec3;
use magi::color::{LinearColor, SRgbU8Color, colors};

#[derive(Component)]
pub struct InitOvergrowthTask {
    pub change_mat_entity: Entity,
}

#[derive(Component)]
pub struct Overgrowth {
    pub burn_progress: f32,
    pub burn_particle_gen: f32,
    pub change_mat_entity: Entity,
}

const OVERGROWTH_BURN_DURATION: f32 = 3.33;

/// Owergrowth which can be burned away
pub struct OvergrowthMocca;

impl Mocca for OvergrowthMocca {
    fn load(mut deps: MoccaDeps) {
        deps.depends_on::<CandyMaterialMocca>();
        deps.depends_on::<CandyMaterialMocca>();
        deps.depends_on::<CandyPrimsMocca>();
        deps.depends_on::<CandyRngMocca>();
        deps.depends_on::<CandySceneTreeMocca>();
        deps.depends_on::<CandyTimeMocca>();
        deps.depends_on::<CandyTimeMocca>();
        deps.depends_on::<LaserPointerMocca>();
    }

    fn start(_world: &mut World) -> Self {
        Self
    }

    fn register_components(world: &mut World) {
        world.register_component::<InitOvergrowthTask>();
        world.register_component::<Overgrowth>();
        world.register_component::<OvergrowthBurnParticle>();
    }

    fn step(&mut self, world: &mut World) {
        world.run(init_overgrowth);
        world.run(burn_overgrowth);
        world.run(spawn_overgrowth_burn_particles);
        world.run(animate_overgrowth_burn_particles);
    }
}

fn init_overgrowth(mut cmd: Commands, query_task: Query<(Entity, &InitOvergrowthTask)>) {
    for (entity, task) in query_task.iter() {
        cmd.entity(entity).remove::<InitOvergrowthTask>();
        cmd.entity(entity)
            .and_set(Overgrowth {
                burn_progress: 0.,
                burn_particle_gen: 0.,
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
            overgrowth.burn_particle_gen += dt;

            overgrowth.burn_progress += dt;
            let q = overgrowth.burn_progress / OVERGROWTH_BURN_DURATION;

            if q >= 1. {
                cmd.despawn_recursive(entity);
            }

            let color = color_fresh.mix(q, color_burnt);
            let mat = PbrMaterial::diffuse_white()
                .with_base_color(color)
                .with_emission(color_burnt * q);

            cmd.entity(overgrowth.change_mat_entity)
                .and_set(Material::Pbr(mat))
                .and_set(MaterialDirty);
        }
    }
}

#[derive(Component)]
struct OvergrowthBurnParticle {
    age: f32,
}

const OVERGROWTH_BURN_PARTICLE_SPAWN_RATE: f32 = 0.0100;
const OVERGROWTH_BURN_SPAWN_BOX: Vec3 = Vec3::new(3.0, 0.3, 6.0);
const OVERGROWTH_BURN_PARTICLE_MAX_AGE: f32 = 1.;
const OVERGROWTH_BURN_PARTICLE_SIZE: f32 = 0.180;
const OVERGROWTH_BURN_PARTICLE_SPEED: f32 = 2.50;
const OVERGROWTH_BURN_PARTICLE_AGE_SIZE_PEAK: f32 = 0.93;

fn spawn_overgrowth_burn_particles(
    mut cmd: Commands,
    mut rng: SingletonMut<Rng>,
    mut query: Query<(&GlobalTransform3, &mut Overgrowth)>,
) {
    for (tf, overgrowth) in query.iter_mut() {
        while overgrowth.burn_particle_gen >= OVERGROWTH_BURN_PARTICLE_SPAWN_RATE {
            overgrowth.burn_particle_gen -= OVERGROWTH_BURN_PARTICLE_SPAWN_RATE;

            cmd.spawn((
                OvergrowthBurnParticle { age: 0. },
                Cuboid,
                Material::Pbr(PbrMaterial::default().with_base_color(colors::BLACK)),
                Visibility::Visible,
                Transform3::identity()
                    .with_translation(
                        tf.transform_point3(
                            rng.uniform_vec3(Vec3::ZERO, OVERGROWTH_BURN_SPAWN_BOX),
                        ),
                    )
                    .with_rotation(rng.uniform_so3())
                    .with_scale_uniform(0.),
                DynamicTransform,
                HierarchyDirty,
            ));
        }
    }
}

fn animate_overgrowth_burn_particles(
    mut cmd: Commands,
    time: Singleton<SimClock>,
    mut query: Query<(Entity, &mut OvergrowthBurnParticle, &mut Transform3)>,
) {
    let age_q_2 = 0.100;

    let color_1: LinearColor = SRgbU8Color::from_rgb(255, 242, 156).to_linear();
    let color_2: LinearColor = SRgbU8Color::from_rgb(240, 97, 26).to_linear();
    let color_3: LinearColor = SRgbU8Color::from_rgb(23, 23, 23).to_linear();

    let em_1: LinearColor = SRgbU8Color::from_rgb(255, 242, 156).to_linear() * 10.0;
    let em_2: LinearColor = SRgbU8Color::from_rgb(240, 97, 26).to_linear() * 2.0;
    let em_3: LinearColor = SRgbU8Color::from_rgb(23, 23, 23).to_linear() * 0.0;

    let dt = time.sim_dt_f32();
    let step = dt * OVERGROWTH_BURN_PARTICLE_SPEED;

    for (entity, particle, tf) in query.iter_mut() {
        particle.age += dt;
        let rel_age = particle.age / OVERGROWTH_BURN_PARTICLE_MAX_AGE;
        if rel_age >= 1. {
            cmd.despawn(entity);
            continue;
        }

        let age_size = if rel_age < OVERGROWTH_BURN_PARTICLE_AGE_SIZE_PEAK {
            rel_age / OVERGROWTH_BURN_PARTICLE_AGE_SIZE_PEAK
        } else {
            1.0 - (rel_age - OVERGROWTH_BURN_PARTICLE_AGE_SIZE_PEAK)
                / (1.0 - OVERGROWTH_BURN_PARTICLE_AGE_SIZE_PEAK)
        };
        tf.scale = Vec3::splat(OVERGROWTH_BURN_PARTICLE_SIZE * age_size);

        let age_color = if rel_age < age_q_2 {
            color_1.mix(rel_age / age_q_2, color_2)
        } else {
            color_2.mix((rel_age - age_q_2) / (1. - age_q_2), color_3)
        };
        let age_em = if rel_age < age_q_2 {
            em_1.mix(rel_age / age_q_2, em_2)
        } else {
            em_2.mix((rel_age - age_q_2) / (1. - age_q_2), em_3)
        };
        let mat = PbrMaterial::diffuse_white()
            .with_base_color(age_color)
            .with_emission(age_em);

        cmd.entity(entity)
            .and_set(Material::Pbr(mat))
            .and_set(MaterialDirty);

        tf.translation.z += step;
    }
}
