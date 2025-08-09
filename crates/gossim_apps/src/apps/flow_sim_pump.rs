use crate::TestRunner;
use approx::{assert_abs_diff_eq, assert_relative_eq};
use flecs_ecs::prelude::*;
use gosim::{
    setup_flow_net, volume_from_liters, volume_to_liters, ElasticTubeBundle, EntityBuilder,
    FlowDirection, FlowNetModule, Pipe, PipeBuilder, PipeConnectionHelper, PipeFlowState,
    PipeGeometry, PortTag, PumpBuilder, PumpDef,
};

// Four pipes connected in a circle: (pipe4) - pump - pipe2 - pipe3 - pipe4 - (pump)
// The first pipe is a pump.
pub struct FlowNetPump {
    pump_outlet: PortTag,
    pump: Entity,
    pump_def: PumpDef,
}

impl TestRunner for FlowNetPump {
    type Config = PortTag;

    fn init(pump_outlet: PortTag, world: &World) -> Self {
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

        let pump_def = PumpDef {
            max_pressure_differential: 10_000.,
            max_flow: 0.2,
            flow_pressure_curve_exponential: 1.5,
            outlet: pump_outlet,
        };
        let pump_builder = PumpBuilder { def: &pump_def };

        let pump = pump_builder.build(&world, pipe_builder.new_named(&world, "pump"));
        let pipe2 = pipe_builder.new_named(&world, "pipe2");
        let pipe3 = pipe_builder.new_named(&world, "pipe3");
        let pipe4 = pipe_builder.new_named(&world, "pipe4");

        let mut conn = PipeConnectionHelper::<()>::default();
        conn.connect_loop(&[pump, pipe2, pipe3, pipe4]);

        FlowNetPump {
            pump_outlet,
            pump: *pump,
            pump_def,
        }
    }

    fn debug_info(&mut self, world: &World) {
        print_pipe_status(&world);
    }

    fn fini_test(&mut self, world: &World) {
        let expected_flow = volume_from_liters(0.173937)
            * match self.pump_outlet {
                PortTag::A => -1.,
                PortTag::B => 1.,
            };

        world
            .query::<(&PipeFlowState,)>()
            .build()
            .each_entity(|e, (state,)| {
                assert_abs_diff_eq!(state.storage_flow(), 0., epsilon = 1e-6);
                assert_abs_diff_eq!(state.through_flow(), expected_flow, epsilon = 1e-6,);

                if e == self.pump {
                    assert_relative_eq!(
                        state.pipe_pressure_differential(FlowDirection::AtoB),
                        self.pump_def.effective_pressure_differential(expected_flow),
                        max_relative = 1e-4
                    );
                } else {
                    assert_abs_diff_eq!(
                        state.pipe_pressure(PortTag::A),
                        state.pipe_pressure(PortTag::B),
                        epsilon = 1e-4
                    );
                }
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
