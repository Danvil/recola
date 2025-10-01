use crate::eph_main_window::{EphMainWindowMocca, MainWindow};
use bigtalk::{add_route, spawn_agent};
use candy::{AssetInstance, AssetUid, CandyMocca};
use candy_input::{CandyInputMocca, ElementState, InputEvent, InputEventMessage, KeyCode};
use candy_mesh::CandyMeshMocca;
use candy_scene_tree::{CandySceneTreeMocca, Transform3};
use candy_terra::CandyTerraMocca;
use candy_time::{CandyTimeMocca, SimClock, Tick};
use excess::prelude::*;
use simplecs::prelude::*;

pub struct EphArchitectMocca;

impl Mocca for EphArchitectMocca {
    fn load(mut deps: MoccaDeps) {
        deps.depends_on::<CandyInputMocca>();
        deps.depends_on::<CandyMeshMocca>();
        deps.depends_on::<CandySceneTreeMocca>();
        deps.depends_on::<CandyTerraMocca>();
        deps.depends_on::<CandyTimeMocca>();
        deps.depends_on::<EphMainWindowMocca>();
        deps.depends_on::<CandyMocca>();
    }

    fn register_components(world: &mut World) {
        world.register_component::<ArchitectController>();
        world.register_component::<PlacementGhost>();
        bigtalk::register_agent_components::<ArchitectController, _>(world);
    }

    fn start(world: &mut World) -> Self {
        world.run(spawn_architect_controller);
        Self
    }

    fn step(&mut self, world: &mut World) {
        world.run(bigtalk::tick_agents::<ArchitectController, _>);
    }

    fn fini(&mut self, _world: &mut World) {}
}

#[derive(Component)]
struct PlacementGhost;

fn spawn_architect_controller(
    clock: Singleton<SimClock>,
    query: Query<Entity, With<MainWindow>>,
    mut cmd: Commands,
) {
    let win = query.single().unwrap();

    let slab_aid = AssetUid::new("build-concrete.slab_1x1");
    let placement_ghost = cmd
        .spawn((
            Transform3::identity(),
            AssetInstance(slab_aid.clone()),
            PlacementGhost,
        ))
        .id();

    let controller_entity = spawn_agent(&mut cmd, ArchitectController::new(placement_ghost));
    cmd.entity(controller_entity)
        .set(Name::from_str("eph_architect_controller"));
    add_route::<InputEventMessage, _>(&mut cmd, win, controller_entity);
    add_route::<Tick, _>(&mut cmd, clock.tick_agent(), controller_entity);
}

#[derive(Component)]
struct ArchitectController {
    placement_ghost: Entity,
    is_enabled: bool,
}

impl ArchitectController {
    pub fn new(placement_ghost: Entity) -> Self {
        Self {
            placement_ghost,
            is_enabled: false,
        }
    }

    pub fn on_input(&mut self, msg: InputEventMessage) {
        match msg.event {
            InputEvent::KeyboardInput {
                key: _,
                code,
                state: ElementState::Pressed,
            } => match code {
                KeyCode::KeyE => {
                    self.is_enabled = true;
                }
                KeyCode::Escape => {
                    self.is_enabled = false;
                }
                _ => {}
            },
            _ => {
                //
            }
        }
    }

    pub fn on_tick(&mut self, msg: Tick) {
        // Intersect mouse with terrain and place blueprint on it
    }
}

impl bigtalk::Agent for ArchitectController {
    fn setup_message_handlers(handler: &mut bigtalk::MessageHandler<Self>) {
        handler.add(ArchitectController::on_input);
        handler.add(ArchitectController::on_tick);
    }
}
