use crate::{
    models::PressureModel, DataColumn, FlowNet, PipeScratch, PipeSolution, PipeState, Port,
    Port::PipeOutlet, PortTag,
};
use gems::{volume_to_liters, IntMap, IntMapTuple, GRAVITY_CONSTANT, ODE};
use std::{cell::RefCell, error::Error, f64::consts::PI};

#[derive(Debug)]
pub struct FlowNetOde<'a> {
    net: &'a FlowNet,
    pipe_scratch: RefCell<IntMap<PipeScratch>>,
    junc_scratch: RefCell<IntMap<JunctionScratch>>,
}

impl<'a> FlowNetOde<'a> {
    pub fn new(net: &'a FlowNet) -> Self {
        Self {
            net,
            pipe_scratch: RefCell::new(IntMap::default()),
            junc_scratch: RefCell::new(IntMap::default()),
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct JunctionScratch {
    pressure: Option<f64>,
    supply_count: usize,
    demand_count: usize,
    supply: f64,
    demand: f64,
    supply_fullfillment: f64,
    demand_fullfillment: f64,
}

pub type PipeStateCol = DataColumn<PipeState>;

impl<'a> ODE<PipeStateCol> for FlowNetOde<'a> {
    fn eval(&self, _time: f64, state: PipeStateCol) -> PipeStateCol {
        // println!("State: {state:?}",);

        // prepare pipe information
        let mut pipe_scratch = (&self.net.pipes, &state.0).map(|(pipe, state)| {
            let mut scr = PipeScratch::default();

            scr.strand_count = pipe.strand_count();

            // Volume cannot become negative!
            let volume = state.volume.max(0.);
            scr.volume = volume;

            scr.cross_section_area = volume / pipe.shape.model.length;
            scr.strand_radius = (scr.cross_section_area / PI / scr.strand_count).sqrt();

            // During solving the volume is changed by the solver. We compute mass and
            // viscosity assuming that the density is constant.
            // Note: Do not use volume from fluid as it is given by the solver state.
            scr.mass = pipe.fluid.density * volume;

            // compute forces

            // If pipe volume increases above nominal a force pushes liquid out of both
            // ports. Force on port from elastic pressure: F = P *
            // A_cross
            let elas_force =
                -pipe.elasticity_pressure_model.pressure(volume) * scr.cross_section_area;
            scr.elas_force = elas_force;

            // We use half the pipe length for both viscous and turbulent force for each
            // port. This will give the full theoretic force in case of
            // only throughflow. In case of only inflow this represents
            // that liquid on each port on average moves only half
            // the pipe length to be stores.
            let effective_length = pipe.shape.model.length * 0.5;

            for side in [PortTag::A, PortTag::B] {
                let pix = side.index();

                // external force, e.g. from a pump
                // F = P A
                let pump_force = pipe.external_port_pressure[pix] * scr.cross_section_area;

                // If pipe is inclined under a positive angle port B is higher than port A
                // and gravity pushes liquid out at the bottom (port
                // A) and in at the top (port B).
                let grav_force = scr.mass
                    * GRAVITY_CONSTANT
                    * pipe.ground_angle.sin()
                    * match side {
                        PortTag::A => -1.,
                        PortTag::B => 1.,
                    };

                // flow velocity through port. positive velocity flows inwards.
                let v = state.velocity[side];

                // Viscous force counter-acts movement.
                let visc_force =
                    (-8. * PI) * pipe.fluid.viscosity * scr.strand_count * effective_length * v;

                // Turbulent force counter-acts movement, but proportional to v^2.
                let turb_force = (-0.25 * PI)
                    * pipe.darcy_factor
                    * pipe.fluid.density
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
            scr
        });

        // compute junction equalization pressure
        {
            let mut junc_scratch = self.junc_scratch.borrow_mut();
            junc_scratch.clear();

            for (junc_id, junc) in self.net.topology.junctions.iter() {
                let junction_pressure =
                    junction_zero_flow_pressure(junc.iter().flat_map(|port| match *port {
                        Port::PipeOutlet { pipe_id, side } => Some((&pipe_scratch[*pipe_id], side)),
                    }));
                junc_scratch.set(
                    junc_id,
                    JunctionScratch {
                        pressure: junction_pressure,
                        ..Default::default()
                    },
                );
                // println!("{junc_id}: junction pressure={}", scr.pressure);

                for port in junc.iter() {
                    match port {
                        Port::PipeOutlet { pipe_id, side } => {
                            pipe_scratch[**pipe_id].junction_pressure[side.index()] =
                                junction_pressure;
                        }
                    }
                }
            }
        }

        // compute derivatives
        let derivatives = (&state.0, &pipe_scratch).map(|(state, scr)| {
            let mut derivatives = PipeState::default();

            // dv = (F + FJ)/m = (F + PJ A)/m
            for side in [PortTag::A, PortTag::B] {
                let pix = side.index();

                // Note: this is the acceleration
                derivatives.velocity[side] = match scr.junction_pressure[pix] {
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
            // Note: this is the change of volume
            derivatives.volume = state.inflow_velocity() * scr.cross_section_area;
            // println!(
            //     "{pipe_idx}: dV/dt={}, v={}, A={}",
            //     *derivatives.volume_change_mut(pipe_idx),
            //     state.storage_velocity(pipe_idx),
            //     scr.cross_section_area
            // );

            derivatives
        });

        // self.print_overview();

        // println!("Scratch: {:?}", self.scratch.pipes);
        // println!("Derivative: {derivatives:?}",);

        *self.pipe_scratch.borrow_mut() = pipe_scratch;

        DataColumn(derivatives)
    }
}

pub fn compute_solution(
    ode: &FlowNetOde,
    state: &IntMap<PipeState>,
    dt: f64,
) -> IntMap<PipeSolution> {
    let mut solution = IntMap::from_count(state.slot_count(), |_| PipeSolution::default());

    let mut junc_scratch = ode.junc_scratch.borrow_mut();
    let pipe_scratch = ode.pipe_scratch.borrow();

    // volume exchange as suggested by new state
    for (junc_id, junc) in ode.net.topology.junctions.iter() {
        let junc_scr = &mut junc_scratch[junc_id];

        // compute supply and demand
        for port in junc.iter() {
            match *port {
                PipeOutlet { pipe_id, side } => {
                    let pipe_idx = *pipe_id;

                    let delta_volume = state[*pipe_id].velocity[side]
                        * dt
                        * pipe_scratch[pipe_idx].cross_section_area;
                    solution[pipe_idx].delta_volume[side] = delta_volume;

                    if delta_volume < 0. {
                        junc_scr.supply_count += 1;
                        junc_scr.supply -= delta_volume;
                    } else {
                        junc_scr.demand_count += 1;
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
        //     "{} ({}) / {} ({})",
        //     junc_scr.supply_fullfillment,
        //     junc_scr.supply_count,
        //     junc_scr.demand_fullfillment,
        //     junc_scr.demand_count
        // );

        // adjust flow to guarantee fullfillment
        for port in junc.iter() {
            match *port {
                PipeOutlet { pipe_id, side } => {
                    let dv = &mut solution[*pipe_id].delta_volume[side];

                    if *dv < 0. {
                        *dv *= junc_scr.supply_fullfillment;
                    } else {
                        *dv *= junc_scr.demand_fullfillment;
                    }
                }
            }
        }
    }

    // update velocity to match actual flow
    for (id, scr) in pipe_scratch.iter() {
        let sol = &mut solution[id];
        sol.velocity[PortTag::A] = sol.delta_volume[PortTag::A] / scr.cross_section_area / dt;
        sol.velocity[PortTag::B] = sol.delta_volume[PortTag::B] / scr.cross_section_area / dt
    }

    solution
}

pub fn apply_solution(state: &mut IntMap<PipeState>, solution: &IntMap<PipeSolution>) {
    let mut total_delta_volume = 0.;
    for (id, state) in state.iter_mut() {
        let delta_volume =
            solution[id].delta_volume[PortTag::A] + solution[id].delta_volume[PortTag::B];

        total_delta_volume += delta_volume;

        state.volume += delta_volume;

        state.velocity = solution[id].velocity
    }

    if total_delta_volume > 1e-9 {
        eprintln!("mass conservation violated: dV={total_delta_volume}");
    }
}

pub fn increment_solution(sub: &IntMap<PipeSolution>, total: &mut IntMap<PipeSolution>) {
    for (id, sub) in sub.iter() {
        total[id].delta_volume = total[id].delta_volume + sub.delta_volume;
        total[id].velocity = sub.velocity;
    }
}

/// Solve sum_i Q_i = 0
/// sum_i v_i A_i = 0 => sum_i (F_i + F_J) A_i/m_i = 0 (derivative of mass conservation)
/// m_i = A_i*L*rho => A_i/m_i = 1/(L*rho)
/// F_J = P_J A_i (junction pressure)
/// Solve for P: P = - (sum_i A_i/m_i F_i) / (sum_i A_i^2/m_i)
fn junction_zero_flow_pressure<'a>(
    ports: impl Iterator<Item = (&'a PipeScratch, PortTag)>,
) -> Option<f64> {
    let mut h = 0.;
    let mut z = 0.;

    for (scr, side) in ports {
        let area = scr.cross_section_area;
        h += scr.force[side.index()] * area / scr.mass;
        z += area * area / scr.mass;
    }

    (z > 0.).then(|| -h / z)
}

impl FlowNetOde<'_> {
    pub fn print_overview(&self, state: &IntMap<PipeState>) {
        println!(">> Junctions:");
        self.print_junction_overview();
        println!(">> Pipes:");
        self.print_pipe_overview(state);
    }

    pub fn print_junction_overview(&self) {
        println!("  {:<6} {:>12} ", "ID", "Pressure [Pa]");
        println!("{}", "-".repeat(6 + 12 * 4 + 6));

        for (id, j) in self.junc_scratch.borrow().iter() {
            println!("  {:<6} {:?}", id, j.pressure,);
        }
    }

    pub fn print_pipe_overview(&self, state: &IntMap<PipeState>) {
        // Print header
        println!(
            "  {:<6} {:>12} {:>12} {:>12} {:>12} {:>12} {:>12}",
            "ID", "Volume [L]", "EForce", "Force A", "Force B", "Vel. A [m/s]", "Vel. B [m/s]"
        );
        println!("{}", "-".repeat(6 + 12 * 6 + 6)); // separator

        // Print each pipe
        for (id, scr) in self.pipe_scratch.borrow().iter() {
            // let pipe = &self.net.pipes[id];
            println!(
                "  {:<6} {:>12.6} {:>12.3} {:>12.3} {:>12.3} {:>12.3} {:>12.3}",
                id,
                volume_to_liters(state[id].volume),
                scr.elas_force,
                scr.force[0],
                scr.force[1],
                state[id].velocity[PortTag::A],
                state[id].velocity[PortTag::B],
            );
        }
        // for (id, state) in self.scratch.pipes.borrow().iter() {
        //     println!("{id}: {state:?}");
        // }

        println!(
            "Total Volume: {} L",
            volume_to_liters(state.iter().map(|(_, pipe)| pipe.volume).sum::<f64>())
        )
    }

    pub fn write_pipes_to_csv(&self, _path: &str) -> Result<(), Box<dyn Error>> {
        todo!();

        // let file = File::create(path)?;
        // let mut writer = BufWriter::new(file);

        // // CSV header
        // writeln!(writer, "ID,Volume,FA,FB,vA,vB")?;

        // // Iterate over pipe states
        // for (id, state) in self.scratch.pipes.borrow().iter() {
        //     let pipe = &self.net.pipes[id];
        //     writeln!(
        //         writer,
        //         "{},{:.6},{:.6},{:.6},{:.6},{:.6}",
        //         id,
        //         pipe.fluid.volume,
        //         state.force[0],
        //         state.force[1],
        //         pipe.velocity[0],
        //         pipe.velocity[1],
        //     )?;
        // }

        // Ok(())
    }
}
