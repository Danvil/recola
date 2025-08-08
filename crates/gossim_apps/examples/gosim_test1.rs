use flecs_ecs::prelude::*;
use gosim::*;
use mocca::{Mocca, MoccaDeps, MoccaRunSettings, MoccaRunner};

fn main() {
    MoccaRunner::run::<GosSimDebugMocca>(MoccaRunSettings::app());
}

struct GosSimDebugMocca;

impl Mocca for GosSimDebugMocca {
    fn load(mut dep: MoccaDeps) {
        dep.dep::<LogMocca>();
        dep.dep::<GosSimMocca>();
    }

    fn start(_: &World) -> Self {
        log::info!("Game of Stonks - Simulation");
        Self
    }

    fn step(&mut self, world: &World) {
        print_report(world);
    }

    fn fini(&mut self, _world: &World) {
        log::info!("terminated.");
    }
}

fn print_report(_world: &World) {
    //     // world
    //     //     .query::<(&CurrentBloodOxygen, &BodyTox, &CurrentBreathingOrgan)>()
    //     //     .build()
    //     //     .each_entity(|e, (oxy, tox, organ)| {
    //     //         println!(
    //     //             "{}: oxy:{:6.03?}, tox: {:6.03?}, abs:{:6.03?}, poll:{:6.03?}",
    //     //             e.name(),
    //     //             oxy.value,
    //     //             tox.amount,
    //     //             organ.oxygen_absorption,
    //     //             organ.pollution_absorption
    //     //         );
    //     //     });

    //     // world
    //     //     .query::<()>()
    //     //     .with(PlayerTag)
    //     //     .build()
    //     //     .each_entity(|e, ()| {
    //     //         println!("{:?}", e.name());
    //     //     });

    //     // world
    //     //     .query::<(&Pipe<BloodProperties>, &PipeFlowState)>()
    //     //     .related("$this", flecs::ChildOf, "$player")
    //     //     .tagged("$player", PlayerTag)
    //     //     .build()
    //     //     .each_entity(|e, (v, state)| {
    //     //         println!(
    //     //             "{:?}: V: {:.03?} l, flow: {:.03?} ml/s",
    //     //             e.name(),
    //     //             v.volume(),
    //     //             state.flow()
    //     //         );
    //     //     });

    //     // world
    //     //     .query::<(&HeartRateBpm,)>()
    //     //     .build()
    //     //     .each_entity(|e, (bpm,)| {
    //     //         println!("{:?}: {} BPM, beat: {}", e.name(), **bpm, e.has(HeartBeat));
    //     //     });

    //     world.query::<(&HeartStats,)>().build().each(|(stats,)| {
    //         if stats.beat {
    //             println!(">>>>> BUM BUM <<<<<");
    //         }
    //     });

    //     // world
    //     //     .query::<(&BloodStats,)>()
    //     //     .with(AlveoliTag)
    //     //     .build()
    //     //     .each_entity(|e, (blood,)| {
    //     //         println!("Alveoli {:?}: SO2: {:.1}%", e.name(), 100. * blood.so2);
    //     //     });

    //     // world
    //     //     .query::<(Option<&BodyPart>, &BloodVessel)>()
    //     //     .build()
    //     //     .each_entity(|e, (part, vessel)| {
    //     //         println!(
    //     //             "Vessel {:?} [{part:?}]: frags: {}",
    //     //             e.name(),
    //     //             vessel.chunks().len()
    //     //         );
    //     //     });

    //     world
    //             .query::<(Option<&BodyPart>, &BloodStats, &BloodVessel, &PipeFlowState)>()
    //             .build()
    //             .each_entity(|e, (part, blood, pipe, state)| {
    //                 println!(
    //                     "Blood {:<16} [{:>12}]: P: {:5.1}|{:5.1} mmHg, Q: {:5.3}|{:5.3} mL/s, V:
    // {:3.0} mL, SO2: {:5.1} %, PO2: {:5.0} mmHg",                     e.name(),
    //                     part.map_or_else(|| String::new(), |x| format!("{x:?}")),
    //                     pressure_to_mm_hg(state.pipe_pressure(PortTag::A)),
    //                     pressure_to_mm_hg(state.pipe_pressure(PortTag::B)),
    //                     volume_to_milli_liters(state.flow(PortTag::A)),
    //                     volume_to_milli_liters(state.flow(PortTag::B)),
    //                     volume_to_milli_liters(pipe.volume()),
    //                     100. * blood.so2,
    //                     blood.po2
    //                 );
    //             });

    //     // world
    //     //     .query::<(Option<&BodyPart>, &Tissue, &TissueStats)>()
    //     //     .build()
    //     //     .each_entity(|e, (part, tissue, stats)| {
    //     //         println!(
    //     //             "Tissue {:?} [{part:?}]: SO2: {:0.0}%, O2 cont: {:0.0} mL/dL",
    //     //             e.name(),
    //     //             100. * stats.o2_saturation,
    //     //             100. * tissue.o2_content
    //     //         );
    //     //     });
}
