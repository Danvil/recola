use atom::prelude::*;
use candy::{material::*, time::SimClock};
use magi::gems::{Lerp, Smoothstep};

/// A selection of materials which can be selected with [MaterialSwapId]
#[derive(Component)]
pub struct MaterialSwap {
    materials: Vec<Material>,
}

impl MaterialSwap {
    pub fn from_iter<I, M>(iter: I) -> Self
    where
        I: IntoIterator<Item = M>,
        M: Into<Material>,
    {
        Self {
            materials: iter.into_iter().map(|m| m.into()).collect(),
        }
    }
}

/// Indicates the desired material used by material swap
#[derive(Component, Debug)]
pub struct MaterialSwapSelection(pub usize);

impl MaterialSwapSelection {
    pub fn from_bool(flag: bool) -> Self {
        Self(flag as usize)
    }
}

/// Indicates the current material used by material swap
#[derive(Component)]
struct MaterialSwapState {
    previous: usize,
    target: usize,
    interp: Smoothstep,
}

/// Allows swapping of materials on demand
pub struct MaterialSwapMocca;

impl Mocca for MaterialSwapMocca {
    fn load(mut deps: MoccaDeps) {
        deps.depends_on::<CandyMaterialMocca>();
    }

    fn start(_world: &mut World) -> Self {
        Self
    }

    fn register_components(world: &mut World) {
        world.register_component::<MaterialSwap>();
        world.register_component::<MaterialSwapSelection>();
        world.register_component::<MaterialSwapState>();
    }

    fn step(&mut self, world: &mut World) {
        world.run(init_current_id);
        world.run(swap_materials);
    }
}

fn init_current_id(
    mut cmd: Commands,
    query: Query<(Entity, &MaterialSwapSelection), Without<MaterialSwapState>>,
) {
    for (entity, id) in query.iter() {
        cmd.entity(entity).and_set(MaterialSwapState {
            previous: id.0,
            target: id.0,
            interp: Smoothstep::default(),
        });
    }
}

fn swap_materials(
    time: Singleton<SimClock>,
    mut cmd: Commands,
    mut query: Query<(
        Entity,
        &MaterialSwap,
        &MaterialSwapSelection,
        &mut MaterialSwapState,
    )>,
) {
    let dt = time.sim_dt_f32();

    for (entity, mats, id, state) in query.iter_mut() {
        if id.0 >= mats.materials.len() {
            log::error!("invalid MaterialSwapId: {id:?}");
            continue;
        }

        if state.target != id.0 {
            state.previous = state.target;
            state.target = id.0;
            state.interp.invert_progress();
        }

        state.interp.step(dt);
        if state.interp.is_max() {
            state.previous = state.target;
        }

        let mat = mats.materials[state.previous]
            .clone()
            .lerp(mats.materials[state.target].clone(), state.interp.value());

        cmd.entity(entity).and_set(mat).and_set(MaterialDirty);
    }
}
