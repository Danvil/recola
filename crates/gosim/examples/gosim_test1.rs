use flecs_ecs::prelude::*;
use gosim::*;

fn main() {
    env_logger::init();

    log::info!("Game of Stonks - Simulation");

    let world = World::new();

    // Optional, gather statistics for explorer
    world.import::<stats::Stats>();

    // Creates REST server on default port (27750)
    world.set(flecs::rest::Rest::default());

    world.import::<WaterfrontModule>();

    loop {
        if !world.progress() {
            break;
        }

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

        // world
        //     .query::<(&HeartRateBpm,)>()
        //     .build()
        //     .each_entity(|e, (bpm,)| {
        //         println!("{:?}: {} BPM, beat: {}", e.name(), **bpm, e.has(HeartBeat));
        //     });

        world
            .query::<(&HeartRateBpm,)>()
            .build()
            .each_entity(|e, (_bpm,)| {
                if e.has(HeartBeat) {
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

        world
            .query::<(Option<&BodyPart>, &BloodStats, &BloodVessel, &PipeFlowState)>()
            .build()
            .each_entity(|e, (part, blood, pipe, state)| {
                println!(
                    "Blood {:<16} [{:>12}]: P: {:4.0} mmHg, V: {:5.2} L, SO2: {:5.1} %, PO2: {:5.0} mmHg",
                    e.name(),
                    part.map_or_else(|| String::new(), |x| format!("{x:?}")),
                    state.pressure() / 133.322,
                    pipe.volume(),
                    100. * blood.so2,
                    blood.po2
                );
            });

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

        std::thread::sleep(std::time::Duration::from_secs_f32(0.050));
    }

    log::info!("terminated.");
}
