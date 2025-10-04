use crate::{GlobalAssetPath, load_assets, spawn_levels};
use bigtalk::{BigtalkMocca, Outbox, Router, add_route, spawn_agent};
use candy::{AssetInstance, AssetLoaded, AssetUid, CandyMocca, MaterialDirty};
use candy_camera::{
    CameraCommand, CameraMatrices, CameraState, CandyCameraMocca, FirstPersonCameraController,
    FirstPersonCameraControllerSettings, Projection, WindowResizedEvent,
};
use candy_forge::{CandyForgeMocca, Transform3Edit};
use candy_input::{CandyInputMocca, InputEventMessage};
use candy_mesh::{CandyMeshMocca, Cuboid};
use candy_scene_tree::{CandySceneTreeMocca, Transform3, Visibility};
use candy_sky::{CandySkyMocca, DayNightCycle, SkyModel, SolisticDays};
use candy_terra::CandyTerraMocca;
use candy_time::{CandyTimeMocca, SimClock, Tick};
use candy_utils::{
    CameraLink, ImageLocation, ImageShape, Material, PbrMaterial, WindowDef, WindowLayout,
};
use excess::prelude::*;
use glam::{Vec2, Vec3};
use magi_color::{LinearColor, SRgbU8Color, colors};
use magi_se::SO3;
use simplecs::prelude::*;
use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
    process::Child,
};

pub struct RecolaMocca;

impl Mocca for RecolaMocca {
    fn load(mut deps: MoccaDeps) {
        deps.depends_on_raw::<BigtalkMocca>();
        deps.depends_on::<CandyCameraMocca>();
        deps.depends_on::<CandyForgeMocca>();
        deps.depends_on::<CandyInputMocca>();
        deps.depends_on::<CandyMeshMocca>();
        deps.depends_on::<CandyMocca>();
        deps.depends_on::<CandySceneTreeMocca>();
        deps.depends_on::<CandySkyMocca>();
        deps.depends_on::<CandyTerraMocca>();
        deps.depends_on::<CandyTimeMocca>();
        deps.depends_on::<CandyInputMocca>();
        deps.depends_on::<CandyMeshMocca>();
        deps.depends_on::<CandySceneTreeMocca>();
        deps.depends_on::<CandyTerraMocca>();
        deps.depends_on::<CandyTimeMocca>();
    }

    fn register_components(world: &mut World) {
        world.register_component::<BlueprintApplied>();
        world.register_component::<RiftJitter>();
        world.register_component::<LaserPointer>();
        world.register_component::<LaserPointerTarget>();
    }

    fn start(world: &mut World) -> Self {
        world.set_singleton(GlobalAssetPath(PathBuf::from("assets/recola")));

        world.set_singleton(Rng(magi_rng::Rng::from_seed(16667)));

        world.run(setup_window_and_camera);
        world.run(load_assets).unwrap();
        world.run(setup_sky);
        world.run(spawn_terrain);
        world.run(spawn_charn);
        world.run(spawn_levels).unwrap();
        world.run(spawn_test_rift);

        Self
    }

    fn step(&mut self, world: &mut World) {
        world.run(load_asset_blueprints);
        world.run(rift_jitter);

        world.run(point_laser_pointers);

        world.run(set_laser_target_material);
    }

    fn fini(&mut self, _world: &mut World) {
        log::info!("terminated.");
    }
}

#[derive(Singleton)]
pub struct Rng(magi_rng::Rng);

impl Deref for Rng {
    type Target = magi_rng::Rng;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Rng {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

fn setup_window_and_camera(clock: Singleton<SimClock>, mut cmd: Commands) {
    let cam = spawn_agent(
        &mut cmd,
        CameraState::from_eye_target_up(
            Vec3::new(30., 30., 1.70),
            Vec3::new(35., 35., 2.00),
            Vec3::Z,
            Projection::Perspective {
                fov: 60.0_f32.to_radians(),
                near: 0.05,
                far: 200.,
            },
        ),
    );
    cmd.entity(cam).set(CameraMatrices::new());

    let cam_ctrl_settings = FirstPersonCameraControllerSettings {
        move_max_speed: 6.0,
        move_acceleration: 20.0,
        move_deacceleration: 25.0,
        yaw_sensitivity: 0.0012,
        pitch_sensitivity: 0.0012,
        pitch_range: (-85.0_f32.to_radians())..(85.0_f32.to_radians()),
        height_smoothing_halflife: 0.15,
        eye_height_clearance: 1.7,
    };
    let mut cam_ctrl = FirstPersonCameraController::new(cam_ctrl_settings);
    cam_ctrl.set_position_xy(Vec2::new(-4.5, -4.5));
    cam_ctrl.set_yaw(90.0_f32.to_radians());
    let cam_ctrl_agent = spawn_agent(&mut cmd, cam_ctrl);
    add_route::<CameraCommand, _>(&mut cmd, cam_ctrl_agent, cam);

    let win = cmd.spawn((
        WindowDef {
            title: "LUDUM DARE 57: RECOLA".to_string(),
            layout: WindowLayout {
                shape: ImageShape::from_width_height(1920, 1080),
                position: ImageLocation::from_horizontal_vertical(200., 200.),
            },
            cursor_visible: false,
            confine_cursor: true,
        },
        Outbox::new(),
        Router::new(),
        (CameraLink, cam),
    ));

    add_route::<WindowResizedEvent, _>(&mut cmd, win, cam);
    add_route::<InputEventMessage, _>(&mut cmd, win, cam_ctrl_agent);
    add_route::<Tick, _>(&mut cmd, clock.tick_agent(), cam_ctrl_agent);
}

fn setup_sky(mut sky: SingletonMut<SkyModel>, mut day_night: SingletonMut<DayNightCycle>) {
    sky.set_sun_raw_radiance(18.0);
    sky.set_moon_raw_radiance(0.35);
    day_night.speed_factor = 0.;
    day_night.local_time = SolisticDays::from_day_hour(0, 12.0);
}

fn spawn_terrain(mut cmd: Commands) {
    const GROUND_PLANE_SIZE: f32 = 1024.;

    cmd.spawn((
        Name::from_str("ground plane"),
        Transform3::identity()
            .with_translation(Vec3::new(0.0, 0.0, -0.55))
            .with_scale(Vec3::new(GROUND_PLANE_SIZE, GROUND_PLANE_SIZE, 1.)),
        Visibility::Visible,
        Cuboid,
        Material::Pbr(PbrMaterial {
            base_color: LinearColor::from_rgb(0.10, 0.10, 0.10),
            metallic: 0.,
            perceptual_roughness: 1.0,
            reflectance: 0.50,
            coat_strength: 0.,
            coat_roughness: 0.,
        }),
    ));
}

fn spawn_charn(mut cmd: Commands) {
    cmd.spawn((Transform3::from_translation_xyz(25., 27., 0.), Cuboid));
}

fn spawn_test_rift(cmd: Commands, mut rng: SingletonMut<Rng>) {
    spawn_rift(cmd, &mut rng, Transform3::from_translation_xyz(9., 0., 3.));
}

fn spawn_rift(mut spawn: impl Spawn, rng: &mut Rng, transform: Transform3) -> Entity {
    let jitter = 0.1;

    let rift_entity = spawn.spawn((Name::from_str("rift"), transform));

    for _ in 0..20 {
        let anchor = 2.0 * (rng.unit_vec3() - 0.5) * jitter;

        spawn.spawn((
            Name::from_str("rift"),
            RiftJitter {
                anchor,
                target: anchor,
                speed: 0.16667,
                cooldown: 0.2 * rng.unit_f32(),
            },
            Transform3::from_translation(anchor),
            AssetInstance(AssetUid::new("prop-rift")),
            (ChildOf, rift_entity),
        ));
    }

    rift_entity
}

#[derive(Component)]
pub struct BlueprintApplied;

fn load_asset_blueprints(
    mut cmd: Commands,
    query: Query<(Entity, &AssetInstance), (With<AssetLoaded>, Without<BlueprintApplied>)>,
    children: Relation<ChildOf>,
    query_name: Query<&Name>,
) {
    for (entity, ainst) in query.iter() {
        match ainst.as_str() {
            "prop-laser" => {
                let pointer =
                    find_child_by_name(&children, &query_name, entity, "pointer").unwrap();
                build_laser_pointer(&mut cmd, pointer);
            }
            "prop-beam_target" => {
                let target = find_child_by_name(&children, &query_name, entity, "target").unwrap();
                build_laser_target(&mut cmd, target);
            }
            _ => {}
        }

        cmd.entity(entity).set(BlueprintApplied);
    }
}

fn find_child_by_name(
    children: &Relation<ChildOf>,
    query_name: &Query<&Name>,
    entity: Entity,
    needle: &str,
) -> Option<Entity> {
    for child_entity in children.iter(entity) {
        if let Some(child_name) = query_name.get(child_entity) {
            if child_name.as_str() == needle {
                return Some(child_entity);
            }
        }
        if let Some(out) = find_child_by_name(children, query_name, child_entity, needle) {
            return Some(out);
        }
    }
    None
}

const CRIMSON: SRgbU8Color = SRgbU8Color::from_rgb(220, 20, 60);
const BEAM_LEN: f32 = 100.;
const BEAM_WIDTH: f32 = 0.0167;

#[derive(Component)]
pub struct LaserPointer {
    pub dir: Vec3,
    pub target_dir: Vec3,

    #[cfg(feature = "disco")]
    pub disco_rng_dir_cooldown: f32,
}

fn build_laser_pointer(cmd: &mut Commands, entity: Entity) {
    cmd.entity(entity).and_set(LaserPointer {
        dir: Vec3::Z,
        target_dir: Vec3::Z,

        #[cfg(feature = "disco")]
        disco_rng_dir_cooldown: 0.,
    });

    let _beam_entity = cmd.spawn((
        Transform3::identity()
            .with_scale_xyz(BEAM_LEN, BEAM_WIDTH, BEAM_WIDTH)
            .with_translation_xyz(BEAM_LEN * 0.5, 0., 0.),
        Visibility::Visible,
        Cuboid,
        Material::Pbr(PbrMaterial::default().with_base_color(CRIMSON)),
        (ChildOf, entity),
    ));
}

fn point_laser_pointers(
    time: Singleton<SimClock>,
    #[cfg(feature = "disco")] mut rng: SingletonMut<Rng>,
    mut query: Query<(&mut Transform3, &mut LaserPointer)>,
) {
    let dt = time.sim_dt_f32();
    let speed = 1.0;

    for (tf, lp) in query.iter_mut() {
        lp.dir = lp.dir.lerp(lp.target_dir, speed * dt).normalize();
        tf.rotation = SO3::from_to(Vec3::X, lp.dir);

        #[cfg(feature = "disco")]
        {
            lp.disco_rng_dir_cooldown -= dt;
            if lp.disco_rng_dir_cooldown <= 0. {
                lp.disco_rng_dir_cooldown += rng.uniform(3. ..6.);
                lp.target_dir = rng.sphere_point();
            }
        }
    }
}

#[derive(Component)]
pub struct LaserPointerTarget {
    pub is_activated: bool,
    pub target_is_activated: bool,

    #[cfg(feature = "disco")]
    pub debug_disco_counter: usize,
}

fn build_laser_target(cmd: &mut Commands, entity: Entity) {
    cmd.entity(entity)
        .and_set(LaserPointerTarget {
            is_activated: false,
            target_is_activated: false,

            #[cfg(feature = "disco")]
            debug_disco_counter: 0,
        })
        .and_set(Material::Pbr(PbrMaterial::default()));
}

fn set_laser_target_material(
    mut cmd: Commands,
    mut query: Query<(Entity, &mut Material, &mut LaserPointerTarget)>,
) {
    let mat_active = PbrMaterial::diffuse_white().with_base_color(CRIMSON);
    let mat_inactive = PbrMaterial::diffuse_white().with_base_color(colors::BLACK);

    for (entity, mat, laser_target) in query.iter_mut() {
        if laser_target.target_is_activated != laser_target.is_activated {
            laser_target.is_activated = laser_target.target_is_activated;

            if laser_target.is_activated {
                *mat = Material::Pbr(mat_active.clone());
            } else {
                *mat = Material::Pbr(mat_inactive.clone());
            }

            cmd.entity(entity).set(MaterialDirty);
        }

        #[cfg(feature = "disco")]
        {
            laser_target.debug_disco_counter += 1;
            if laser_target.debug_disco_counter > 100 {
                laser_target.target_is_activated = !laser_target.is_activated;
                laser_target.debug_disco_counter = 0;
            }
        }
    }
}

#[derive(Component)]
pub struct RiftJitter {
    anchor: Vec3,
    target: Vec3,
    speed: f32,
    cooldown: f32,
}

fn rift_jitter(
    time: Singleton<SimClock>,
    mut rng: SingletonMut<Rng>,
    mut query: Query<(&mut RiftJitter, &mut Transform3)>,
) {
    let dt = time.sim_dt_f32();

    let jitter = Vec3::new(0.133, 0.133, 0.333);

    for (jit, tf) in query.iter_mut() {
        jit.cooldown -= dt;
        if jit.cooldown <= 0. {
            jit.cooldown += rng.uniform(1. ..3.);
            let delta = 2.0 * (rng.unit_vec3() - 0.5);
            jit.target = jit.anchor + delta * jitter;
        }
        let dir = jit.target - tf.translation;
        tf.translation += dir * jit.speed * dt;
    }
}
