use flowsim::{
    models::{Bundle, ElasticTube, HoopTubePressureModel, TurbulentFlowModel},
    FlowNet, FlowNetSolver, Fluid, FluidChunk, PipeBundle, PipeVessel, PortTag,
};
use gems::{pressure_from_atm, DENSITY_BLOOD, VISCOSITY_BLOOD};

use gems::Cylinder;

fn standard_pipe(count: f64, target_pressure: f64) -> PipeBundle {
    let cylinder = Cylinder {
        radius: 0.010,
        length: 1.000,
    };

    let tube = ElasticTube {
        shape: cylinder.clone(),
        wall_thickness: 0.003,
        youngs_modulus: 500_000.,
    };

    let pressure_model = HoopTubePressureModel::new(tube, -1000.0);
    let volume = pressure_model.volume(target_pressure).unwrap() * count;

    let mut pipe = PipeBundle {
        shape: cylinder.clone(),
        vessel: PipeVessel::default(),
        port_velocity: [0., 0.],
        external_port_pressure: [0., 0.],
        elasticity_pressure_model: Bundle {
            model: pressure_model,
            count,
        },
        flow_model: Bundle {
            model: TurbulentFlowModel::new(cylinder, DENSITY_BLOOD, VISCOSITY_BLOOD, 1.0),
            count,
        },
        ground_angle: 0.,
        darcy_factor: 64. / 2000., // e.g. 64/Re
        dampening: 0.0,
    };

    pipe.vessel
        .fill(PortTag::A, FluidChunk::from_fluid(Fluid::blood(volume)));

    pipe
}

fn solve(net: &mut FlowNet) {
    let mut solver = FlowNetSolver::new();

    for i in 1..=200 {
        // println!("ITERATION {i}");
        solver.step(i, net, 0.050);
        // solver
        //     .write_pipes_to_csv(&format!("I:/Ikabur/gos/tmp/solver_{i:05}.csv"))
        //     .unwrap();
    }
}

#[test]
fn test_pipe_chain() {
    // Creates 10 pipes in a chain. The first pipe is over-pressured.

    let mut flownet = FlowNet::new();

    let mut pipes = Vec::new();
    for i in 0..10 {
        let pressure = pressure_from_atm(if i == 0 { 0.1 } else { 0.0 });
        let pipe = standard_pipe(1.0, pressure);
        pipes.push(flownet.add_pipe(pipe));
    }

    for w in pipes.windows(2) {
        flownet.connect((w[0], PortTag::B), (w[1], PortTag::A));
    }

    solve(&mut flownet);
}

#[test]
fn test_pipe_count_imbalance() {
    // Creates two connected pipes. The first one is over-pressured. The second one has count 10.

    let mut flownet = FlowNet::new();

    let pipe_1 = standard_pipe(1., pressure_from_atm(0.1));
    let pipe_2 = standard_pipe(10., pressure_from_atm(0.0));

    let p1 = flownet.add_pipe(pipe_1);
    let p2 = flownet.add_pipe(pipe_2);
    flownet.connect((p1, PortTag::B), (p2, PortTag::A));

    solve(&mut flownet);
}
