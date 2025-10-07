use flowsim::{
    FlowNet, FlowNetSolver, FluidDensityViscosity, PipeDef, PipeId, PipeState, PortMap, PortTag,
    models::{Bundle, ElasticTube, HoopTubePressureModel},
};
use gems::{IntMap, pressure_from_atm};

use gems::Cylinder;

fn standard_pipe(count: f64, target_pressure: f64) -> (PipeDef, PipeState) {
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

    let pipe = PipeDef {
        name: "".to_string(),
        shape: Bundle {
            model: cylinder.clone(),
            count: count,
        },
        fluid: FluidDensityViscosity::blood(),
        external_port_pressure: PortMap::from_array([0., 0.]),
        elasticity_pressure_model: Bundle {
            model: pressure_model,
            count,
        },
        ground_angle: 0.,
        darcy_factor: 64. / 2000.,
        dampening: 0.0,
        port_area_factor: PortMap::from_array([1., 1.]),
    };

    let state = PipeState {
        volume,
        velocity: PortMap::default(),
    };

    (pipe, state)
}

#[test]
fn test_flow_sim_pipe_chain() {
    // Creates 10 pipes in a chain. The first pipe is over-pressured.

    let mut net = FlowNet::new();

    let mut state = IntMap::from_count(10, |_| PipeState::default());

    let mut pipes = Vec::new();
    for i in 0..10 {
        let pressure = pressure_from_atm(if i == 0 { 0.1 } else { 0.0 });
        let (pipe, pstate) = standard_pipe(1.0, pressure);
        pipes.push(PipeId(net.pipes.insert(pipe)));
        state[i] = pstate;
    }

    for w in pipes.windows(2) {
        net.topology.connect((w[0], PortTag::B), (w[1], PortTag::A));
    }

    approx::assert_relative_eq!(state[0].volume, 0.3613256e-3, max_relative = 1e-6);
    approx::assert_relative_eq!(state[9].volume, 0.3141593e-3, max_relative = 1e-6);

    let state = FlowNetSolver::new().solve(&mut net, state, 2000, 0.050);

    approx::assert_relative_eq!(state[0].volume, 0.31887589e-3, max_relative = 1e-6);
    approx::assert_relative_eq!(state[9].volume, 0.31887591e-3, max_relative = 1e-6);
}

#[test]
fn test_flow_sim_pipe_count_imbalance() {
    // Creates two connected pipes. The first one is over-pressured. The second one has count 10.

    let mut net = FlowNet::new();

    let mut state = IntMap::from_count(2, |_| PipeState::default());

    let (pipe_1, pstate_1) = standard_pipe(1., pressure_from_atm(0.1));
    let (pipe_2, pstate_2) = standard_pipe(10., pressure_from_atm(0.0));

    state[0] = pstate_1;
    state[1] = pstate_2;

    let p1 = PipeId(net.pipes.insert(pipe_1));
    let p2 = PipeId(net.pipes.insert(pipe_2));
    net.topology.connect((p1, PortTag::B), (p2, PortTag::A));

    approx::assert_relative_eq!(state[*p1].volume, 0.3613256e-3, max_relative = 1e-6);
    approx::assert_relative_eq!(state[*p2].volume, 3.141592e-3, max_relative = 1e-6);

    let state = FlowNetSolver::new().solve(&mut net, state, 2000, 0.050);

    approx::assert_relative_eq!(state[*p1].volume, 0.3184471e-3, max_relative = 1e-6);
    approx::assert_relative_eq!(state[*p2].volume, 3.184471e-3, max_relative = 1e-6);
}
