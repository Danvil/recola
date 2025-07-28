use crate::{
    create_blood_vessel, create_blood_vessel_aux, create_tissue, setup_flow_net, stat_component,
    BloodProperties, BloodVessel, BodyPartModule, FlecsQueryRelationHelpers, FlowNetModule,
    FluidFlowLink, IsPumpActive, PipeStats, PumpIntakePipe, PumpOutputPipe, PumpStats, PumpVolume,
    PumpVolumeBase, PumpVolumeMods, Time, TimeModule,
};
use flecs_ecs::prelude::*;
use gems::{BeatEma, RateEma};

/// The heart is an organ which pumps blood through the body.
#[derive(Component)]
pub struct HeartModule;

#[derive(Component)]
pub struct HeartConfig {}

/// A human heart usually has two halfes - one from body to lungs and one from lungs to body.
#[derive(Component)]
pub struct HeartChamberOf;

/// Internal state used for computation of heart beat
#[derive(Component, Clone, Default, Debug)]
struct HeartBeatState {
    time_to_beat: f64,
}

stat_component!(
    /// Heart rate stored as beats per minute
    HeartRateBpm
);

/// Internal tag used to signal heart beats
#[derive(Component)]
pub struct HeartBeat;

/// Statistics measured for heart
#[derive(Component, Default, Clone)]
pub struct HeartStats {
    pub heart_rate: BeatEma,
    pub monitor: HeartRateMonitor,
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
pub fn create_heart<'a>(world: &'a World, entity: EntityView<'a>) -> HeartSlots<'a> {
    let heart = entity
        .set(HeartBeatState::default())
        .set(HeartRateBpm::new(60.))
        .set(HeartRateBpmBase::new(60.))
        .set(HeartRateBpmMods::default())
        .set(HeartStats {
            heart_rate: BeatEma::from_halflife(5.),
            monitor: HeartRateMonitor::with_len(40),
            ..Default::default()
        })
        .set(PumpVolume::new(0.07))
        .set(PumpVolumeBase::new(0.07))
        .set(PumpVolumeMods::default())
        .set(PumpStats {
            flow: RateEma::from_halflife(3.),
            ..Default::default()
        });

    create_tissue(heart);

    // The heart is a body part which needs blood itself
    let heart_vessel = create_blood_vessel(world, heart, 0.050);

    let heart_chamber_fn = |tag, in_vessel_stats, out_vessel_stats| {
        let intake = create_blood_vessel_aux(
            world,
            world.entity_named(&format!("{tag}_in")).child_of(heart),
            in_vessel_stats,
        );

        let output = create_blood_vessel_aux(
            world,
            world.entity_named(&format!("{tag}_out")).child_of(heart),
            out_vessel_stats,
        );

        let chamber = world
            .entity_named(&format!("{tag}_chamber"))
            .child_of(heart)
            .add((PumpIntakePipe, intake))
            .add((PumpOutputPipe, output))
            .add((HeartChamberOf, heart));

        (intake, chamber, output)
    };

    let pumonary_vene_vessels = PipeStats {
        radius: 0.004,
        length: 0.050,
        wall_thickness: 0.0005,
        youngs_modulus: 15_000.,
        count: 100.,
        pressure_min: -5_000.,
    };
    let aorta_vessel = PipeStats {
        radius: 0.0125,
        length: 0.300,
        wall_thickness: 0.002,
        youngs_modulus: 400_000.,
        count: 1.,
        pressure_min: -1_000.,
    };

    let (red_in, _red, red_out) = heart_chamber_fn("red", pumonary_vene_vessels, aorta_vessel);

    let systemic_vein_collective = PipeStats {
        radius: 0.008,
        length: 0.100,
        wall_thickness: 0.0007,
        youngs_modulus: 12_000.0,
        count: 10.0, // SVC, IVC, + 1 major tributary
        pressure_min: -5000.0,
    };
    let pulmonary_artery_outtake = PipeStats {
        radius: 0.009,
        length: 0.250,
        wall_thickness: 0.0015,
        youngs_modulus: 300_000.0,
        count: 2.0, // LPA + RPA
        pressure_min: -1000.0,
    };
    let (blue_in, _blue, blue_out) =
        heart_chamber_fn("blue", systemic_vein_collective, pulmonary_artery_outtake);

    // Blood flows from red_out to heart to blue_in
    red_out.add((FluidFlowLink, heart_vessel));
    heart_vessel.add((FluidFlowLink, blue_in));

    HeartSlots {
        heart,
        red_in,
        red_out,
        blue_in,
        blue_out,
    }
}

#[derive(Component, Clone)]
pub struct HeartSlots<'a> {
    pub heart: EntityView<'a>,
    pub red_in: EntityView<'a>,
    pub red_out: EntityView<'a>,
    pub blue_in: EntityView<'a>,
    pub blue_out: EntityView<'a>,
}

impl Module for HeartModule {
    fn module(world: &World) {
        world.module::<HeartModule>("HeartModule");

        world.import::<TimeModule>();
        world.import::<BodyPartModule>();
        world.import::<FlowNetModule>();

        world.component::<HeartConfig>();

        world
            .component::<HeartChamberOf>()
            .add_trait::<flecs::Exclusive>();

        world.component::<HeartBeatState>();

        HeartRateBpm::setup(world);

        setup_flow_net::<BloodProperties>(world);

        world.add(HeartConfig {});

        // Check if the heart beats
        world
            .system::<(&Time, &HeartRateBpm, &mut HeartBeatState)>()
            .singleton_at(0)
            .each_entity(|e, (t, rate, state)| {
                let dt = 60. / *rate;
                state.time_to_beat += t.sim_dt_f64();
                if state.time_to_beat >= dt {
                    state.time_to_beat -= dt;
                    e.add(HeartBeat);
                    e.add(IsPumpActive);
                } else {
                    e.remove(HeartBeat);
                    e.remove(IsPumpActive);
                }
            });

        // Compute statistics
        world
            .system::<(&Time, &mut HeartStats)>()
            .singleton_at(0)
            .each_entity(|e, (t, stats)| {
                let beat = e.has(HeartBeat);
                stats.heart_rate.step(t.sim_dt_f64(), beat);
                stats.monitor.step(beat);
            });
    }
}
