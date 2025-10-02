use crate::{eph_architect::EphArchitectMocca, eph_main_window::EphMainWindowMocca};
use bigtalk::BigtalkMocca;
use candy::{AssetInstance, AssetLibrary, AssetUid, CandyMocca, GltfAssetDescriptor};
use candy_camera::CandyCameraMocca;
use candy_forge::CandyForgeMocca;
use candy_input::CandyInputMocca;
use candy_mesh::{CandyMeshMocca, Cuboid};
use candy_scene_tree::{CandySceneTreeMocca, Transform3};
use candy_sky::{CandySkyMocca, SkyModel};
use candy_terra::{
    CandyTerraMocca, Ground, LoadTerrainCommand, TerraChunkStreamingStatusLoaded, Terrain,
    TerrainChunk,
};
use candy_time::CandyTimeMocca;
use candy_utils::{Material, PbrMaterial};
use excess::prelude::*;
use flowsim::{PipeDef, PipeVessel, PortTag};
use gems::{VolumeModel, pressure_to_mm_hg, volume_to_milli_liters};
use glam::{UVec2, Vec2, Vec3};
use gosim::*;
use magi_color::LinearColor;
use magi_rng::{Range2F32, Rng};
use simplecs::prelude::*;
use std::path::PathBuf;

pub struct EphMocca;

impl Mocca for EphMocca {
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
        deps.depends_on::<EphMainWindowMocca>();
        deps.depends_on::<EphArchitectMocca>();
        // deps.depends_on::<GosSimMocca>();
    }

    fn register_components(world: &mut World) {
        world.register_component::<TerrainTileFoliageSpawned>();
    }

    fn start(world: &mut World) -> Self {
        world.run(load_assets);
        world.run(setup_sky);
        world.run(spawn_terrain);
        world.run(spawn_charn);
        world.run(spawn_church);
        // world.run(enable_flow_sim_logging);
        Self
    }

    fn step(&mut self, world: &mut World) {
        world.run(spawn_terrain_tile_foliage);
        // print_report(world);
    }

    fn fini(&mut self, _world: &mut World) {
        log::info!("terminated.");
    }
}

fn enable_flow_sim_logging(mut cfg: SingletonMut<FlowSimConfig>) {
    cfg.pipe_stats_csv_path = Some("I:/Ikabur/gos/tmp/heart/".into());
    cfg.graph_topology_path = Some("I:/Ikabur/gos/tmp/heart/".into());
    cfg.debug_print_ode_solution = true;
}

fn setup_sky(mut sky: SingletonMut<SkyModel>) {
    sky.set_sun_raw_radiance(12.0);
    sky.set_moon_raw_radiance(0.18);
}

fn spawn_terrain(mut cmd: Commands) {
    cmd.spawn(LoadTerrainCommand {
        path: PathBuf::from("I:/Ikabur/eph/assets/terrain/eph_world.json"),
    });

    const WATER_PLANE_SIZE: f32 = 1024.;

    cmd.spawn((
        Name::from_str("water plane"),
        Transform3::identity()
            .with_translation(Vec3::new(
                0.5 * WATER_PLANE_SIZE,
                0.5 * WATER_PLANE_SIZE,
                -0.5,
            ))
            .with_scale(Vec3::new(WATER_PLANE_SIZE, WATER_PLANE_SIZE, 1.)),
        Cuboid,
        Material::Pbr(PbrMaterial {
            base_color: LinearColor::from_rgb(0.02, 0.13, 0.35),
            metallic: 0.,
            perceptual_roughness: 0.05,
            reflectance: 0.35,
            coat_strength: 0.,
            coat_roughness: 0.,
        }),
    ));
}

fn load_assets(mut asli: SingletonMut<AssetLibrary>) {
    asli.load_gltf(
        &AssetUid::new("cactus"),
        GltfAssetDescriptor {
            path: PathBuf::from("I:/Ikabur/eph/assets/models/flora/cactus.glb"),
            scene: None,
            node: None,
        },
    );
    asli.load_gltf(
        &AssetUid::new("char-red_priest_germanicus"),
        GltfAssetDescriptor {
            path: PathBuf::from("I:/Ikabur/eph/assets/models/characters/red_priest_germanicus.glb"),
            scene: None,
            node: None,
        },
    );
    asli.load_gltf(
        &AssetUid::new("build-concrete.slab_1x1"),
        GltfAssetDescriptor {
            path: PathBuf::from("I:/Ikabur/eph/assets/models/buildings/build-concrete.glb"),
            scene: Some("Scene".into()),
            node: Some("build-concrete.slab_1x1".into()),
        },
    );
    asli.load_gltf(
        &AssetUid::new("build-concrete.wall_1x2"),
        GltfAssetDescriptor {
            path: PathBuf::from("I:/Ikabur/eph/assets/models/buildings/build-concrete.glb"),
            scene: Some("Scene".into()),
            node: Some("build-concrete.wall_1x2".into()),
        },
    );
    asli.load_gltf(
        &AssetUid::new("build-concrete.corner"),
        GltfAssetDescriptor {
            path: PathBuf::from("I:/Ikabur/eph/assets/models/buildings/build-concrete.glb"),
            scene: Some("Scene".into()),
            node: Some("build-concrete.corner".into()),
        },
    );
    asli.load_gltf(
        &AssetUid::new("prop-crimson_altar"),
        GltfAssetDescriptor {
            path: PathBuf::from("I:/Ikabur/eph/assets/models/prop/prop-crimson_altar.glb"),
            scene: Some("Scene".into()),
            node: Some("prop-crimson_altar".into()),
        },
    );
    asli.load_gltf(
        &AssetUid::new("prop-crimson_beacon"),
        GltfAssetDescriptor {
            path: PathBuf::from("I:/Ikabur/eph/assets/models/prop/prop-crimson_beacon.glb"),
            scene: Some("Scene".into()),
            node: Some("prop-crimson_beacon".into()),
        },
    );
}

#[derive(Component)]
struct TerrainTileFoliageSpawned;

fn spawn_terrain_tile_foliage(
    terrain: Singleton<Terrain>,
    query_tiles: Query<
        (Entity, &TerrainChunk),
        (
            With<TerraChunkStreamingStatusLoaded>,
            Without<TerrainTileFoliageSpawned>,
        ),
    >,
    mut cmd: Commands,
) {
    let cactus_aid = AssetUid::new("cactus");

    let mut rng = Rng::new();

    let spacing = 30.0;

    let terrain = terrain.inner();

    let mut count = 0;

    for (terrain_chunk_entity, terrain_chunk) in query_tiles.iter() {
        let foliage_root_entity = cmd.spawn((
            Name::from_str("foliage"),
            Transform3::identity(),
            (ChildOf, terrain_chunk_entity),
        ));

        cmd.entity(terrain_chunk_entity)
            .set(TerrainTileFoliageSpawned);

        let chunk_id = **terrain_chunk;
        let chunk_pos = terrain.chunk_position(chunk_id);

        let area_count = (terrain.chunk_size() / spacing).floor().as_uvec2();
        let roi_size = terrain.chunk_size() / area_count.as_vec2();

        for i in 0..area_count[0] {
            for j in 0..area_count[1] {
                let roi_center =
                    chunk_pos + (UVec2::new(i, j).as_vec2() + 0.5 * Vec2::ONE) * roi_size;
                let roi = Range2F32::from_center_size(roi_center, 0.8 * roi_size);
                let pos_world = rng.uniform_roi2_f32(&roi);

                let Some(loc) = terrain.locate(pos_world) else {
                    continue;
                };
                let height = terrain.height(loc);

                let pos_local = pos_world - chunk_pos;

                cmd.spawn((
                    Transform3::from_translation_xyz(pos_local.x, pos_local.y, height),
                    AssetInstance(cactus_aid.clone()),
                    (ChildOf, foliage_root_entity),
                ));

                count += 1;
            }
        }
    }

    log::debug!("spawned {} cacti", count);
}

fn spawn_charn(mut cmd: Commands) {
    cmd.spawn((
        Transform3::from_translation_xyz(35., 35., 0.).with_rotation_z_deg(180.),
        AssetInstance(AssetUid::new("char-red_priest_germanicus")),
    ));
}

fn spawn_church(mut cmd: Commands) {
    let prop_crimson_altar_aid = AssetUid::new("prop-crimson_altar");
    let prop_crimson_beacon_aid = AssetUid::new("prop-crimson_beacon");

    let church_entity = cmd.spawn((
        Name::from_str("church"),
        Transform3::from_translation_xyz(20., 20., 0.),
    ));

    spawn_foundation(&mut cmd, church_entity);
    spawn_room(&mut cmd, church_entity);

    cmd.spawn((
        Transform3::from_translation_xyz(5., 5., 0.),
        AssetInstance(prop_crimson_altar_aid.clone()),
        (ChildOf, church_entity),
    ));

    cmd.spawn((
        Transform3::from_translation_xyz(7., 5., 0.),
        AssetInstance(prop_crimson_beacon_aid.clone()),
        (ChildOf, church_entity),
    ));
}

fn spawn_foundation(cmd: &mut Commands, parent: Entity) {
    let slab_aid = AssetUid::new("build-concrete.slab_1x1");

    let foundation_entity = cmd.spawn((
        Name::from_str("foundation"),
        Transform3::identity(),
        (ChildOf, parent),
    ));

    let nx = 8 + 4;
    let ny = 10 + 4;

    for i in 0..nx {
        for j in 0..ny {
            cmd.spawn((
                Transform3::from_translation_xyz(i as f32, j as f32, 0.),
                AssetInstance(slab_aid.clone()),
                (ChildOf, foundation_entity),
            ));
        }
    }
}

fn spawn_room(cmd: &mut Commands, parent: Entity) {
    let wall_aid = AssetUid::new("build-concrete.wall_1x2");
    let corner_aid = AssetUid::new("build-concrete.corner");

    let room_entity = cmd.spawn((
        Name::from_str("room"),
        Transform3::from_translation_xyz(2., 2., 0.),
        (ChildOf, parent),
    ));

    let nx = 8;
    let ny = 10;

    for x in 1..nx - 1 {
        cmd.spawn((
            Transform3::from_translation_xyz(x as f32, 0., 0.).with_rotation_z_deg(90.),
            AssetInstance(wall_aid.clone()),
            (ChildOf, room_entity),
        ));
        cmd.spawn((
            Transform3::from_translation_xyz(x as f32, (ny - 1) as f32, 0.)
                .with_rotation_z_deg(270.),
            AssetInstance(wall_aid.clone()),
            (ChildOf, room_entity),
        ));
    }

    for y in 1..ny - 1 {
        cmd.spawn((
            Transform3::from_translation_xyz(0., y as f32, 0.),
            AssetInstance(wall_aid.clone()),
            (ChildOf, room_entity),
        ));
        cmd.spawn((
            Transform3::from_translation_xyz((nx - 1) as f32, y as f32, 0.)
                .with_rotation_z_deg(180.),
            AssetInstance(wall_aid.clone()),
            (ChildOf, room_entity),
        ));
    }

    cmd.spawn((
        Transform3::from_translation_xyz(0., 0., 0.),
        AssetInstance(corner_aid.clone()),
        (ChildOf, room_entity),
    ));
    cmd.spawn((
        Transform3::from_translation_xyz((nx - 1) as f32, 0., 0.).with_rotation_z_deg(90.),
        AssetInstance(corner_aid.clone()),
        (ChildOf, room_entity),
    ));
    cmd.spawn((
        Transform3::from_translation_xyz((nx - 1) as f32, (ny - 1) as f32, 0.)
            .with_rotation_z_deg(180.),
        AssetInstance(corner_aid.clone()),
        (ChildOf, room_entity),
    ));
    cmd.spawn((
        Transform3::from_translation_xyz(0., (ny - 1) as f32, 0.).with_rotation_z_deg(270.),
        AssetInstance(corner_aid.clone()),
        (ChildOf, room_entity),
    ));
}

fn print_report(world: &mut World) {
    // world
    //     .query::<(&CurrentBloodOxygen, &BodyTox, &CurrentBreathingOrgan)>()
    //     .build()
    //     .each_entity(|e, (oxy, tox, organ)| {
    //         println!(
    //             "{}: oxy:{:6.03?}, tox: {:6.03?}, abs:{:6.03?}, poll:{:6.03?}",
    //             e.name(),
    //             oxy.value,
    //             tox.amount,
    //             organ.oxygen_absorption,
    //             organ.pollution_absorption
    //         );
    //     });

    // world
    //     .query::<()>()
    //     .with(PlayerTag)
    //     .build()
    //     .each_entity(|e, ()| {
    //         println!("{:?}", e.name());
    //     });

    // world
    //     .query::<(&Pipe<BloodProperties>, &PipeFlowState)>()
    //     .related("$this", flecs::ChildOf, "$player")
    //     .tagged("$player", PlayerTag)
    //     .build()
    //     .each_entity(|e, (v, state)| {
    //         println!(
    //             "{:?}: V: {:.03?} l, flow: {:.03?} ml/s",
    //             e.name(),
    //             v.volume(),
    //             state.flow()
    //         );
    //     });

    world
        .query::<(&HeartRateBpm, &HeartStats, Option<&Name>)>()
        .each(|(bpm, stats, name)| {
            println!(
                "{:?}: {} BPM, beat: {}, stage: {:?} [{:4.1}%]",
                name,
                **bpm,
                stats.beat,
                stats.stage,
                stats.stage_progress * 100.
            );
        });

    world.query::<(&HeartStats,)>().each(|(stats,)| {
        if stats.beat {
            println!(">>>>> BUM BUM <<<<<");
        }
    });

    // world
    //     .query::<(&BloodStats,)>()
    //     .with(AlveoliTag)
    //     .build()
    //     .each_entity(|e, (blood,)| {
    //         println!("Alveoli {:?}: SO2: {:.1}%", e.name(), 100. * blood.so2);
    //     });

    // world
    //     .query::<(Option<&BodyPart>, &BloodVessel)>()
    //     .build()
    //     .each_entity(|e, (part, vessel)| {
    //         println!(
    //             "Vessel {:?} [{part:?}]: frags: {}",
    //             e.name(),
    //             vessel.chunks().len()
    //         );
    //     });

    println!("CARDIOVASCULAR Summary:");
    println!("{}", "-".repeat(108));
    println!(
        "| {:<16} [{:>12}] | {:>15} | {:>15} | {:>11} | {:>7} | {:>10} |",
        "Name",
        "Body Part",
        "Pressure [mmHg]",
        "Flow [mL/s]",
        "Volume [mL]",
        "SO2 [%]",
        "PO2 [mmHg]"
    );
    println!("{}", "-".repeat(108));
    world
        .query::<(
            Option<&BodyPart>,
            &BloodStats,
            &PipeDef,
            &PipeVessel,
            &PipeFlowState,
            Option<&Name>,
        )>()
        .each(|(part, blood, def, vessel, state, name)| {
            println!(
                "| {:<16} [{:>12}] | {:7.1} {:7.1} | {:7.1} {:7.1} | {:5.1} {:5.1} | {:7.1} | {:10.0} |",
                name.map_or("", |n| n.as_str()),
                part.map_or_else(|| String::new(), |x| format!("{x:?}")),
                pressure_to_mm_hg(state.pressure(PortTag::A)),
                pressure_to_mm_hg(state.pressure(PortTag::B)),
                volume_to_milli_liters(state.flow(PortTag::A)),
                volume_to_milli_liters(state.flow(PortTag::B)),
                volume_to_milli_liters(vessel.volume()),
                volume_to_milli_liters(def.shape.nominal_volume()),
                100. * blood.so2,
                blood.po2
            );
        });
    println!("{}", "-".repeat(108));

    // world
    //     .query::<(Option<&BodyPart>, &Tissue, &TissueStats)>()
    //     .build()
    //     .each_entity(|e, (part, tissue, stats)| {
    //         println!(
    //             "Tissue {:?} [{part:?}]: SO2: {:0.0}%, O2 cont: {:0.0} mL/dL",
    //             e.name(),
    //             100. * stats.o2_saturation,
    //             100. * tissue.o2_content
    //         );
    //     });
}
