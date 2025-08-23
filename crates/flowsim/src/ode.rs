use crate::{
    models::PressureModel, JunctionScratch, PipeDef, PipeJunctionPort, PipeScratch, PipeSolution,
    PipeState, PipeStateDerivative, PortTag, SolutionDeltaVolume,
};
use gems::{GRAVITY_CONSTANT, ODE};
use nalgebra as na;
use simplecs::prelude::*;
use std::{cell::RefCell, f64::consts::PI};

#[derive(Component)]
struct VecIndex(pub usize);

#[derive(Singleton)]
struct VecCount(pub usize);

/// ODE for solving fluid flow
pub struct FlowNetOde<'w> {
    world: RefCell<&'w mut World>,
}

impl<'w> FlowNetOde<'w> {
    pub fn new(world: &'w mut World) -> Self {
        Self {
            world: world.into(),
        }
    }

    pub fn init_state(&self) -> na::DVector<f64> {
        let world = &mut self.world.borrow_mut();
        world.run(ode_init_junc);
        world.run(ode_vectorize_assign_indices);
        world.run(ode_assure_components);
        world.run(ode_state_to_vec)
    }

    pub fn step(&self, dt: f64, state: na::DVector<f64>) {
        let world = &mut self.world.borrow_mut();
        world.run_with_input(state, ode_vec_to_state);
        world.run_with_input(dt, ode_solution_fullfillment);
        world.run_with_input(dt, ode_solution_velocity);
        world.run(ode_apply_solution);
    }
}

impl<'w> ODE<na::DVector<f64>> for FlowNetOde<'w> {
    fn eval(&self, _time: f64, state: na::DVector<f64>) -> na::DVector<f64> {
        let world = &mut self.world.borrow_mut();
        world.run_with_input(state, ode_vec_to_state);
        world.run(ode_pipe_preprocess);
        world.run(ode_junction_equalize_pressure);
        world.run(ode_derivatives);
        world.run(ode_derivative_to_vec)
    }
}

fn ode_init_junc(q: Query<This, With<(E1, PipeJunctionPort, This)>>, mut cmd: Commands) {
    for ejunc in q.iter() {
        cmd.entity(ejunc).set(JunctionScratch::default());
    }
}

fn ode_vectorize_assign_indices(q: Query<This, With<PipeDef>>, mut cmd: Commands) {
    let mut len = 0;
    q.each(|e| {
        cmd.entity(e).set(VecIndex(len));
        len += 1;
    });
    cmd.set_singleton(VecCount(len));
}

fn ode_assure_components(q: Query<This, With<PipeDef>>, mut cmd: Commands) {
    q.each(|e| {
        cmd.entity(e).set(PipeScratch::default());
        cmd.entity(e).set(PipeStateDerivative::default());
        cmd.entity(e).set(PipeSolution::default());
        cmd.entity(e).set(SolutionDeltaVolume::default());
    });
}

fn ode_state_to_vec(
    q: Query<(&VecIndex, &PipeState)>,
    vec_len: Singleton<VecCount>,
) -> na::DVector<f64> {
    let mut out = na::DVector::zeros(3 * vec_len.0);
    q.each(|(&VecIndex(i), state)| {
        out[3 * i] = state.volume;
        out[3 * i + 1] = state.velocity[0];
        out[3 * i + 2] = state.velocity[1];
    });
    out
}

fn ode_derivative_to_vec(
    q: Query<(&VecIndex, &PipeStateDerivative)>,
    vec_len: Singleton<VecCount>,
) -> na::DVector<f64> {
    let mut out = na::DVector::zeros(3 * vec_len.0);
    q.each(|(&VecIndex(i), derivative)| {
        out[3 * i] = derivative.flow;
        out[3 * i + 1] = derivative.accel[0];
        out[3 * i + 2] = derivative.accel[1];
    });
    out
}

fn ode_vec_to_state(vec: In<na::DVector<f64>>, mut q: Query<(&VecIndex, &mut PipeState)>) {
    q.each_mut(|(&VecIndex(i), state)| {
        state.volume = vec[3 * i];
        state.velocity[0] = vec[3 * i + 1];
        state.velocity[1] = vec[3 * i + 2];
    });
}

fn ode_pipe_preprocess(mut q: Query<(&PipeDef, &PipeState, &mut PipeScratch)>) {
    q.each_mut(|(pipe, state, scr)| {
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
    });
}

fn ode_junction_equalize_pressure(
    mut q_junc: Query<(This, &mut JunctionScratch)>,
    mut q_junc_pipes: Query<(&mut PipeScratch, (This, &PipeJunctionPort, E1))>,
) {
    for (ejunc, junc_scr) in q_junc.iter_mut() {
        // Narrow the query to only pipes which connect to this specific junction
        let mut q = q_junc_pipes.bind(E1, ejunc);

        // Compute equalized junction pressure
        junc_scr.pressure =
            junction_zero_flow_pressure(q.iter().map(|(pipe_scr, pj)| (&*pipe_scr, pj.0)));

        // Store junction pressure in pipe scratch space
        for (pipe_scr, pj) in q.iter_mut() {
            pipe_scr.junction_pressure[pj.0.index()] = junc_scr.pressure;
        }
    }
}

fn ode_derivatives(mut q: Query<(&PipeState, &mut PipeScratch, &mut PipeStateDerivative)>) {
    for (state, scr, derivative) in q.iter_mut() {
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

            // Note: Acceleration is stored in the "velocity" slot of the derivative.
            derivative.accel[side] = accel;
        }

        // dV = (va + vb) * A
        // Note: this is the change of volume
        derivative.flow = state.velocity[0] * scr.port_cross_section_area[0]
            + state.velocity[1] * scr.port_cross_section_area[1];
        assert!(derivative.flow.is_finite());

        // println!(
        //     "{pipe_idx}: dV/dt={}, v={}, A={}",
        //     *derivative.volume_change_mut(pipe_idx),
        //     state.storage_velocity(pipe_idx),
        //     scr.cross_section_area
        // );
    }
}

fn ode_solution_fullfillment(
    dt: In<f64>,
    mut q_junc: Query<(This, &mut JunctionScratch)>,
    mut q_junc_pipes: Query<(
        &PipeState,
        &PipeScratch,
        &mut PipeSolution,
        (This, &PipeJunctionPort, E1),
    )>,
) {
    for (ejunc, junc_scr) in q_junc.iter_mut() {
        // Narrow the query to only pipes which connect to this specific junction
        let mut q = q_junc_pipes.bind(E1, ejunc);

        // Compute total supply and demand on the junction
        for (pipe_state, pipe_scr, pipe_sol, &PipeJunctionPort(side)) in q.iter_mut() {
            let delta_volume =
                pipe_state.velocity[side] * *dt * pipe_scr.port_cross_section_area[side.index()];
            pipe_sol.delta_volume[side] = delta_volume;

            if delta_volume < 0. {
                junc_scr.supply_count += 1;
                junc_scr.supply -= delta_volume;
            } else {
                junc_scr.demand_count += 1;
                junc_scr.demand += delta_volume;
            }
        }

        // Equalize supply and demand
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
        for (_, _, pipe_sol, &PipeJunctionPort(side)) in q.iter_mut() {
            let dv = &mut pipe_sol.delta_volume[side];

            if *dv < 0. {
                *dv *= junc_scr.supply_fullfillment;
            } else {
                *dv *= junc_scr.demand_fullfillment;
            }
        }
    }
}

fn ode_solution_velocity(dt: In<f64>, mut q: Query<(&PipeScratch, &mut PipeSolution)>) {
    for (scr, sol) in q.iter_mut() {
        for side in [PortTag::A, PortTag::B] {
            let area = scr.port_cross_section_area[side.index()];

            if area == 0. {
                sol.velocity[side] = 0.;
            } else {
                sol.velocity[side] = sol.delta_volume[side] / area / *dt;
            }
        }
    }
}

fn ode_apply_solution(mut q: Query<(&PipeSolution, &mut PipeState, &mut SolutionDeltaVolume)>) {
    let mut total_delta_volume = 0.;
    for (sol, state, delta) in q.iter_mut() {
        let delta_volume = sol.delta_volume[PortTag::A] + sol.delta_volume[PortTag::B];

        total_delta_volume += delta_volume;
        delta.delta_volume += delta_volume;
        state.volume += delta_volume;

        state.velocity = sol.velocity;
    }

    if total_delta_volume > 1e-9 {
        eprintln!("mass conservation violated: dV={total_delta_volume}");
    }
}

// pub fn increment_solution(sub: &IntMap<PipeSolution>, total: &mut IntMap<PipeSolution>) {
//     for (id, sub) in sub.iter() {
//         total[id].delta_volume = total[id].delta_volume + sub.delta_volume;
//         total[id].velocity = sub.velocity;
//     }
// }

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
