use crate::{
    create_blood_vessels, ecs::prelude::*, stat_component, utils::EntityBuilder, BloodMocca,
    BloodVesselBuilder, BodyPartMocca, CardiacCycle, CardiacCycleStage, ExternalPipePressure,
    FlowDirection, FlowSimMocca, PipeConnectionHelper, Time, TimeMocca, TissueBuilder,
    ValveBuilder, ValveDef, ValveKind,
};
use flowsim::{models::ElasticTube, PortTag};
use gems::{volume_from_liters, BeatEma, Cylinder};

/// The heart is an organ which pumps blood through the body.
#[derive(Component)]
pub struct HeartMocca;

#[derive(Singleton)]
pub struct HeartConfig {}

/// Internal state used for computation of heart beat
#[derive(Component, Clone, Default)]
struct HeartBeatState {
    cycle: CardiacCycle,
}

stat_component!(
    /// Heart rate stored as beats per minute
    HeartRateBpm
);

/// The ventricles of the heart
#[derive(Component)]
struct HeartChambers {
    blue_atrium: Entity,
    blue_ventricle: Entity,
    red_atrium: Entity,
    red_ventricle: Entity,
}

/// Statistics measured for heart
#[derive(Component, Default, Clone)]
pub struct HeartStats {
    pub beat: bool,
    pub heart_rate: BeatEma,
    pub monitor: HeartRateMonitor,
    pub stage: CardiacCycleStage,
    pub stage_progress: f64,
}

/// Monitors heart rate similiar to the typical medical device
#[derive(Default, Clone)]
pub struct HeartRateMonitor {
    beats: Vec<bool>,
    index: usize,
}

impl HeartRateMonitor {
    pub fn with_len(cap: usize) -> Self {
        Self {
            beats: vec![false; cap].into(),
            index: 0,
        }
    }

    pub fn step(&mut self, beat: bool) {
        self.beats[self.index] = beat;
        self.index = (self.index + 1) % self.beats.len();
    }

    pub fn as_slice(&self) -> &[bool] {
        &self.beats
    }

    pub fn latest_index(&self) -> usize {
        self.index
    }
}

/// Create a standard human heart
///
/// The heart has two chambers each for the pulmonary (oxygen enrichment) and systemic (oxygen
/// supply) loops. The ventricle chambers are modeled as a (elastic) pipe with pressure valves
/// which only allow throughflow in the corresponding direction. The atria are simple modeled
/// as elastic pipes connected to the ventricles.
///
/// blue atrium => (tricuspid valve) blue ventricle (pulmonary valve)
/// red atrium => (mitral valve) red ventricle (aortic valve)
///
/// We run a simulation for the heart beat and correspondingly apply external pressure to the
/// ventricle pipes.
pub fn create_heart<'a>(
    entity: EntityWorldMut<'a>,
    con: &mut PipeConnectionHelper,
) -> HeartJunctions {
    let mut heart = entity
        .and_set(HeartBeatState::default())
        .and_set(HeartRateBpm::new(60.))
        .and_set(HeartRateBpmBase::new(60.))
        .and_set(HeartRateBpmMods::default())
        .and_set(HeartStats {
            heart_rate: BeatEma::from_halflife(5.),
            monitor: HeartRateMonitor::with_len(40),
            ..Default::default()
        });

    let valve_builder = ValveBuilder {
        def: &ValveDef {
            conductance_factor_closed: 0.,
            kind: ValveKind::Throughflow(FlowDirection::AtoB),
            hysteresis: 0.10,
        },
    };

    let systemic_veins = BloodVesselBuilder {
        tube: ElasticTube {
            shape: Cylinder {
                radius: 0.008,
                length: 0.150,
            },
            wall_thickness: 0.0007,
            youngs_modulus: 36_000.0,
        },
        strand_count: 3., // SVC, IVC, + 1 major tributary
        collapse_pressure: -2000.0,
    };

    let blue_atrium = BloodVesselBuilder {
        tube: ElasticTube {
            shape: Cylinder {
                radius: 0.014,
                length: 0.035,
            },
            wall_thickness: 0.0025,
            youngs_modulus: 60_000., // Pa
        },
        strand_count: 1.,
        collapse_pressure: -500., // Pa
    };

    let blue_ventricle = BloodVesselBuilder {
        tube: ElasticTube {
            shape: Cylinder {
                radius: 0.022,
                length: 0.045,
            },
            wall_thickness: 0.004,
            youngs_modulus: 75_000.,
        },
        strand_count: 1.,
        collapse_pressure: -1_000.,
    };

    let pulmonary_artery = BloodVesselBuilder {
        tube: ElasticTube {
            shape: Cylinder {
                radius: 0.006,
                length: 0.200,
            },
            wall_thickness: 0.0015,
            youngs_modulus: 300_000.,
        },
        strand_count: 2., // LPA + RPA
        collapse_pressure: -1000.0,
    };

    let pulmonary_veins = BloodVesselBuilder {
        tube: ElasticTube {
            shape: Cylinder {
                radius: 0.006,
                length: 0.150,
            },
            wall_thickness: 0.0005,
            youngs_modulus: 45_000.,
        },
        strand_count: 4.,
        collapse_pressure: -1_000.,
    };

    let red_atrium = BloodVesselBuilder {
        tube: ElasticTube {
            shape: Cylinder {
                radius: 0.018,
                length: 0.035,
            },
            wall_thickness: 0.0025,
            youngs_modulus: 60_000.,
        },
        strand_count: 1.,
        collapse_pressure: -500.,
    };

    let red_ventricle = BloodVesselBuilder {
        tube: ElasticTube {
            shape: Cylinder {
                radius: 0.028,
                length: 0.055,
            },
            wall_thickness: 0.010,
            youngs_modulus: 120_000.,
        },
        strand_count: 1.,
        collapse_pressure: -1_500.,
    };

    let aorta = BloodVesselBuilder {
        tube: ElasticTube {
            shape: Cylinder {
                radius: 0.0125,
                length: 0.300,
            },
            wall_thickness: 0.002,
            youngs_modulus: 400_000.,
        },
        strand_count: 1.,
        collapse_pressure: -1_000.,
    };

    // [vein, atrium, ventricle, artery]
    let mut heart_chamber_f = |names: [&'static str; 4], builder: [BloodVesselBuilder; 4]| {
        let vein = builder[0].new_named(heart.world_mut(), names[0]).id();

        let atrium = builder[1].new_named(heart.world_mut(), names[1]).id();
        // entities[1].child_of(heart);

        let ventricle = builder[2].new_named(heart.world_mut(), names[2]);
        let ventricle = valve_builder.build(ventricle).id();
        // entities[2].child_of(heart);

        let artery = builder[3].new_named(heart.world_mut(), names[3]).id();

        let ids = [vein, atrium, ventricle, artery];

        con.connect_chain(heart.world_mut(), &ids);

        con.connect_to_new_junction(heart.world_mut(), (vein, PortTag::A));
        con.connect_to_new_junction(heart.world_mut(), (artery, PortTag::B));

        ids
    };

    let blue = heart_chamber_f(
        [
            "systemic_veins",
            "blue_atrium",
            "blue_ventricle",
            "pulmonary_artery",
        ],
        [
            systemic_veins,
            blue_atrium,
            blue_ventricle,
            pulmonary_artery,
        ],
    );

    let red = heart_chamber_f(
        ["pulmonary_veins", "red_atrium", "red_ventricle", "aorta"],
        [pulmonary_veins, red_atrium, red_ventricle, aorta],
    );

    heart.set(HeartChambers {
        blue_atrium: blue[1],
        blue_ventricle: blue[2],
        red_atrium: red[1],
        red_ventricle: red[2],
    });

    // The heart is a body part which needs blood itself
    let heart = TissueBuilder { volume: 1.0 }.build(heart);
    let mut heart = create_blood_vessels(heart, volume_from_liters(0.050));
    let heart_id = heart.id();

    // Connect heart blood supply directly
    con.connect_chain(heart.world_mut(), &[red[3], heart_id]);
    con.connect_chain(heart.world_mut(), &[heart_id, blue[0]]);

    HeartJunctions {
        red_in: con.junction(red[0], PortTag::A).unwrap(),
        red_out: con.junction(red[3], PortTag::B).unwrap(),
        blue_in: con.junction(blue[0], PortTag::A).unwrap(),
        blue_out: con.junction(blue[3], PortTag::B).unwrap(),
    }
}

#[derive(Component, Clone)]
pub struct HeartJunctions {
    pub red_in: Entity,
    pub red_out: Entity,
    pub blue_in: Entity,
    pub blue_out: Entity,
}

impl Mocca for HeartMocca {
    fn load(mut dep: MoccaDeps) {
        dep.depends_on::<TimeMocca>();
        dep.depends_on::<BodyPartMocca>();
        dep.depends_on::<BloodMocca>();
        dep.depends_on::<FlowSimMocca>();
    }

    fn register_components(world: &mut World) {
        world.register_component::<HeartConfig>();
        world.register_component::<HeartBeatState>();
        world.register_component::<HeartRateBpm>();
        world.register_component::<HeartChambers>();
        world.register_component::<HeartStats>();
        HeartRateBpm::register_components(world);
    }

    fn start(world: &mut World) -> Self {
        world.set_singleton(HeartConfig {});

        Self
    }

    fn step(&mut self, world: &mut World) {
        HeartRateBpm::step(world);
        world.run(heart_beat);
        world.run(heart_pressure);
        world.run(heart_stats_update);
    }
}

// Check if the heart beats
fn heart_beat(query: Query<(&HeartRateBpm, &mut HeartBeatState)>, time: Singleton<Time>) {
    query.each(|(rate, state)| {
        state.cycle.set_target_bpm(**rate);
        state.cycle.step(time.sim_dt_f64());
    });
}

// Apply pressure to chambers
fn heart_pressure(
    query: Query<(&HeartChambers, &HeartBeatState)>,
    _time: Singleton<Time>,
    mut cmd: Commands,
) {
    query.each(|(chambers, state)| {
        let [red_atrium_pressure, blue_atrium_pressure, red_ventricle_pressure, blue_ventricle_pressure] = match state.cycle.stage() {
            (CardiacCycleStage::DiastolePhase1, _) => [
                ExternalPipePressure::ubiquous(0.),
                ExternalPipePressure::ubiquous(0.),
                ExternalPipePressure::ubiquous(0.),
                ExternalPipePressure::ubiquous(0.)
            ],
            (CardiacCycleStage::ArterialSystole, q) => {
                let a = attack(q);
                [
                    ExternalPipePressure::ubiquous(-1_000. * a),
                    ExternalPipePressure::ubiquous(-1_000. * a),
                    ExternalPipePressure::ubiquous(0.),
                    ExternalPipePressure::ubiquous(0.)
                ]
            }
            (CardiacCycleStage::Systole, q) => {
                let a = attack(q);
                [
                    ExternalPipePressure::ubiquous(0.),
                    ExternalPipePressure::ubiquous(0.),
                    ExternalPipePressure::ubiquous(-16_000. * a),
                    ExternalPipePressure::ubiquous(-3_300. * a)
                ]
            }
        };

        cmd.entity(chambers.red_atrium)
            .set(red_atrium_pressure);
        cmd.entity(chambers.blue_atrium)
            .set(blue_atrium_pressure);
        cmd.entity(chambers.red_ventricle)
            .set(red_ventricle_pressure);
        cmd.entity(chambers.blue_ventricle)
            .set(blue_ventricle_pressure);
    });
}

fn attack(q: f64) -> f64 {
    use core::f64::consts::PI;
    (q * PI).sin().sqrt()
}

// Update heart statistics
fn heart_stats_update(query: Query<(&HeartBeatState, &mut HeartStats)>, time: Singleton<Time>) {
    query.each(|(state, stats)| {
        let beat = state.cycle.beat();
        stats.beat = beat;
        stats.heart_rate.step(time.sim_dt_f64(), beat);
        stats.monitor.step(beat);
        stats.stage = state.cycle.stage().0;
        stats.stage_progress = state.cycle.stage().1;
    });
}
