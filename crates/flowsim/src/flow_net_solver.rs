use crate::{
    apply_ode_state_derivatives_to_net, read_ode_state_from_net, FlowNet, FlowNetOde,
    FlowNetOdeScratch, NetState, NetStateMut,
};
use gems::{runge_kutta_3_8, ODE};
use std::{error::Error, fs::File, io::Write};

#[derive(Debug)]
pub struct FlowNetSolver {
    max_sub_steps: usize,
    max_rel_velocity: f64,
}

impl FlowNetSolver {
    pub fn new() -> Self {
        Self {
            max_sub_steps: 1,
            max_rel_velocity: 0.05,
        }
    }

    pub fn step(&mut self, k: usize, net: &mut FlowNet, dt: f64) {
        let min_sub_dt = dt / self.max_sub_steps as f64;

        let mut scratch = FlowNetOdeScratch::default();

        let mut remaining_dt = dt;
        let mut step_count = 0;
        loop {
            step_count += 1;

            // // compute max relative speed
            // let state_w = NetState(&state);
            // for (pipe_id, pipe) in net.pipes.iter() {
            // }
            let sub_dt = min_sub_dt;

            let ode = FlowNetOde::new(net, &mut scratch);
            let mut state_vec = ode.new_state_vec();
            read_ode_state_from_net(net, NetStateMut(&mut state_vec));

            let derivative = ode.eval(0., state_vec.clone());
            write_vector_to_csv(
                &derivative,
                &format!(
                    "I:/Ikabur/gos/tmp/{:03}_{:03}_derivative.csv",
                    k, step_count
                ),
            )
            .unwrap();

            let next = runge_kutta_3_8(0., state_vec, sub_dt, &ode);
            // let next = forward_integrate(0., state_vec, sub_dt, &ode);
            write_vector_to_csv(
                &next,
                &format!("I:/Ikabur/gos/tmp/{:03}_{:03}_state.csv", k, step_count),
            )
            .unwrap();

            apply_ode_state_derivatives_to_net(net, &mut scratch, sub_dt, NetState(&next));

            remaining_dt -= sub_dt;
            if remaining_dt <= 0. {
                break;
            }
        }

        // let ode = FlowNetOde::new(net, &mut scratch);
        // ode.print_overview();
        // println!("took {step_count} substeps");
    }
}

fn write_vector_to_csv<D, S>(
    vec: &nalgebra::Vector<f64, D, S>,
    path: &str,
) -> Result<(), Box<dyn Error>>
where
    D: nalgebra::Dim,
    S: nalgebra::Storage<f64, D>,
    nalgebra::DefaultAllocator: nalgebra::allocator::Allocator<D>,
{
    let mut file = File::create(path)?;
    let mut iter = vec.iter();
    if let Some(first) = iter.next() {
        write!(file, "{}", first)?;
        for v in iter {
            write!(file, ",{}", v)?;
        }
    }
    writeln!(file)?;
    Ok(())
}
