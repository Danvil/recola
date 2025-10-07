use flowsim::{FluidComposition, PortMap, models::ElasticTube};
use gems::{Cylinder, VolumeModel, volume_to_liters};
use gosim::{
    EntityBuilder, ExternalPipePressure, FlowNetPipeDef, FlowNetPipeState, FlowNetPipeVessel,
    FlowSimMocca, PipeBuilder, PipeConnectionHelper, Time, TimeMocca,
};
use simplecs::prelude::*;

// Four pipes connected in a line: pipe1 - pipe2 - pipe3 - pipe4
// External pressure on first pipe.
pub struct FlowSimPipeChainMocca;

impl Mocca for FlowSimPipeChainMocca {
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
                youngs_modulus: 1_000_000.0,
            },
            strand_count: 10.,
            fluid: FluidComposition::water(1.),
            target_pressure: 2_000.0,
        };

        let pipe1 = pipe_builder
            .new_named(world, "pipe1")
            .set(ExternalPipePressure(PortMap::from_array([0., -10_000.])));
        let pipe2 = pipe_builder.new_named(world, "pipe2");
        let pipe3 = pipe_builder.new_named(world, "pipe3");
        let pipe4 = pipe_builder.new_named(world, "pipe4");

        let mut conn = PipeConnectionHelper::default();
        conn.connect_chain(&[pipe1, pipe2, pipe3, pipe4]);

        Self
    }

    fn step(&mut self, world: &World) {
        let step = world.get::<&Time>(|t| t.frame_count);
        println!("ITERATION {step}");
        print_pipe_status(&world);
    }

    fn fini_test(&mut self, world: &World) {
        world
            .query::<(&FlowNetPipeVessel, &FlowNetPipeState)>()
            .build()
            .each(|(_vessel, state)| {
                // assert_abs_diff_eq!(vessel.0.volume(), 1.70497, epsilon = 1e-4);
                // assert_relative_eq!(
                //     state.pipe_pressure(PortTag::A),
                //     12210.3566,
                //     max_relative = 1e-5
                // );
                // assert_relative_eq!(
                //     state.pipe_pressure(PortTag::B),
                //     12210.3566,
                //     max_relative = 1e-5
                // );
                assert!(state.0.inflow_velocity().abs() < 0.005);
                assert!(state.0.throughflow_velocity().abs() < 0.005);
            });
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
