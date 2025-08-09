use crate::{
    create_blood_vessels, stat_component, utils::EntityBuilder, BloodMocca, BloodVesselBuilder,
    BodyPartMocca, CardiacCycle, CardiacCycleStage, ExternalPipePressure,
    FlecsQueryRelationHelpers, FlowDirection, FlowSimMocca, PipeConnectionHelper, Time, TimeMocca,
    TissueBuilder, ValveBuilder, ValveDef, ValveKind,
};
use flecs_ecs::prelude::*;
use flowsim::{models::ElasticTube, PortTag};
use gems::{volume_from_liters, BeatEma, Cylinder};
use mocca::{Mocca, MoccaDeps};

/// The heart is an organ which pumps blood through the body.
#[derive(Component)]
pub struct HeartMocca;

#[derive(Component)]
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
    world: &'a World,
    entity: EntityView<'a>,
    con: &mut PipeConnectionHelper,
) -> HeartJunctions {
    let heart = entity
        .set(HeartBeatState::default())
        .set(HeartRateBpm::new(60.))
        .set(HeartRateBpmBase::new(60.))
        .set(HeartRateBpmMods::default())
        .set(HeartStats {
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
    let mut heart_chamber_f = |names: [&str; 4], builder: [BloodVesselBuilder; 4]| {
        let entities = names
            .iter()
            .zip(builder.iter())
            .map(|(name, b)| b.build(world, world.entity_named(name)))
            .collect::<Vec<_>>();

        entities[1].child_of(heart);
        entities[2].child_of(heart);

        valve_builder.build(world, entities[2]);

        con.connect_chain(&entities);

        con.connect_to_new_junction((entities[0], PortTag::A));
        con.connect_to_new_junction((entities[3], PortTag::B));

        entities
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
        blue_atrium: *blue[1],
        blue_ventricle: *blue[2],
        red_atrium: *red[1],
        red_ventricle: *red[2],
    });

    // The heart is a body part which needs blood itself
    TissueBuilder { volume: 1.0 }.build(world, heart);
    let heart_vessel = create_blood_vessels(world, heart, volume_from_liters(0.050));

    // Connect heart blood supply directly
    con.connect_chain(&[red[3], heart_vessel]);
    con.connect_chain(&[heart_vessel, blue[0]]);

    HeartJunctions {
        red_in: con.junction(*red[0], PortTag::A).unwrap(),
        red_out: con.junction(*red[3], PortTag::B).unwrap(),
        blue_in: con.junction(*blue[0], PortTag::A).unwrap(),
        blue_out: con.junction(*blue[3], PortTag::B).unwrap(),
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
        dep.dep::<TimeMocca>();
        dep.dep::<BodyPartMocca>();
        dep.dep::<BloodMocca>();
        dep.dep::<FlowSimMocca>();
    }

    fn register_components(world: &World) {
        world.component::<HeartConfig>();
        world.component::<HeartBeatState>();
        world.component::<HeartRateBpm>();
        world.component::<HeartChambers>();
        world.component::<HeartStats>();
    }

    fn start(world: &World) -> Self {
        HeartRateBpm::setup(world);

        world.add(HeartConfig {});

        Self
    }

    fn step(&mut self, world: &World) {
        // Check if the heart beats
        world
            .query::<(&Time, &HeartRateBpm, &mut HeartBeatState)>()
            .singleton_at(0)
            .build()
            .each(|(t, rate, state)| {
                state.cycle.set_target_bpm(**rate);
                state.cycle.step(t.sim_dt_f64());
            });

        // Apply pressure to chambers
        world
            .query::<(&Time, &HeartChambers, &HeartBeatState)>()
            .singleton_at(0)
            .build()
            .each_entity(|e, (_t, chambers, state)| {
                let world = e.world();

                let red_atrium = world.entity_from_id(chambers.red_atrium);
                let blue_atrium = world.entity_from_id(chambers.blue_atrium);
                let red_ventricle = world.entity_from_id(chambers.red_ventricle);
                let blue_ventricle = world.entity_from_id(chambers.blue_ventricle);

                match state.cycle.stage() {
                    (CardiacCycleStage::DiastolePhase1, _) => {
                        red_atrium.set(ExternalPipePressure::ubiquous(0.));
                        blue_atrium.set(ExternalPipePressure::ubiquous(0.));
                        red_ventricle.set(ExternalPipePressure::ubiquous(0.));
                        blue_ventricle.set(ExternalPipePressure::ubiquous(0.));
                    }
                    (CardiacCycleStage::ArterialSystole, q) => {
                        let a = attack(q);
                        red_atrium.set(ExternalPipePressure::ubiquous(-1_000. * a));
                        blue_atrium.set(ExternalPipePressure::ubiquous(-1_000. * a));
                        red_ventricle.set(ExternalPipePressure::ubiquous(0.));
                        blue_ventricle.set(ExternalPipePressure::ubiquous(0.));
                    }
                    (CardiacCycleStage::Systole, q) => {
                        let a = attack(q);
                        red_atrium.set(ExternalPipePressure::ubiquous(0.));
                        blue_atrium.set(ExternalPipePressure::ubiquous(0.));
                        red_ventricle.set(ExternalPipePressure::ubiquous(-16_000. * a));
                        blue_ventricle.set(ExternalPipePressure::ubiquous(-3_300. * a));
                    }
                }
            });

        // Update heart statistics
        world
            .query::<(&Time, &HeartBeatState, &mut HeartStats)>()
            .singleton_at(0)
            .build()
            .each(|(t, state, stats)| {
                let beat = state.cycle.beat();
                stats.beat = beat;
                stats.heart_rate.step(t.sim_dt_f64(), beat);
                stats.monitor.step(beat);
                stats.stage = state.cycle.stage().0;
                stats.stage_progress = state.cycle.stage().1;
            });
    }
}

fn attack(q: f64) -> f64 {
    use core::f64::consts::PI;
    (q * PI).sin().sqrt()
}
