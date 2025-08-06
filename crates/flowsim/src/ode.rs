use crate::{models::PressureModel, FlowNet, Port, Port::PipeOutlet, PortTag, ReservoirVessel};
use gems::{volume_to_liters, IntMap, GRAVITY_CONSTANT, ODE};
use nalgebra::DVector;
use std::{
    cell::RefCell,
    error::Error,
    f64::consts::PI,
    fs::File,
    io::{BufWriter, Write},
};

pub struct FlowNetOde<'a> {
    net: &'a FlowNet,
    scratch: &'a FlowNetOdeScratch,
}

impl<'a> FlowNetOde<'a> {
    pub fn new(net: &'a FlowNet, scratch: &'a mut FlowNetOdeScratch) -> Self {
        Self { net, scratch }
    }

    pub fn new_state_vec(&self) -> DVector<f64> {
        DVector::zeros(self.net.pipes.len() * 3)
    }
}

pub fn read_ode_state_from_net<'a>(net: &FlowNet, mut state: NetStateMut) {
    for (pipe_id, pipe) in net.pipes.iter() {
        let pipe_idx = pipe_id; // FIXME
        *state.velocity_a_mut(pipe_idx) = pipe.port_velocity[0];
        *state.velocity_b_mut(pipe_idx) = pipe.port_velocity[1];
        *state.volume_mut(pipe_idx) = pipe.vessel.combined_chunk().map_or(0., |c| c.volume());
    }
}

pub fn apply_ode_state_derivatives_to_net(
    net: &mut FlowNet,
    scratch: &mut FlowNetOdeScratch,
    dt: f64,
    state: NetState,
) {
    let mut junc_scratch = scratch.junctions.borrow_mut();
    let mut pipe_scratch = scratch.pipes.borrow_mut();

    // volume exchange as suggested by new state
    for (junc_id, junc) in net.junctions.iter() {
        let junc_idx = junc_id;
        let junc_scr = &mut junc_scratch[junc_idx];

        // compute supply and demand
        for port in junc.iter() {
            match *port {
                PipeOutlet { pipe_id, side } => {
                    let pipe_idx = *pipe_id;
                    let pipe_scr = &mut pipe_scratch[pipe_idx];

                    let delta_volume =
                        state.velocity(pipe_idx, side) * dt * pipe_scr.cross_section_area;
                    pipe_scr.delta_volume[side.index()] = delta_volume;

                    if delta_volume < 0. {
                        junc_scr.supply -= delta_volume;
                    } else {
                        junc_scr.demand += delta_volume;
                    }
                }
            }
        }

        // compute factors s, d such that s*S = d*D
        //    S    D    s    d
        //   >0   >0    1   S/D
        //   >0    0    0   Nul
        //    0   >0   Nul   0
        //    0    0   Nul  Nul
        if junc_scr.supply > 0. && junc_scr.demand > 0. {
            junc_scr.supply_fullfillment = 1.;
            junc_scr.demand_fullfillment = junc_scr.supply / junc_scr.demand;
        } else {
            junc_scr.supply_fullfillment = 0.;
            junc_scr.demand_fullfillment = 0.;
        }
        // println!(
        //     "[{junc_idx}] fullfillment: {:5.3?} {:5.3?} | {:5.3E} /{:5.3E}",
        //     junc_scr.supply_fullfillment,
        //     junc_scr.demand_fullfillment,
        //     junc_scr.supply,
        //     junc_scr.demand
        // );

        // pipe outflow: pipe vessel -> junction vessel (negative delta volume)
        for port in junc.iter() {
            match *port {
                PipeOutlet { pipe_id, side } => {
                    let pipe_idx = *pipe_id;
                    let pipe = &mut net.pipes[*pipe_id];

                    let dv = &mut pipe_scratch[pipe_idx].delta_volume[side.index()];

                    if *dv < 0. {
                        *dv *= junc_scr.supply_fullfillment;
                        if *dv < 0. {
                            for chunk in pipe.vessel.drain(side, -*dv) {
                                junc_scr.vessel.fill(chunk);
                            }
                        }
                    }
                }
            }
        }

        // pipe inflow: junction vessel -> pipe vessel (positive delta volume)
        for port in junc.iter() {
            match *port {
                PipeOutlet { pipe_id, side } => {
                    let pipe_idx = *pipe_id;
                    let pipe = &mut net.pipes[*pipe_id];

                    let dv = &mut pipe_scratch[pipe_idx].delta_volume[side.index()];

                    if *dv > 0. {
                        *dv *= junc_scr.demand_fullfillment;
                        if *dv > 0. {
                            if let Some(chunk) = junc_scr.vessel.drain(*dv) {
                                pipe.vessel.fill(side, chunk);
                            }
                        }
                    }
                }
            }
        }
    }

    // update velocity to match actual flow
    for (pipe_id, pipe) in net.pipes.iter_mut() {
        let pipe_idx = pipe_id;
        let scr = &mut pipe_scratch[pipe_idx];

        pipe.port_velocity[0] = scr.delta_volume[0] / scr.cross_section_area / dt;
        pipe.port_velocity[1] = scr.delta_volume[1] / scr.cross_section_area / dt
    }
}

#[derive(Default, Debug)]
pub struct FlowNetOdeScratch {
    /// Additional state for each pipe
    pub pipes: RefCell<IntMap<PipeBundleScratch>>,

    /// Pressure of each junction
    pub junctions: RefCell<IntMap<JunctionScratch>>,
}

#[derive(Default, Clone, Debug)]
pub struct PipeBundleScratch {
    /// Radius of a single pipe in the bundle (at current volume)
    strand_radius: f64,

    /// Number of strands in the bundle. All properties not prefixed with strand_ are are wrt to
    /// the whole bundel.
    strand_count: f64,

    /// Cross section area of the pipe bundle (at current volume)
    cross_section_area: f64,

    /// Mass of liquid contained in the pipe
    volume: f64,

    /// Mass of liquid contained in the pipe
    mass: f64,

    /// Density of liquid contained in the pipe
    density: f64,

    /// Viscosity of liquid contained in the pipe
    viscosity: f64,

    pump_force: [f64; 2],
    grav_force: [f64; 2],
    elas_force: f64,
    visc_force: [f64; 2],
    turb_force: [f64; 2],
    damp_force: [f64; 2],

    /// Force acting on the ports of a pipe. Positive force pushes inwards
    force: [f64; 2],

    /// Junction pressure at ports
    junction_pressure: [Option<f64>; 2],

    /// Volume change over each port
    delta_volume: [f64; 2],
}

#[derive(Default, Clone, Debug)]
pub struct JunctionScratch {
    pressure: f64,
    vessel: ReservoirVessel,
    supply: f64,
    demand: f64,
    supply_fullfillment: f64,
    demand_fullfillment: f64,
}

impl<'a> ODE<DVector<f64>> for FlowNetOde<'a> {
    fn eval(&self, _time: f64, state_vec: DVector<f64>) -> DVector<f64> {
        let state = NetState(&state_vec);

        // prepare pipe information
        {
            let mut pipe_scratch = self.scratch.pipes.borrow_mut();
            pipe_scratch.clear();
            for (pipe_id, pipe) in self.net.pipes.iter() {
                let pipe_idx = pipe_id; // FIXME

                let mut scr = PipeBundleScratch::default();

                scr.strand_count = pipe.strand_count();

                // Volume cannot become negative!
                let volume = state.volume(pipe_idx).max(0.);
                scr.volume = volume;

                scr.cross_section_area = volume / pipe.shape.length;
                scr.strand_radius = (scr.cross_section_area / PI / scr.strand_count).sqrt();

                // During solving the volume is changed by the solver. We compute mass and viscosity
                // assuming that the density is constant.
                let fluid = pipe.vessel.combined_chunk();
                scr.density = fluid.as_ref().map_or(1e3, |c| c.density());
                scr.mass = scr.density * volume;
                scr.viscosity = fluid.as_ref().map_or(1e-3, |c| c.viscosity());

                // compute forces

                // If pipe volume increases above nominal a force pushes liquid out of both ports.
                // Force on port from elastic pressure: F = P * A_cross
                let elas_force =
                    -pipe.elasticity_pressure_model.pressure(volume) * scr.cross_section_area;
                scr.elas_force = elas_force;

                // We use half the pipe length for both viscous and turbulent force for each port.
                // This will give the full theoretic force in case of only throughflow. In case of
                // only inflow this represents that liquid on each port on average moves only half
                // the pipe length to be stores.
                let effective_length = pipe.shape.length * 0.5;

                for side in [PortTag::A, PortTag::B] {
                    let pix = side.index();

                    // external force, e.g. from a pump
                    // F = P A
                    let pump_force = pipe.external_port_pressure[pix] * scr.cross_section_area;

                    // If pipe is inclined under a positive angle port B is higher than port A and
                    // gravity pushes liquid out at the bottom (port A) and in at the top (port B).
                    let grav_force = scr.mass
                        * GRAVITY_CONSTANT
                        * pipe.ground_angle.sin()
                        * match side {
                            PortTag::A => -1.,
                            PortTag::B => 1.,
                        };

                    // flow velocity through port. positive velocity flows inwards.
                    let v = state.velocity(pipe_idx, side);

                    // Viscous force counter-acts movement.
                    let visc_force =
                        (-8. * PI) * scr.viscosity * scr.strand_count * effective_length * v;

                    // Turbulent force counter-acts movement, but proportional to v^2.
                    let turb_force = (-0.25 * PI)
                        * pipe.darcy_factor
                        * scr.density
                        * scr.strand_radius
                        * scr.strand_count
                        * effective_length
                        * v
                        * v.abs();

                    // Additional dampening force linear in v with tuned coefficient.
                    let damp_force = -pipe.dampening * scr.strand_count * v;

                    scr.force[pix] =
                        pump_force + elas_force + grav_force + visc_force + turb_force + damp_force;

                    scr.pump_force[pix] = pump_force;
                    scr.grav_force[pix] = grav_force;
                    scr.visc_force[pix] = visc_force;
                    scr.turb_force[pix] = turb_force;
                    scr.damp_force[pix] = damp_force;
                }

                pipe_scratch.set(pipe_id, scr);
            }
        }

        // compute junction equalization pressure
        {
            let mut pipe_scratch = self.scratch.pipes.borrow_mut();
            let mut junction_scratch = self.scratch.junctions.borrow_mut();
            junction_scratch.clear();
            for (junc_id, junc) in self.net.junctions.iter() {
                let mut scr = JunctionScratch::default();

                scr.pressure =
                    junction_zero_flow_pressure(junc.iter().flat_map(|port| match *port {
                        Port::PipeOutlet { pipe_id, side } => Some((&pipe_scratch[*pipe_id], side)),
                    }));
                // println!("{junc_id}: junction pressure={}", scr.pressure);

                for port in junc.iter() {
                    match port {
                        Port::PipeOutlet { pipe_id, side } => {
                            let pipe_idx = **pipe_id; // FIXME
                            pipe_scratch[pipe_idx].junction_pressure[side.index()] =
                                Some(scr.pressure);
                        }
                    }
                }

                junction_scratch.set(junc_id, scr);
            }
        }

        // compute derivatives
        let mut derivatives_vec = DVector::zeros(state.0.len());
        {
            let pipe_scratch = self.scratch.pipes.borrow();

            let mut derivatives = NetStateDerivativeMut(&mut derivatives_vec);

            for (pipe_id, _) in self.net.pipes.iter() {
                let pipe_idx = pipe_id; // FIXME pipe linear index from pipe slab index
                let scr = &pipe_scratch[pipe_idx];

                // dv = (F + FJ)/m = (F + PJ A)/m
                for side in [PortTag::A, PortTag::B] {
                    let pix = side.index();

                    *derivatives.acceleration_mut(pipe_idx, side) = match scr.junction_pressure[pix]
                    {
                        Some(junction_pressure) => {
                            (scr.force[pix] + junction_pressure * scr.cross_section_area) / scr.mass
                        }
                        None => {
                            // acceleration is 0 if port is not connected to a junction
                            0.
                        }
                    }
                }

                // dV = (va + vb) * A
                *derivatives.volume_change_mut(pipe_idx) =
                    state.storage_velocity(pipe_idx) * scr.cross_section_area;
                // println!(
                //     "{pipe_idx}: dV/dt={}, v={}, A={}",
                //     *derivatives.volume_change_mut(pipe_idx),
                //     state.storage_velocity(pipe_idx),
                //     scr.cross_section_area
                // );
            }
        }

        // self.print_overview();

        derivatives_vec
    }
}

/// Wrapper around vector representing the variables of the ODE: inflow velocities at both ports
/// of the pipe and total volume stored in the pipe.
pub struct NetState<'a>(pub &'a DVector<f64>);

impl<'a> NetState<'a> {
    pub fn pipe_count(&self) -> usize {
        self.0.len() / 3
    }

    pub fn velocity(&self, i: usize, side: PortTag) -> f64 {
        match side {
            PortTag::A => self.velocity_a(i),
            PortTag::B => self.velocity_b(i),
        }
    }

    fn velocity_a(&self, i: usize) -> f64 {
        self.0[3 * i]
    }

    fn velocity_b(&self, i: usize) -> f64 {
        self.0[3 * i + 1]
    }

    pub fn volume(&self, i: usize) -> f64 {
        self.0[3 * i + 2]
    }

    /// Combined velocity of liquid flowing into the pipe from both ports
    pub fn storage_velocity(&self, i: usize) -> f64 {
        inwards(self.velocity_a(i), self.velocity_b(i))
    }

    /// Velocity of liquid flowing through the pipe (from port A to B)
    pub fn through_velocity(&self, i: usize) -> f64 {
        through(self.velocity_a(i), self.velocity_b(i))
    }
}

pub struct NetStateMut<'a>(pub &'a mut DVector<f64>);

impl<'a> NetStateMut<'a> {
    pub fn velocity_mut(&mut self, i: usize, side: PortTag) -> &mut f64 {
        match side {
            PortTag::A => self.velocity_a_mut(i),
            PortTag::B => self.velocity_b_mut(i),
        }
    }

    pub fn velocity_a_mut(&mut self, i: usize) -> &mut f64 {
        &mut self.0[3 * i]
    }

    pub fn velocity_b_mut(&mut self, i: usize) -> &mut f64 {
        &mut self.0[3 * i + 1]
    }

    pub fn volume_mut(&mut self, i: usize) -> &mut f64 {
        &mut self.0[3 * i + 2]
    }
}

/// Wrapper around vector representing the derivatives of the ODE variables: change of velocities
/// at both ports and change of volume stored in the pipe.
pub struct NetStateDerivativeMut<'a>(pub &'a mut DVector<f64>);

impl<'a> NetStateDerivativeMut<'a> {
    fn acceleration_mut(&mut self, i: usize, side: PortTag) -> &mut f64 {
        match side {
            PortTag::A => self.acceleration_a_mut(i),
            PortTag::B => self.acceleration_b_mut(i),
        }
    }

    fn acceleration_a_mut(&mut self, i: usize) -> &mut f64 {
        &mut self.0[3 * i]
    }

    fn acceleration_b_mut(&mut self, i: usize) -> &mut f64 {
        &mut self.0[3 * i + 1]
    }

    fn volume_change_mut(&mut self, i: usize) -> &mut f64 {
        &mut self.0[3 * i + 2]
    }

    pub fn acceleration_a(&self, i: usize) -> f64 {
        self.0[3 * i]
    }

    pub fn acceleration_b(&self, i: usize) -> f64 {
        self.0[3 * i + 1]
    }

    pub fn volume_change(&self, i: usize) -> f64 {
        self.0[3 * i + 2]
    }
}

fn inwards(a: f64, b: f64) -> f64 {
    a + b
}

fn through(a: f64, b: f64) -> f64 {
    a.max(-b).min(0.) + a.min(-b).max(0.)
}

/// Solve sum_i Q_i = 0
/// sum_i v_i A_i = 0 => sum_i (F_i + F_J) A_i/m_i = 0 (derivative of mass conservation)
/// m_i = A_i*L*rho => A_i/m_i = 1/(L*rho)
/// F_J = P_J A_i (junction pressure)
/// Solve for P: P = - (sum_i A_i/m_i F_i) / (sum_i A_i^2/m_i)
fn junction_zero_flow_pressure<'a>(
    ports: impl Iterator<Item = (&'a PipeBundleScratch, PortTag)>,
) -> f64 {
    let mut h = 0.;
    let mut z = 0.;

    for (scr, side) in ports {
        let area = scr.cross_section_area;
        h += scr.force[side.index()] * area / scr.mass;
        z += area * area / scr.mass;
    }

    -h / z
}

impl FlowNetOde<'_> {
    pub fn print_overview(&self) {
        println!(">> Junctions:");
        self.print_junction_overview();
        println!(">> Pipes:");
        self.print_pipe_overview();
    }

    pub fn print_junction_overview(&self) {
        println!("  {:<6} {:>12} ", "ID", "Pressure [Pa]");
        println!("{}", "-".repeat(6 + 12 * 4 + 6));

        for (id, j) in self.scratch.junctions.borrow().iter() {
            println!("  {:<6} {:>12.4}", id, j.pressure,);
        }
    }

    pub fn print_pipe_overview(&self) {
        // Print header
        println!(
            "  {:<6} {:>12} {:>12} {:>12} {:>12} {:>12} {:>12}",
            "ID", "Volume [L]", "EForce", "Force A", "Force B", "Vel. A [m/s]", "Vel. B [m/s]"
        );
        println!("{}", "-".repeat(6 + 12 * 6 + 6)); // separator

        // Print each pipe
        for (id, state) in self.scratch.pipes.borrow().iter() {
            let pipe = &self.net.pipes[id];
            println!(
                "  {:<6} {:>12.6} {:>12.3} {:>12.3} {:>12.3} {:>12.3} {:>12.3}",
                id,
                volume_to_liters(pipe.vessel.combined_chunk().map_or(0., |c| c.volume())),
                state.elas_force,
                state.force[0],
                state.force[1],
                pipe.port_velocity[0],
                pipe.port_velocity[1],
            );
        }
        // for (id, state) in self.scratch.pipes.borrow().iter() {
        //     println!("{id}: {state:?}");
        // }

        println!(
            "Total Volume: {} L",
            volume_to_liters(
                self.net
                    .pipes
                    .iter()
                    .map(|(_, s)| s.vessel.combined_chunk().unwrap().volume())
                    .sum::<f64>()
            )
        )
    }

    pub fn write_pipes_to_csv(&self, path: &str) -> Result<(), Box<dyn Error>> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // CSV header
        writeln!(writer, "ID,Volume,FA,FB,vA,vB")?;

        // Iterate over pipe states
        for (id, state) in self.scratch.pipes.borrow().iter() {
            let pipe = &self.net.pipes[id];
            writeln!(
                writer,
                "{},{:.6},{:.6},{:.6},{:.6},{:.6}",
                id,
                pipe.vessel.combined_chunk().map_or(0., |c| c.volume()),
                state.force[0],
                state.force[1],
                pipe.port_velocity[0],
                pipe.port_velocity[1],
            )?;
        }

        Ok(())
    }
}
