use crate::FlowNetOde;
use gems::runge_kutta_3_8;
use simplecs::prelude::*;

#[derive(Debug)]
pub struct FlowNetSolver {
    max_sub_steps: usize,
}

impl FlowNetSolver {
    pub fn new() -> Self {
        Self { max_sub_steps: 1 }
    }

    pub fn step<'a>(&self, world: &'a mut World, dt: f64) {
        let min_sub_dt = dt / self.max_sub_steps as f64;

        let ode = FlowNetOde::new(world);

        let mut state = ode.init_state();

        let mut remaining_dt = dt;
        loop {
            let sub_dt = min_sub_dt;

            let next = runge_kutta_3_8(0., state.clone(), sub_dt, &ode);
            ode.step(sub_dt, next.clone());
            state = next;

            remaining_dt -= sub_dt;
            if remaining_dt <= 0. {
                return;
            }
        }
    }

    // pub fn solve(&self, world: &mut World, steps: usize, dt: f64) {
    //     for _i in 0..steps {
    //         // println!("ITERATION {i:?}",);
    //         self.step(world, dt);

    //         print_overview(world);

    //         // let derivative = ode.eval(0., state_vec.clone());
    //         // write_vector_to_csv(
    //         //     &derivative,
    //         //     &format!(
    //         //         "I:/Ikabur/gos/tmp/{:03}_{:03}_derivative.csv",
    //         //         k, step_count
    //         //     ),
    //         // )
    //         // .unwrap();

    //         // write_vector_to_csv(
    //         //     &next,
    //         //     &format!("I:/Ikabur/gos/tmp/{:03}_{:03}_state.csv", k, step_count),
    //         // )
    //         // .unwrap();
    //     }
    // }
}
