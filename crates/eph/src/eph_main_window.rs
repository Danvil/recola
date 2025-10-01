use bigtalk::{Outbox, Router, add_route, spawn_agent};
use candy_camera::{
    CameraCommand, CameraMatrices, CameraState, GroundFocusPointCameraController,
    GroundFocusPointCameraControllerSettings, Projection, WindowResizedEvent,
};
use candy_input::{CandyInputMocca, InputEventMessage};
use candy_mesh::CandyMeshMocca;
use candy_scene_tree::CandySceneTreeMocca;
use candy_terra::CandyTerraMocca;
use candy_time::{CandyTimeMocca, SimClock, Tick};
use candy_utils::{CameraLink, ImageLocation, ImageShape, WindowDef, WindowLayout};
use excess::prelude::*;
use glam::{Vec2, Vec3};
use simplecs::prelude::*;

#[derive(Component)]
pub struct MainWindow;

pub struct EphMainWindowMocca;

impl Mocca for EphMainWindowMocca {
    fn load(mut deps: MoccaDeps) {
        deps.depends_on::<CandyInputMocca>();
        deps.depends_on::<CandyMeshMocca>();
        deps.depends_on::<CandySceneTreeMocca>();
        deps.depends_on::<CandyTerraMocca>();
        deps.depends_on::<CandyTimeMocca>();
    }

    fn register_components(world: &mut World) {
        world.register_component::<MainWindow>();
    }

    fn start(world: &mut World) -> Self {
        world.run(setup_window_and_camera);
        Self
    }

    fn step(&mut self, _world: &mut World) {}

    fn fini(&mut self, _world: &mut World) {}
}

fn setup_window_and_camera(clock: Singleton<SimClock>, mut cmd: Commands) {
    let cam = spawn_agent(
        &mut cmd,
        CameraState::from_eye_target_up(
            Vec3::new(30., 30., 1.70),
            Vec3::new(35., 35., 2.00),
            Vec3::Z,
            Projection::Perspective {
                fov: 45.0_f32.to_radians(),
                near: 0.25,
                far: 1500.,
            },
        ),
    );
    cmd.entity(cam).set(CameraMatrices::new());

    let cam_ctrl_settings = GroundFocusPointCameraControllerSettings {
        translation_max_speed: 1.2,
        translation_acceleration: 5.0,
        translation_deacceleration: 20.0,
        zoom_power: 1.30,
        azimuth_sensitivity: 2.4,
        tilt_sensitivity: 1.3,
        tilt_range: 5.0_f32.to_radians()..70.0_f32.to_radians(),
        eye_distance_range: 2.0..75.0,
        height_smoothing_halflife: 0.10,
        eye_height_clearance: 0.5,
    };
    let mut cam_ctrl = GroundFocusPointCameraController::new(cam_ctrl_settings);
    cam_ctrl.set_focus_point(Vec2::new(30., 30.));
    let cam_ctrl_agent = spawn_agent(&mut cmd, cam_ctrl);
    add_route::<CameraCommand, _>(&mut cmd, cam_ctrl_agent, cam);

    let win = cmd
        .spawn((
            MainWindow,
            WindowDef {
                title: "EARTH POWER HOUSE".to_string(),
                layout: WindowLayout {
                    shape: ImageShape::from_width_height(1920, 1080),
                    position: ImageLocation::from_horizontal_vertical(200., 200.),
                },
            },
            Outbox::new(),
            Router::new(),
            (CameraLink, cam),
        ))
        .id();

    add_route::<WindowResizedEvent, _>(&mut cmd, win, cam);
    add_route::<InputEventMessage, _>(&mut cmd, win, cam_ctrl_agent);
    add_route::<Tick, _>(&mut cmd, clock.tick_agent(), cam_ctrl_agent);
}
