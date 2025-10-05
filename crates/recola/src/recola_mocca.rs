use crate::{
    CollidersMocca, FoundationMocca, LaserPointerMocca, Rng, STATIC_SETTINGS, spawn_levels,
};
use bigtalk::{BigtalkMocca, Outbox, Router, add_route, spawn_agent};
use candy::{AssetInstance, AssetUid, CandyMocca};
use candy_asset::CandyAssetMocca;
use candy_camera::{
    CameraCommand, CameraMatrices, CameraState, CandyCameraMocca, FirstPersonCameraController,
    FirstPersonCameraControllerSettings, Projection, WindowResizedEvent,
};
use candy_forge::CandyForgeMocca;
use candy_input::{CandyInputMocca, InputEventMessage, InputState};
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
use magi_color::LinearColor;
use simplecs::prelude::*;

#[derive(Component)]
pub struct MainCamera;

pub struct RecolaMocca;

impl Mocca for RecolaMocca {
    fn load(mut deps: MoccaDeps) {
        deps.depends_on::<CandyAssetMocca>();
        deps.depends_on::<CandyCameraMocca>();
        deps.depends_on::<CandyInputMocca>();
        deps.depends_on::<CandyMeshMocca>();
        deps.depends_on::<CandyMocca>();
        deps.depends_on::<CandySceneTreeMocca>();
        deps.depends_on::<CandySkyMocca>();
        deps.depends_on::<CandyTerraMocca>();
        deps.depends_on::<CandyTimeMocca>();
        deps.depends_on::<CollidersMocca>();
        deps.depends_on::<FoundationMocca>();
        deps.depends_on::<LaserPointerMocca>();
        deps.depends_on_raw::<BigtalkMocca>();

        if STATIC_SETTINGS.enable_forge {
            deps.depends_on::<CandyForgeMocca>();
        };
    }

    fn register_components(world: &mut World) {
        world.register_component::<MainCamera>();
        world.register_component::<RiftJitter>();
        world.register_component::<InputStateController>();
        bigtalk::register_agent_components::<InputStateController, _>(world);
    }

    fn start(world: &mut World) -> Self {
        world.run(setup_window_and_camera);
        world.run(setup_sky);
        world.run(spawn_terrain);
        world.run(spawn_charn);
        world.run(spawn_levels).unwrap();
        world.run(spawn_test_rift);
        Self
    }

    fn step(&mut self, world: &mut World) {
        world.run(bigtalk::tick_agents::<InputStateController, _>);
        world.run(rift_jitter);
    }

    fn fini(&mut self, _world: &mut World) {
        log::info!("terminated.");
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
    cmd.entity(cam)
        .and_set(MainCamera)
        .and_set(CameraMatrices::new());

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
    add_route::<InputEventMessage, _>(&mut cmd, win, cam_ctrl_agent);
    add_route::<Tick, _>(&mut cmd, clock.tick_agent(), cam_ctrl_agent);

    let activator = InputStateController::new();
    let activator_agent = spawn_agent(&mut cmd, activator);
    add_route::<InputEventMessage, _>(&mut cmd, win, activator_agent);
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
    spawn_rift(cmd, &mut rng, Transform3::from_translation_xyz(9., 0., 2.));
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

#[derive(Component)]
pub struct InputStateController {
    state: InputState,
}

impl InputStateController {
    pub fn new() -> Self {
        Self {
            state: InputState::default(),
        }
    }

    pub fn state(&self) -> &InputState {
        &self.state
    }

    pub fn on_input_event(&mut self, msg: InputEventMessage) {
        self.state = msg.state;
    }
}

impl bigtalk::Agent for InputStateController {
    fn setup_message_handlers(handler: &mut bigtalk::MessageHandler<Self>) {
        handler.add(InputStateController::on_input_event);
    }
}
