use crate::{
    STATIC_SETTINGS,
    level::*,
    mechanics::colliders::*,
    props::{door::KeyId, rift::RiftLevel},
    recola_mocca::RecolaAssetsMocca,
};
use atom::prelude::*;
use candy::{
    audio::*,
    camera::*,
    can::*,
    input::*,
    prelude::{DynamicTransform, HierarchyDirty, Transform3},
    sky::*,
    time::*,
    utils::{CameraLink, ImageLocation, ImageShape, WindowDef, WindowLayout},
};
use glam::{Vec2, Vec3, Vec3Swizzles};
use std::collections::HashSet;

#[derive(Component)]
pub struct MainCamera;

#[derive(Singleton)]
pub struct Player {
    pub previous_position: Vec2,

    pub eye_position: Vec3,
    pub rift_charges: HashSet<RiftLevel>,
    pub keys: HashSet<KeyId>,

    pub hours: f32,
    pub hours_target: f32,

    /// If enabled collision detection is disabled and speed is 10x
    pub cheat_ghost_mode: bool,

    /// Used to track cheat teleport
    pub cheat_teleport: usize,

    pub listener_entity: Entity,
}

/// Player camera and basic user input interaction
pub struct PlayerMocca;

impl Mocca for PlayerMocca {
    fn load(mut deps: MoccaDeps) {
        deps.depends_on::<CandyAudioMocca>();
        deps.depends_on::<CandyCameraMocca>();
        deps.depends_on::<CandyCanMocca>();
        deps.depends_on::<CandyInputMocca>();
        deps.depends_on::<CandySkyMocca>();
        deps.depends_on::<CandySkyMocca>();
        deps.depends_on::<CandyTimeMocca>();
        deps.depends_on::<CollidersMocca>();
        deps.depends_on::<RecolaAssetsMocca>();

        // FIXME currently not possible because level => foundation => rift => player
        // deps.depends_on::<LevelMocca>();

        deps.depends_on_raw::<BigtalkMocca>();
    }

    fn register_components(world: &mut World) {
        world.register_component::<MainCamera>();
        world.register_component::<InputRaycastController>();
        atom::register_agent_components::<InputRaycastController, _>(world);
    }

    fn start(world: &mut World) -> Self {
        world.run(play_welcome_clip);

        let listener_entity = world.spawn((
            Name::from_str("player listener"),
            Transform3::identity(),
            AudioListener::default(),
            DynamicTransform,
            HierarchyDirty,
        ));

        world.set_singleton(Player {
            previous_position: Vec2::ZERO,
            eye_position: Vec3::Z,
            rift_charges: HashSet::new(),
            keys: HashSet::new(),
            hours: 12.0,
            hours_target: 12.0,
            cheat_ghost_mode: false,
            cheat_teleport: 0,
            listener_entity,
        });

        world.run(setup_window_and_camera);

        Self
    }

    fn step(&mut self, world: &mut World) {
        world.run(atom::tick_agents::<InputRaycastController, _>);
        world.run(input_raycast);
        world.run(restrict_player_movement);
        world.run(update_player_eye);
        world.run(advance_time);
        world.run(update_player_entity_position);

        if STATIC_SETTINGS.enable_cheats {
            world.run(cheats);
        }
    }
}

fn play_welcome_clip(mut cmd: Commands, asset_resolver: Singleton<SharedAssetResolver>) {
    let path = asset_resolver.resolve("audio/music/intro-1.wav").unwrap();

    cmd.spawn((
        Name::from_str("background music"),
        AudioSource::new(path).with_repeat(AudioRepeatKind::OneShot),
        NonSpatialAudioSource::default(),
    ));
}

fn restrict_player_movement(
    mut player: SingletonMut<Player>,
    colliders: Singleton<ColliderWorld>,
    mut query_cam_ctrl: Query<&mut FirstPersonCameraController>,
) {
    if player.cheat_ghost_mode {
        return;
    }

    let cam_ctrl = query_cam_ctrl
        .single_mut()
        .expect("must have FirstPersonCameraController");

    // shoot rays from eye downwards
    const TEST_RAY_RADIUS: f32 = 0.333;
    const TEST_RAY_ANGLES_DEG: [f32; 8] = [0.0_f32, 45., 90., 135., 180., 225., 270., 315.];
    let new_pos = cam_ctrl.position();
    let is_colliding = TEST_RAY_ANGLES_DEG.iter().any(|angle_deg| {
        let delta = TEST_RAY_RADIUS * Vec2::from_angle(angle_deg.to_radians());
        let origin = new_pos + Vec3::new(delta.x, delta.y, 1.7);
        let ray = Ray3::from_origin_direction(origin, -Vec3::Z).unwrap();
        colliders.raycast(&ray, None, CollisionLayer::Nav).is_some()
    });

    // If there is an obstacle set back position
    if is_colliding {
        cam_ctrl.set_position_xy(player.previous_position);
    } else {
        player.previous_position = new_pos.xy();
    }
}

fn update_player_eye(
    mut player: SingletonMut<Player>,
    query_cam: Query<&CameraMatrices, With<MainCamera>>,
) {
    let cam = query_cam.single().expect("must have MainCamera");
    player.eye_position = cam.world_t_camera().transform_point3(Vec3::ZERO);
}

const HOURS_PER_RIFT_LEVEL: f32 = 1.000;
const HOURS_ADVANCE_RATE: f32 = 0.133;

fn advance_time(
    time: Singleton<SimClock>,
    mut player: SingletonMut<Player>,
    mut day_night: SingletonMut<DayNightCycle>,
) {
    let rift_level = player
        .rift_charges
        .iter()
        .map(|lvl| lvl.0)
        .max()
        .unwrap_or(0);

    player.hours_target = 12.0 + HOURS_PER_RIFT_LEVEL * rift_level as f32;
    if player.hours < player.hours_target {
        player.hours =
            (player.hours + time.sim_dt_f32() * HOURS_ADVANCE_RATE).min(player.hours_target);
    }

    day_night.local_time = SolisticDays::from_day_hour(0, player.hours as f64);
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
            title: "LUDUM DARE 58: RECOLA".to_string(),
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

    let activator = InputRaycastController::new();
    let activator_agent = spawn_agent(&mut cmd, activator);
    add_route::<InputEventMessage, _>(&mut cmd, win, activator_agent);
}

#[derive(Component)]
pub struct InputRaycastController {
    state: InputState,
    raycast_entity_and_distance: Option<(Entity, f32)>,

    cheat_ghost_mode: bool,
    cheat_teleport: usize,
}

impl InputRaycastController {
    pub fn new() -> Self {
        Self {
            state: InputState::default(),
            raycast_entity_and_distance: None,
            cheat_ghost_mode: false,
            cheat_teleport: 0,
        }
    }

    pub fn state(&self) -> &InputState {
        &self.state
    }

    pub fn raycast_entity_and_distance(&self) -> Option<(Entity, f32)> {
        self.raycast_entity_and_distance
    }

    pub fn on_input_event(&mut self, msg: InputEventMessage) {
        self.state = msg.state;

        match msg.event {
            InputEvent::KeyboardInput {
                state: ElementState::Pressed,
                code: KeyCode::KeyG,
                ..
            } => {
                self.cheat_ghost_mode = !self.cheat_ghost_mode;
            }
            _ => {}
        }
        match msg.event {
            InputEvent::KeyboardInput {
                state: ElementState::Pressed,
                code: KeyCode::KeyT,
                ..
            } => {
                self.cheat_teleport += 1;
            }
            _ => {}
        }
    }
}

impl atom::Agent for InputRaycastController {
    fn setup_message_handlers(handler: &mut atom::MessageHandler<Self>) {
        handler.add(InputRaycastController::on_input_event);
    }
}

fn input_raycast(
    colliders: Singleton<ColliderWorld>,
    mut query_input_raycast: Query<&mut InputRaycastController>,
    query_cam: Query<&CameraMatrices, With<MainCamera>>,
    query_routing: Query<&CollisionRouting>,
) {
    let input_raycast = query_input_raycast.single_mut().unwrap();
    input_raycast.raycast_entity_and_distance = None;

    // Ray through center pixel
    let Some(cam) = query_cam.single() else {
        return;
    };
    let ray = cam.center_pixel_ray();

    // Find collider along ray
    let Some((hit_entity, lam)) = colliders
        .raycast(&ray, None, CollisionLayer::Interact)
        .map(|(id, lam)| (colliders[id].user(), lam))
    else {
        return;
    };

    // Find attached collider
    let Some(collisiont_routing) = query_routing.get(hit_entity) else {
        return;
    };

    input_raycast.raycast_entity_and_distance = Some((collisiont_routing.on_raycast_entity, lam));
}

fn cheats(
    mut player: SingletonMut<Player>,
    levels: Singleton<LevelSummary>,
    mut query_input_raycast: Query<&mut InputRaycastController>,
    mut query_cam_ctrl: Query<&mut FirstPersonCameraController>,
) {
    let input_raycast = query_input_raycast.single_mut().unwrap();
    let cam_ctrl = query_cam_ctrl.single_mut().unwrap();

    // dev mode: toggle ghost mode
    player.cheat_ghost_mode = input_raycast.cheat_ghost_mode;
    let settings = cam_ctrl.settings_mut();
    if input_raycast.cheat_ghost_mode {
        settings.move_max_speed = 6.0 * 4.;
        settings.move_acceleration = 20.0 * 4.;
        settings.move_deacceleration = 25.0 * 4.;
    } else {
        settings.move_max_speed = 6.0;
        settings.move_acceleration = 20.0;
        settings.move_deacceleration = 25.0;
    }

    // dev mode: Teleport player to level start
    if player.cheat_teleport != input_raycast.cheat_teleport {
        player.cheat_teleport = input_raycast.cheat_teleport;
        cam_ctrl.set_position_xy(levels.pos[player.cheat_teleport % levels.pos.len()].xy());
    }
}

fn update_player_entity_position(player: Singleton<Player>, mut query_tf: Query<&mut Transform3>) {
    query_tf
        .get_mut(player.listener_entity)
        .unwrap()
        .translation = player.eye_position;
}
