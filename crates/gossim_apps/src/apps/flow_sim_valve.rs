use approx::{assert_abs_diff_eq, assert_relative_eq};
use flecs_ecs::prelude::*;
use flowsim::{models::ElasticTube, FluidComposition, PortMap};
use gems::{volume_to_liters, Cylinder, VolumeModel};
use gosim::{
    EntityBuilder, ExternalPipePressure, FlowDirection, FlowNetPipeDef, FlowNetPipeState,
    FlowNetPipeVessel, FlowSimMocca, PipeBuilder, PipeConnectionHelper, PipeFlowState, Time,
    TimeMocca, ValveBuilder, ValveDef, ValveKind,
};
use mocca::{Mocca, MoccaDeps};

// 10 pipes connected in a circle: (pipe9) - pump - pipe1 - pipe2 - pipe3 - .. - (pump)
// The first pipe has a pump which periodically applies external pressure.
// The pump also has a valve which prevents backflow.
pub struct FlowSimValveMocca {
    pump: Entity,
    pipe1: Entity,
}

const PIPE_COUNT: usize = 10;

impl Mocca for FlowSimValveMocca {
    fn load(mut dep: MoccaDeps) {
        dep.dep::<TimeMocca>();
        dep.dep::<FlowSimMocca>();
    }

    fn start(world: &World) -> Self {
        let pipe_builder = PipeBuilder {
            tube: ElasticTube {
                shape: Cylinder {
                    radius: 0.010,
                    length: 1.000,
                },
                wall_thickness: 0.001,
                youngs_modulus: 500_000.0,
            },
            strand_count: 10.,
            fluid: FluidComposition::water(1.),
            target_pressure: 2_000.0,
        };

        let valve_builder = ValveBuilder {
            def: &ValveDef {
                conductance_factor_closed: 0.,
                kind: ValveKind::Throughflow(FlowDirection::AtoB),
                hysteresis: 0.05,
            },
        };

        let pump = pipe_builder.build(&world, pipe_builder.new_named(&world, "pump"));
        valve_builder.build(&world, pump);

        let pipes: Vec<EntityView> = std::iter::once(pump)
            .chain((1..PIPE_COUNT).map(|i| pipe_builder.new_named(&world, format!("pipe{i}"))))
            .collect();

        let mut conn = PipeConnectionHelper::default();
        conn.connect_loop(&pipes);

        Self {
            pump: *pump,
            pipe1: *pipes[0],
        }
    }

    fn step(&mut self, world: &World) {
        let step = world.get::<&Time>(|t| t.frame_count);
        println!("ITERATION {step}");

        // Step function profile for pump power
        let time = world.get::<&Time>(|t| t.sim_time.as_secs_f64());
        let ppf = ((time as u64 + 1) % 2) as f64;
        world
            .entity_from_id(self.pump)
            .set(ExternalPipePressure(PortMap::from_array([
                0.,
                -10_000.0 * ppf,
            ])));

        print_pipe_status(&world);
    }

    fn fini_test(&mut self, _world: &World) {
        // TODO
        // world
        //     .query::<(&PipeFlowState,)>()
        //     .build()
        //     .each_entity(|e, (state,)| {
        //         if e == self.pump {
        //             // pump is under-pressured and filles from pipe9
        //             assert!(state.storage_flow() > 0.);

        //             // No throughflow in pump as valve prevents backflow
        //             assert_abs_diff_eq!(state.through_flow(), 0., epsilon = 1e-6,);
        //         }

        //         if e == self.pipe1 {
        //             // pipe1 is overpressured and empties into pipe2
        //             assert!(state.storage_flow() < 0.);

        //             // No throughflow in pipe1 as valve prevents backflow
        //             assert_abs_diff_eq!(state.through_flow(), 0., epsilon = 1e-6,);
        //         }

        //         // No pipe has a intrinsic pressure differential
        //         assert_relative_eq!(
        //             state.pressure_differential(FlowDirection::AtoB),
        //             0.,
        //             max_relative = 1e-4
        //         );
        //     });
    }
}

fn print_pipe_status(world: &World) {
    world
        .query::<(&FlowNetPipeVessel, &FlowNetPipeDef, &FlowNetPipeState)>()
        .build()
        .each_entity(|e, (pipe, geo, state)| {
            println!(
                "{:<7}: V: {:.5}, V0: {:.5}, v_in: {:8.5}, v_thr: {:8.5}",
                e.name(),
                volume_to_liters(pipe.0.volume()),
                volume_to_liters(geo.0.shape.nominal_volume()),
                state.0.inflow_velocity(),
                state.0.throughflow_velocity(),
            );
            // println!("{state:?}");
        });
}
