use approx::{assert_abs_diff_eq, assert_relative_eq};
use flecs_ecs::prelude::*;
use gosim::{
    setup_flow_net, EntityBuilder, ExternalPipePressure, FlowDirection, FlowNetModule,
    JunctionBuilder, Pipe, PipeBuilder, PipeFlowState, PipeGeometry, PortAJunction, PortBJunction,
    PortTag, PumpBuilder, PumpDef, PumpPowerFactor, TestRunner, Time, ValveBuilder, ValveDef,
    ValveKind,
};

// Four pipes connected in a line: pipe1 - pipe2 - pipe3 - pipe4
// External pressure on first pipe.
struct FlowNetLine;

impl TestRunner for FlowNetLine {
    type Config = ();

    fn init(_: (), world: &World) -> Self {
        world.import::<FlowNetModule>();

        let pipe_builder = PipeBuilder {
            geometry: &PipeGeometry {
                radius: 0.010,
                length: 1.000,
                wall_thickness: 0.001,
                youngs_modulus: 1_000_000.0,
                count: 10.0,
                pressure_min: -1_000.0,
            },
            data: &(),
            target_pressure: 10_000.0,
        };

        let junc_builder = JunctionBuilder::<()>::default();

        let pipe1 = pipe_builder
            .new_named(&world, "pipe1")
            .set(ExternalPipePressure(10000.));
        let junc1 = junc_builder.new_named(&world, "junc1");
        let pipe2 = pipe_builder.new_named(&world, "pipe2");
        let junc2 = junc_builder.new_named(&world, "junc2");
        let pipe3 = pipe_builder.new_named(&world, "pipe3");
        let junc3 = junc_builder.new_named(&world, "junc3");
        let pipe4 = pipe_builder.new_named(&world, "pipe4");

        pipe1.add((PortBJunction, junc1));
        pipe2.add((PortAJunction, junc1));
        pipe2.add((PortBJunction, junc2));
        pipe3.add((PortAJunction, junc2));
        pipe3.add((PortBJunction, junc3));
        pipe4.add((PortAJunction, junc3));

        setup_flow_net::<()>(&world);

        FlowNetLine
    }

    fn fini(self, world: &World) {
        print_pipe_status(&world);
    }

    fn fini_test(&mut self, world: &World) {
        world.query::<(&PipeFlowState,)>().build().each(|(state,)| {
            assert_relative_eq!(
                state.intrinsic_port_pressure(PortTag::A),
                11333.8,
                max_relative = 1e-5
            );
            assert_relative_eq!(
                state.intrinsic_port_pressure(PortTag::B),
                11333.8,
                max_relative = 1e-5
            );
            assert_abs_diff_eq!(state.storage_flow(), 0., epsilon = 1e-4);
            assert_abs_diff_eq!(state.through_flow(), 0., epsilon = 1e-4,);
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
                pipe.volume(),
                geo.nominal_volume(),
                state.storage_flow(),
                state.through_flow(),
                state.intrinsic_port_pressure(PortTag::A),
                state.intrinsic_port_pressure(PortTag::B),
            );
        });
}

#[test]
fn test_flow_net_line() {
    FlowNetLine::run_test((), 2000);
}

#[test]
fn test_flow_net_with_pump_a() {
    FlowNetPump::run_test(PortTag::A, 2000);
}

#[test]
fn test_flow_net_with_pump_b() {
    FlowNetPump::run_test(PortTag::B, 2000);
}

// Four pipes connected in a circle: (pipe4) - pump - pipe2 - pipe3 - pipe4 - (pump)
// The first pipe is a pump.
struct FlowNetPump {
    pump_outlet: PortTag,
    pump: Entity,
    pump_def: PumpDef,
}

impl TestRunner for FlowNetPump {
    type Config = PortTag;

    fn init(pump_outlet: PortTag, world: &World) -> Self {
        world.import::<FlowNetModule>();

        let pipe_builder = PipeBuilder {
            geometry: &PipeGeometry {
                radius: 0.010,
                length: 1.000,
                wall_thickness: 0.001,
                youngs_modulus: 1_000_000.0,
                count: 10.0,
                pressure_min: -1_000.0,
            },
            data: &(),
            target_pressure: 10_000.0,
        };

        let junc_builder = JunctionBuilder::<()>::default();

        let pump_def = PumpDef {
            max_pressure_differential: 10_000.,
            max_flow: 0.2,
            flow_pressure_curve_exponential: 1.5,
            outlet: pump_outlet,
        };
        let pump_builder = PumpBuilder { def: &pump_def };

        let pump = pump_builder.build(&world, pipe_builder.new_named(&world, "pump"));
        let junc1 = junc_builder.new_named(&world, "junc1");
        let pipe2 = pipe_builder.new_named(&world, "pipe2");
        let junc2 = junc_builder.new_named(&world, "junc2");
        let pipe3 = pipe_builder.new_named(&world, "pipe3");
        let junc3 = junc_builder.new_named(&world, "junc3");
        let pipe4 = pipe_builder.new_named(&world, "pipe4");
        let junc4 = junc_builder.new_named(&world, "junc4");

        pump.add((PortAJunction, junc4));
        pump.add((PortBJunction, junc1));
        pipe2.add((PortAJunction, junc1));
        pipe2.add((PortBJunction, junc2));
        pipe3.add((PortAJunction, junc2));
        pipe3.add((PortBJunction, junc3));
        pipe4.add((PortAJunction, junc3));
        pipe4.add((PortBJunction, junc4));

        setup_flow_net::<()>(&world);

        FlowNetPump {
            pump_outlet,
            pump: *pump,
            pump_def,
        }
    }

    fn fini(self, world: &World) {
        print_pipe_status(&world);
    }

    fn fini_test(&mut self, world: &World) {
        let expected_flow = 0.075351
            * match self.pump_outlet {
                PortTag::A => -1.,
                PortTag::B => 1.,
            };

        world
            .query::<(&PipeFlowState,)>()
            .build()
            .each_entity(|e, (state,)| {
                assert_abs_diff_eq!(state.storage_flow(), 0., epsilon = 1e-4);
                assert_abs_diff_eq!(state.through_flow(), expected_flow, epsilon = 1e-4,);

                if e == self.pump {
                    assert_relative_eq!(
                        state.intrinsic_pressure_differential(FlowDirection::AtoB),
                        self.pump_def.effective_pressure_differential(expected_flow),
                        max_relative = 1e-4
                    );
                } else {
                    assert_abs_diff_eq!(
                        state.intrinsic_port_pressure(PortTag::A),
                        state.intrinsic_port_pressure(PortTag::B),
                        epsilon = 1e-4
                    );
                }
            });
    }
}

// 10 pipes connected in a circle: (pipe4) - pipe1 - pipe2 - pipe3 - pipe4 - .. - (pipe1)
//                                             ^ pump
// The first pipe has a pump moves liquid right.
// The pump is operated intermittantly.
// The pump also has a valve which prevents backflow.
struct FlowNetValve {
    pump: Entity,
    pipe1: Entity,
}

impl TestRunner for FlowNetValve {
    type Config = ();

    fn init(_: (), world: &World) -> Self {
        world.import::<FlowNetModule>();

        let pipe_builder = PipeBuilder {
            geometry: &PipeGeometry {
                radius: 0.020,
                length: 1.000,
                wall_thickness: 0.001,
                youngs_modulus: 1_000_000.0,
                count: 10.0,
                pressure_min: -1_000.0,
            },
            data: &(),
            target_pressure: 10_000.0,
        };

        let junc_builder = JunctionBuilder::<()>::default();

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
            },
        };

        let pump = pump_builder.build(&world, pipe_builder.new_named(&world, "pump"));
        valve_builder.build(&world, pump);

        let junc0 = junc_builder.new_named(&world, format!("junc0"));

        pump.add((PortBJunction, junc0));

        let mut pipe1 = pump;

        let mut prev_junc = junc0;
        for i in 1..10 {
            let pipe = pipe_builder.new_named(&world, format!("pipe{i}"));
            let junc = junc_builder.new_named(&world, format!("junc{i}"));

            pipe.add((PortAJunction, prev_junc));
            pipe.add((PortBJunction, junc));
            prev_junc = junc;

            if i == 1 {
                pipe1 = pipe;
            }
            if i == 9 {
                pump.add((PortAJunction, junc));
            }
        }

        setup_flow_net::<()>(&world);

        FlowNetValve {
            pump: *pump,
            pipe1: *pipe1,
        }
    }

    fn step(&mut self, world: &World) {
        // Step function profile for pump power
        let time = world.get::<&Time>(|t| t.sim_time.as_secs_f64());
        let ppf = ((time as u64 + 1) % 2) as f64;
        world.entity_from_id(self.pump).set(PumpPowerFactor(ppf));
    }

    fn fini(self, world: &World) {
        print_pipe_status(&world);
    }

    fn fini_test(&mut self, world: &World) {
        world
            .query::<(&PipeFlowState,)>()
            .build()
            .each_entity(|e, (state,)| {
                // No throughflow in pump as valve prevents backflow
                if e == self.pump {
                    assert_abs_diff_eq!(state.storage_flow(), 0.24763, epsilon = 1e-4);
                    assert_abs_diff_eq!(state.through_flow(), 0., epsilon = 1e-4,);
                }

                // No throughflow in pipe1 as valve prevents backflow
                if e == self.pipe1 {
                    assert_abs_diff_eq!(state.storage_flow(), -0.25538, epsilon = 1e-4);
                    assert_abs_diff_eq!(state.through_flow(), 0., epsilon = 1e-4,);
                }

                // No pipe has a intrinsic pressure differential
                assert_relative_eq!(
                    state.intrinsic_pressure_differential(FlowDirection::AtoB),
                    0.,
                    max_relative = 1e-4
                );
            });
    }
}

#[test]
fn test_flow_net_with_valve() {
    FlowNetValve::run_test((), 2000);
}
