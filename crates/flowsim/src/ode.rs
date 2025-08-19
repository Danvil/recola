use crate::{
    models::PressureModel, DataColumn, FlowNet, PipeDef, PipeScratch, PipeSolution, PipeState,
    Port, Port::PipeOutlet, PortTag,
};
use gems::{volume_to_liters, IntMap, IntMapTuple, GRAVITY_CONSTANT, ODE};
use std::{
    cell::{Ref, RefCell},
    error::Error,
    f64::consts::PI,
};

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

    pub fn net(&self) -> &FlowNet {
        &self.net
    }

    pub fn pipe_scratch(&self) -> Ref<IntMap<PipeScratch>> {
        self.pipe_scratch.borrow()
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
            assert!(scr.strand_count > 0.);

            // Volume cannot become negative!
            let volume = state.volume.max(0.);
            assert!(volume.is_finite());
            assert!(volume >= 0.);
            scr.volume = volume;

            // scr.port_cross_section_area = scr.strand_count *
            // pipe.shape.model.cross_section_area();

            scr.tube_cross_section_area = volume / pipe.shape.model.length;
            scr.tube_strand_radius = (scr.tube_cross_section_area / PI / scr.strand_count).sqrt();

            // During solving the volume is changed by the solver. We compute mass and
            // viscosity assuming that the density is constant.
            // Note: Do not use volume from fluid as it is given by the solver state.
            scr.mass = pipe.fluid.density * volume;

            // Port Area / Mass does not actually depend on the mass stored in the pipe.
            scr.area_per_mass = 1.0 / (pipe.shape.model.length * pipe.fluid.density);

            // compute forces

            // If pipe volume increases above nominal a force pushes liquid out of both ports.
            // Force on port from elastic pressure: F = P * A_cross. Note that we use the nominal
            // port area to avoid diminishing force due to vessel collapse.
            // We compute it as acceleration so it also works with m = 0:
            // F = P A = m a => a = P * A / m
            scr.elas_pressure = pipe.elasticity_pressure_model.pressure(volume);
            let elas_accel = -scr.elas_pressure * scr.area_per_mass;
            scr.elas_accel = elas_accel;

            // We use half the pipe length for both viscous and turbulent force for each
            // port. This will give the full theoretic force in case of
            // only throughflow. In case of only inflow this represents
            // that liquid on each port on average moves only half
            // the pipe length to be stores.
            let effective_length = pipe.shape.model.length * 0.5;

            for side in [PortTag::A, PortTag::B] {
                let pix = side.index();

                scr.port_cross_section_area[pix] =
                    scr.tube_cross_section_area * pipe.port_area_factor[pix];

                // external force, e.g. from a pump. We use nominal area to avoid diminishing force
                // in case of volume changes.
                // We compute it as acceleration so it also works with m = 0:
                // F = P A = m a => a = P * A / m
                let pump_accel = pipe.external_port_pressure[pix] * scr.area_per_mass;

                // If pipe is inclined under a positive angle port B is higher than port A
                // and gravity pushes liquid out at the bottom (port
                // A) and in at the top (port B).
                // Naturally computes as an acceleration independent of mass.
                let grav_accel = GRAVITY_CONSTANT
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
                    * scr.tube_strand_radius
                    * scr.strand_count
                    * effective_length
                    * v
                    * v.abs();

                // Additional dampening force linear in v with tuned coefficient.
                let damp_force = -pipe.dampening * scr.strand_count * v;

                scr.pump_accel[pix] = pump_accel;
                scr.grav_accel[pix] = grav_accel;
                scr.visc_force[pix] = visc_force;
                scr.turb_force[pix] = turb_force;
                scr.damp_force[pix] = damp_force;

                scr.ext_accels[pix] = pump_accel + elas_accel + grav_accel;
                scr.drag_forces[pix] = visc_force + turb_force + damp_force;

                // if volume == 0. {
                //     println!("VOL0: {scr:?}");
                // }
            }

            // if scr.pump_accel[0] != 0. || scr.pump_accel[1] != 0. {
            //     println!("{scr:?}");
            // }

            scr
        });

        // compute junction equalization pressure
        {
            let mut junc_scratch = self.junc_scratch.borrow_mut();
            junc_scratch.clear();

            for (junc_id, junc) in self.net.topology.junctions.iter() {
                let ports = junc.iter().filter_map(|port| match *port {
                    Port::PipeOutlet { pipe_id, side } => {
                        let scr = &pipe_scratch[*pipe_id];
                        Some((scr, side))
                    }
                });

                let junction_pressure = junction_zero_flow_pressure(ports);

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
        let derivatives = (&state.0, &mut pipe_scratch).map(|(state, scr)| {
            let mut derivatives = PipeState::default();

            // a = F_drag/m + (a_mix - F_junc/m)
            // F_junc = P_junc * A
            // a = F_drag/m + a_mix - P_junc * (A/m)
            // If m=0 then F_drag is irrelevant and ignored.
            for side in [PortTag::A, PortTag::B] {
                let pix = side.index();

                let mut accel = match scr.junction_pressure[pix] {
                    Some(junction_pressure) => {
                        let a_drag = if scr.mass > 0. {
                            scr.drag_forces[pix] / scr.mass
                        } else {
                            0.
                        };

                        // println!(
                        //     "{side:?}: {} {} {} {}",
                        //     a_drag,
                        //     junction_pressure,
                        //     junction_pressure * scr.area_per_mass,
                        //     scr.ext_accels[pix]
                        // );

                        a_drag + scr.ext_accels[pix] - junction_pressure * scr.area_per_mass
                    }
                    None => {
                        // acceleration is 0 if port is not connected to a junction
                        0.
                    }
                };
                assert!(accel.is_finite());

                // If mass is zero acceleration cannot be negative (no outflow)
                if scr.mass == 0. {
                    accel = accel.max(0.);
                }

                scr.total_accel[pix] = accel;

                // Note: Acceleration is stored in the "velocity" slot of the derivative.
                derivatives.velocity[side] = accel;
            }

            // dV = (va + vb) * A
            // Note: this is the change of volume
            let flow = state.velocity[0] * scr.port_cross_section_area[0]
                + state.velocity[1] * scr.port_cross_section_area[1];
            assert!(flow.is_finite());

            // println!(
            //     "{pipe_idx}: dV/dt={}, v={}, A={}",
            //     *derivatives.volume_change_mut(pipe_idx),
            //     state.storage_velocity(pipe_idx),
            //     scr.cross_section_area
            // );

            // Note: Change of volume (flow) is stored in the "volume" slot of the derivative.
            derivatives.volume = flow;

            derivatives
        });

        // self.print_overview();

        // println!("Scratch: {:?}", pipe_scratch);
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
                        * pipe_scratch[pipe_idx].port_cross_section_area[side.index()];
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
        for side in [PortTag::A, PortTag::B] {
            let area = scr.port_cross_section_area[side.index()];

            if area == 0. {
                sol.velocity[side] = 0.;
            } else {
                sol.velocity[side] = sol.delta_volume[side] / area / dt;
            }
        }
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
/// F_J = P_J A_i (junction pressure)
/// Solve for P: P = - (sum_i A_i/m_i F_i) / (sum_i A_i^2/m_i)
/// We use m_i = n A_i L ρ => A_i/m_i = 1/(n L ρ) to avoid division by zero for empty pipes
fn junction_zero_flow_pressure<'a>(
    ports: impl Iterator<Item = (&'a PipeScratch, PortTag)>,
) -> Option<f64> {
    let mut h = 0.;
    let mut z = 0.;

    for (scr, side) in ports {
        let pix = side.index();
        // h = (F_drag + accel*m) * A/m = F_drag * A/m + accel * A
        // This will also work for m=0.
        h += scr.drag_forces[pix] * scr.area_per_mass
            + scr.ext_accels[pix] * scr.port_cross_section_area[pix];
        z += scr.port_cross_section_area[pix] * scr.area_per_mass;
    }

    (z > 0.).then(|| h / z)
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
            "  {:<6} {:>16} {:>12} {:>12} {:>12} {:>12} {:>12} {:>12} {:>12} {:>12} {:>12} {:>12} {:>12} {:>12}",
            "ID",
            "Name",
            "Volume [L]",
            "P_elas",
            "Drag F A",
            "Drag F B",
            "Ext F A",
            "Ext F B",
            "Junc F A",
            "Junc F B",
            "Vel. A [m/s]",
            "Vel. B [m/s]",
            "Area A [cm2]",
            "Area B [cm2]"
        );
        println!("{}", "-".repeat(6 + 16 + 12 * 12 + 15)); // separator

        // Print each pipe
        for (id, scr) in self.pipe_scratch.borrow().iter() {
            // let pipe = &self.net.pipes[id];
            println!(
                "  {:<6} {:>16} {:>12.6} {:>12.3} {:>12.3} {:>12.3} {:>12.3} {:>12.3} {:>12.3} {:>12.3} {:>12.3} {:>12.3} {:>12.3} {:>12.3}",
                id,
                self.net.pipes[id].name,
                volume_to_liters(state[id].volume),
                scr.elas_pressure,
                scr.drag_forces[0],
                scr.drag_forces[1],
                scr.ext_accels[0] * scr.mass,
                scr.ext_accels[1] * scr.mass,
                scr.junction_pressure[0].unwrap_or(0.) * scr.area_per_mass,
                scr.junction_pressure[1].unwrap_or(0.) * scr.area_per_mass,
                state[id].velocity[PortTag::A],
                state[id].velocity[PortTag::B],
                10000. * scr.port_cross_section_area[0],
                10000. * scr.port_cross_section_area[1],
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
