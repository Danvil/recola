use crate::{
    models::{FlowModel, PressureModel},
    FlowNet, Pipe,
    Port::PipeOutlet,
    PortTag, ReservoirVessel,
};
use gems::{newton_root_solver, volume_to_liters, IntMap, NewtonRootSolverError};
use std::{
    error::Error,
    fs::File,
    io::{BufWriter, Write},
};

#[derive(Debug)]
pub struct FlowNetSolver {
    pipe_state: IntMap<PipeSolverState>,
    junction_state: IntMap<JunctionSolverState>,
    max_sub_steps: usize,
    max_rel_velocity: f64,
    dampening: f64,
}

#[derive(Default, Clone, Debug)]
struct PipeSolverState {
    poiseuille_conductance: f64,
    volume: f64,
    pressure: [f64; 2],
    velocity: [f64; 2],
    substep_flow: [f64; 2],
    delta_volume_substep: [f64; 2],
    flow: [f64; 2],
}

#[derive(Default, Clone, Debug)]
struct JunctionSolverState {
    vessel: ReservoirVessel,
    pressure: Option<f64>,
}

impl Default for FlowNetSolver {
    fn default() -> Self {
        Self {
            pipe_state: IntMap::default(),
            junction_state: IntMap::default(),
            max_sub_steps: 1000,
            max_rel_velocity: 0.01,
            dampening: 10e8,
        }
    }
}

impl FlowNetSolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn step(&mut self, net: &mut FlowNet, dt: f64) {
        self.sync(net);

        let min_sub_dt = dt / self.max_sub_steps as f64;

        let mut remaining_dt = dt;
        let mut step_count = 0;
        loop {
            step_count += 1;

            self.update_state(net);
            let sub_dt = self.substep(net, dt, min_sub_dt);

            remaining_dt -= sub_dt;
            if remaining_dt <= 0. {
                break;
            }

            // println!("[{step_count}] {sub_dt}/{dt}");
            // self.print_overview();
        }
        println!("took {step_count} substeps");
    }

    pub fn sync(&mut self, net: &FlowNet) {
        // prepare pipe state
        // TODO re-use memory efficiently
        self.pipe_state.clear();
        for (id, pipe) in net.pipes.iter() {
            let mut state = PipeSolverState::default();
            state.poiseuille_conductance = pipe.flow_model.poiseuille_conductance();
            self.pipe_state.set(id, state);
        }

        // prepare junction state
        // TODO re-use memory efficiently
        self.junction_state.clear();
        for (id, _junc) in net.junctions.iter() {
            self.junction_state.set(id, JunctionSolverState::default());
        }

        self.update_state(net);
    }

    fn update_state(&mut self, net: &FlowNet) {
        for (pipe_id, pipe) in net.pipes.iter() {
            let state = &mut self.pipe_state[pipe_id];

            state.volume = pipe.vessel.volume();

            let elastic_pressure = pipe.elasticity_pressure_model.pressure(state.volume);
            for pix in 0..2 {
                // println!(
                //     "{}, {}",
                //     state.flow[pix].abs(),
                //     self.dampening * state.flow[pix].abs()
                // );
                state.pressure[pix] = /*pipe.external_pressure[pix]
                    + elastic_pressure
                    + */self.dampening * state.flow[pix].abs();
            }
        }
    }

    fn substep(&mut self, net: &mut FlowNet, dt: f64, min_sub_dt: f64) -> f64 {
        // solve all junctions
        let mut max_rel_velocity: f64 = self.max_rel_velocity;

        for (junc_id, junc) in net.junctions.iter() {
            // equalization pressure
            let maybe_pressure = self
                .junction_zero_flow_pressure(junc.iter().flat_map(|port| match *port {
                    PipeOutlet { pipe_id, side } => {
                        Some((&net.pipes[*pipe_id], &self.pipe_state[*pipe_id], side))
                    }
                }))
                .ok();
            self.junction_state[junc_id].pressure = maybe_pressure;

            // compute actually exchanged volume for each port
            if let Some(junction_pressure) = maybe_pressure {
                for port in junc.iter() {
                    match *port {
                        PipeOutlet { pipe_id, side } => {
                            let pix = side.index();
                            let pipe = &net.pipes[*pipe_id];
                            let state = &mut self.pipe_state[*pipe_id];

                            let delta_pressure = junction_pressure - state.pressure[pix];
                            let flow = pipe.flow_model.flow(delta_pressure);
                            state.substep_flow[pix] = flow;

                            let velocity =
                                flow / (pipe.shape.cross_section_area() * pipe.flow_model.count);
                            state.velocity[pix] = velocity;
                            max_rel_velocity = max_rel_velocity.max(velocity / pipe.shape.length);
                        }
                    }
                }
            }
        }

        // compute substep dt
        let sub_dt = (self.max_rel_velocity / max_rel_velocity * dt).max(min_sub_dt);
        // println!("max_rel_velocity: {max_rel_velocity} => sub_dt: {sub_dt}");

        // compute flow based on sub step
        for (_, state) in self.pipe_state.iter_mut() {
            for pix in [0, 1] {
                let delta_volume =
                    (state.volume + state.substep_flow[pix] * sub_dt).max(0.) - state.volume;
                state.delta_volume_substep[pix] = delta_volume;
                state.flow[pix] += delta_volume / dt;
            }
        }

        // pipe outflow: pipe vessel -> junction vessel
        for (junc_id, junc) in net.junctions.iter() {
            let junc_state = &mut self.junction_state[junc_id];

            for port in junc.iter() {
                match *port {
                    PipeOutlet { pipe_id, side } => {
                        let delta_volume =
                            self.pipe_state[*pipe_id].delta_volume_substep[side.index()];

                        if delta_volume < 0. {
                            let pipe = &mut net.pipes[*pipe_id];
                            for chunk in pipe.vessel.drain(side, -delta_volume) {
                                junc_state.vessel.fill(chunk);
                            }
                        }
                    }
                }
            }
        }

        // pipe inflow: junction vessel -> pipe vessel
        for (junc_id, junc) in net.junctions.iter() {
            let junc_state = &mut self.junction_state[junc_id];

            for port in junc.iter() {
                match *port {
                    PipeOutlet { pipe_id, side } => {
                        let delta_volume =
                            self.pipe_state[*pipe_id].delta_volume_substep[side.index()];

                        if delta_volume > 0. {
                            let pipe = &mut net.pipes[*pipe_id];
                            if let Some(chunk) = junc_state.vessel.drain(delta_volume) {
                                pipe.vessel.fill(side, chunk);
                            }
                        }
                    }
                }
            }
        }

        sub_dt
    }

    fn junction_poiseuille_pressure<'a>(
        &self,
        ports: impl Iterator<Item = (&'a Pipe, &'a PipeSolverState, PortTag)> + Clone,
    ) -> Option<f64> {
        let total_conductance = ports
            .clone()
            .map(|(_, state, _)| state.poiseuille_conductance)
            .sum::<f64>();

        (total_conductance > 0.).then(|| {
            ports
                .map(|(_, state, side)| state.poiseuille_conductance * state.pressure[side.index()])
                .sum::<f64>()
                / total_conductance
        })
    }

    fn junction_zero_flow_pressure<'a>(
        &self,
        ports: impl Iterator<Item = (&'a Pipe, &'a PipeSolverState, PortTag)> + Clone,
    ) -> Result<f64, JunctionPressureSolverError> {
        if ports.clone().count() == 0 {
            return Err(JunctionPressureSolverError::NoPorts);
        }

        // Equalized pressure based on Poiseuille flow as initial guess
        let poiseuille_pressure = match self.junction_poiseuille_pressure(ports.clone()) {
            Some(p) => p,
            None => return Err(JunctionPressureSolverError::NoConductance),
        };
        // println!("poiseuille: {poiseuille_pressure}");

        // Objective: Total flow over all junction ports must be zero.
        let obj_f = |x| -> f64 {
            ports
                .clone()
                .map(|(pipe, state, port)| pipe.flow_model.flow(x - state.pressure[port.index()]))
                .sum()
        };

        // Derivative of cost function
        let dx_f = |x| -> f64 {
            ports
                .clone()
                .map(|(pipe, state, port)| {
                    pipe.flow_model.flow_dx(x - state.pressure[port.index()])
                })
                .sum()
        };

        match newton_root_solver(
            poiseuille_pressure,
            FLOW_CONSERVATION_THRESHOLD,
            125,
            obj_f,
            dx_f,
        ) {
            Ok(p) => Ok(p),
            Err(err) => Err(JunctionPressureSolverError::DidNotConverge(err)),
        }
    }

    pub fn print_overview(&self) {
        println!(">> Junctions:");
        self.print_junction_overview();
        println!(">> Pipes:");
        self.print_pipe_overview();
    }

    pub fn print_junction_overview(&self) {
        println!("  {:<6} {:>12} {:>12}", "ID", "Pressure [Pa]", "Volume [L]");
        println!("{}", "-".repeat(6 + 12 * 4 + 6));

        for (id, j) in self.junction_state.iter() {
            let pressure = j.pressure.unwrap_or(f64::NAN);
            let volume = j.vessel.volume();

            println!(
                "  {:<6} {:>12.4} {:>12.4}",
                id,
                pressure,
                volume_to_liters(volume)
            );
        }
    }

    pub fn print_pipe_overview(&self) {
        // Print header
        println!(
            "  {:<6} {:>12} {:>12} {:>12} {:>12} {:>12} {:>12} {:>12} {:>12}",
            "ID",
            "Volume [L]",
            "Conductance",
            "Pressure A",
            "Pressure B",
            "Flow A [L/s]",
            "Flow B [L/s]",
            "Vel. A [m/s]",
            "Vel. B [m/s]"
        );
        println!("{}", "-".repeat(6 + 12 * 6 + 6)); // separator

        // Print each pipe
        for (id, pipe) in self.pipe_state.iter() {
            println!(
                "  {:<6} {:>12.6} {:>12.2E} {:>12.1} {:>12.1} {:>12.6} {:>12.6} {:>12.3} {:>12.3}",
                id,
                volume_to_liters(pipe.volume),
                pipe.poiseuille_conductance,
                pipe.pressure[0],
                pipe.pressure[1],
                volume_to_liters(pipe.flow[0]),
                volume_to_liters(pipe.flow[1]),
                pipe.velocity[0],
                pipe.velocity[1],
            );
        }
    }

    pub fn write_pipes_to_csv(&self, path: &str) -> Result<(), Box<dyn Error>> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // CSV header
        writeln!(writer, "ID,Volume,Conductance,PA,PB,QA,QB,vA,vB")?;

        // Iterate over pipe states
        for (id, pipe) in self.pipe_state.iter() {
            writeln!(
                writer,
                "{},{:.6},{:.6e},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6}",
                id,
                pipe.volume,
                pipe.poiseuille_conductance,
                pipe.pressure[0],
                pipe.pressure[1],
                pipe.flow[0],
                pipe.flow[1],
                pipe.velocity[0],
                pipe.velocity[1],
            )?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JunctionPressureSolverError {
    NoPorts,
    DidNotConverge(NewtonRootSolverError),
    NoConductance,
}

/// Accuracy threshold for flow conservation
pub const FLOW_CONSERVATION_THRESHOLD: f64 = 1e-8; // 0.01 mL/s
