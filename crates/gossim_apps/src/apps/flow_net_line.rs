use crate::TestRunner;
use approx::{assert_abs_diff_eq, assert_relative_eq};
use flecs_ecs::prelude::*;
use gosim::{
    setup_flow_net, volume_to_liters, ElasticTubeBundle, EntityBuilder, ExternalPipePressure,
    FlowNetModule, Pipe, PipeBuilder, PipeConnectionHelper, PipeFlowState, PipeGeometry, PortTag,
};

// Four pipes connected in a line: pipe1 - pipe2 - pipe3 - pipe4
// External pressure on first pipe.
pub struct FlowNetLine;

impl TestRunner for FlowNetLine {
    type Config = ();

    fn init(_: (), world: &World) -> Self {
        world.import::<FlowNetModule>();
        setup_flow_net::<()>(&world);

        let pipe_builder = PipeBuilder {
            geometry: &PipeGeometry {
                tubes: ElasticTubeBundle {
                    radius: 0.010,
                    length: 1.000,
                    wall_thickness: 0.001,
                    youngs_modulus: 1_000_000.0,
                    count: 10.0,
                },
                collapse_pressure: -1_000.0,
                conductance_factor: 0.1,
            },
            data: &(),
            target_pressure: 10_000.0,
        };

        let pipe1 = pipe_builder
            .new_named(&world, "pipe1")
            .set(ExternalPipePressure(10_000.));
        let pipe2 = pipe_builder.new_named(&world, "pipe2");
        let pipe3 = pipe_builder.new_named(&world, "pipe3");
        let pipe4 = pipe_builder.new_named(&world, "pipe4");

        let mut conn = PipeConnectionHelper::<()>::default();
        conn.connect_chain(&[pipe1, pipe2, pipe3, pipe4]);

        FlowNetLine
    }

    fn debug_info(&mut self, world: &World) {
        print_pipe_status(&world);
    }

    fn fini_test(&mut self, world: &World) {
        world.query::<(&PipeFlowState,)>().build().each(|(state,)| {
            assert_relative_eq!(
                state.pipe_pressure(PortTag::A),
                12210.3566,
                max_relative = 1e-5
            );
            assert_relative_eq!(
                state.pipe_pressure(PortTag::B),
                12210.3566,
                max_relative = 1e-5
            );
            assert_abs_diff_eq!(state.storage_flow(), 0., epsilon = 1e-6);
            assert_abs_diff_eq!(state.through_flow(), 0., epsilon = 1e-6,);
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
