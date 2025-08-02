use crate::TestRunner;
use approx::{assert_abs_diff_eq, assert_relative_eq};
use flecs_ecs::prelude::*;
use gosim::{
    setup_flow_net, volume_to_liters, ElasticTubeBundle, EntityBuilder, FlowDirection,
    FlowNetModule, Pipe, PipeBuilder, PipeConnectionHelper, PipeFlowState, PipeGeometry, PortTag,
    PumpBuilder, PumpDef, PumpPowerFactor, Time, ValveBuilder, ValveDef, ValveKind,
};

// 10 pipes connected in a circle: (pipe9) - pump - pipe1 - pipe2 - pipe3 - .. - (pump)
// The first pipe has a pump moves liquid right.
// The pump is operated intermittantly.
// The pump also has a valve which prevents backflow.
pub struct FlowNetValve {
    pump: Entity,
    pipe1: Entity,
}

impl TestRunner for FlowNetValve {
    type Config = ();

    fn init(_: (), world: &World) -> Self {
        world.import::<FlowNetModule>();
        setup_flow_net::<()>(&world);

        let pipe_builder = PipeBuilder {
            geometry: &PipeGeometry {
                tubes: ElasticTubeBundle {
                    radius: 0.005,
                    length: 1.000,
                    wall_thickness: 0.001,
                    youngs_modulus: 500_000.0,
                    count: 10.0,
                },
                collapse_pressure: -1_000.0,
                conductance_factor: 1.0,
            },
            data: &(),
            target_pressure: 10_000.0,
        };

        let pump_builder = PumpBuilder {
            def: &PumpDef {
                max_pressure_differential: 10_000.,
                max_flow: 1.0,
                flow_pressure_curve_exponential: 1.5,
                outlet: PortTag::B,
            },
        };

        let valve_builder = ValveBuilder {
            def: &ValveDef {
                conductance_factor_closed: 0.,
                kind: ValveKind::Throughflow(FlowDirection::AtoB),
                hysteresis: 0.01,
            },
        };

        let pump = pump_builder.build(&world, pipe_builder.new_named(&world, "pump"));
        valve_builder.build(&world, pump);

        let pipes: Vec<EntityView> = std::iter::once(pump)
            .chain((1..10).map(|i| pipe_builder.new_named(&world, format!("pipe{i}"))))
            .collect();

        let mut conn = PipeConnectionHelper::<()>::default();
        conn.connect_loop(&pipes);

        FlowNetValve {
            pump: *pump,
            pipe1: *pipes[1],
        }
    }

    fn step(&mut self, world: &World) {
        // Step function profile for pump power
        let time = world.get::<&Time>(|t| t.sim_time.as_secs_f64());
        let ppf = ((time as u64 + 1) % 2) as f64;
        world.entity_from_id(self.pump).set(PumpPowerFactor(ppf));
    }

    fn debug_info(&mut self, world: &World) {
        print_pipe_status(&world);
    }

    fn fini_test(&mut self, world: &World) {
        world
            .query::<(&PipeFlowState,)>()
            .build()
            .each_entity(|e, (state,)| {
                if e == self.pump {
                    // pump is under-pressured and filles from pipe9
                    assert!(state.storage_flow() > 0.);

                    // No throughflow in pump as valve prevents backflow
                    assert_abs_diff_eq!(state.through_flow(), 0., epsilon = 1e-6,);
                }

                if e == self.pipe1 {
                    // pipe1 is overpressured and empties into pipe2
                    assert!(state.storage_flow() < 0.);

                    // No throughflow in pipe1 as valve prevents backflow
                    assert_abs_diff_eq!(state.through_flow(), 0., epsilon = 1e-6,);
                }

                // No pipe has a intrinsic pressure differential
                assert_relative_eq!(
                    state.pipe_pressure_differential(FlowDirection::AtoB),
                    0.,
                    max_relative = 1e-4
                );
            });
    }
}

fn print_pipe_status(world: &World) {
    world
        .query::<(&Pipe<()>, &PipeGeometry, &PipeFlowState)>()
        .build()
        .each_entity(|e, (pipe, geo, state)| {
            println!(
                "{:<7}: V: {:.5}, V0: {:.5}, dQ: {:8.5}, Q: {:8.5}, Pa: {:7.1}, Pb: {:7.1}",
                e.name(),
                volume_to_liters(pipe.volume()),
                volume_to_liters(geo.tubes.nominal_volume()),
                volume_to_liters(state.storage_flow()),
                volume_to_liters(state.through_flow()),
                state.pipe_pressure(PortTag::A),
                state.pipe_pressure(PortTag::B),
            );
            // println!("{state:?}");
        });
}
