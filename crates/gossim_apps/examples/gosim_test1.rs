use flowsim::PortTag;
use gems::{pressure_to_mm_hg, volume_to_milli_liters, VolumeModel};
use gosim::*;
use simplecs::prelude::*;

fn main() {
    GosSimDebugMocca::run(MoccaRunSettings::app());
}

struct GosSimDebugMocca;

impl Mocca for GosSimDebugMocca {
    fn load(mut deps: MoccaDeps) {
        deps.depends_on::<LogMocca>();
        deps.depends_on::<GosSimMocca>();
    }

    fn start(world: &mut World) -> Self {
        log::info!("Game of Stonks - Simulation");

        // enable flow sim logging
        let cfg = world.singleton_mut::<FlowSimConfig>();
        cfg.pipe_stats_csv_path = Some("I:/Ikabur/gos/tmp/heart/".into());
        cfg.graph_topology_path = Some("I:/Ikabur/gos/tmp/heart/".into());

        Self
    }

    fn step(&mut self, world: &mut World) {
        print_report(world);
    }

    fn fini(&mut self, _world: &mut World) {
        log::info!("terminated.");
    }
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
            &FlowNetPipeDef,
            &FlowNetPipeVessel,
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
                volume_to_milli_liters(vessel.0.volume()),
                volume_to_milli_liters(def.0.shape.nominal_volume()),
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
