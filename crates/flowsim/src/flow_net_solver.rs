use crate::{
    apply_solution, compute_solution, increment_solution, FlowNet, FlowNetOde, PipeId,
    PipeSolution, PipeState,
};
use gems::{runge_kutta_3_8, Dom, IntMap};
use std::ops::{Add, Index, IndexMut, Mul, Sub};

#[derive(Debug)]
pub struct FlowNetSolver {
    max_sub_steps: usize,
}

impl FlowNetSolver {
    pub fn new() -> Self {
        Self { max_sub_steps: 1 }
    }

    pub fn step<'a>(
        &self,
        net: &'a FlowNet,
        mut state: IntMap<PipeState>,
        dt: f64,
    ) -> (FlowNetOde<'a>, IntMap<PipeSolution>, IntMap<PipeState>) {
        let min_sub_dt = dt / self.max_sub_steps as f64;

        let ode = FlowNetOde::new(net);

        let mut sol = None;

        let mut remaining_dt = dt;
        loop {
            let sub_dt = min_sub_dt;

            let next = runge_kutta_3_8(0., DataColumn(state.clone()), sub_dt, &ode);
            // let next = forward_integrate(0., state_vec, sub_dt, &ode);

            let sub_sol = compute_solution(&ode, &next.0, sub_dt);
            apply_solution(&mut state, &sub_sol);

            match sol.as_mut() {
                Some(sol) => increment_solution(&sub_sol, sol),
                None => sol = Some(sub_sol),
            }

            remaining_dt -= sub_dt;
            if remaining_dt <= 0. {
                return (ode, sol.unwrap(), state);
            }
        }
    }

    pub fn solve(
        &self,
        net: &FlowNet,
        mut state: IntMap<PipeState>,
        steps: usize,
        dt: f64,
    ) -> IntMap<PipeState> {
        for _i in 0..steps {
            // println!("ITERATION {i:?}",);
            let (ode, _sol, next) = self.step(net, state.clone(), dt);
            state = next;

            ode.print_overview(&state);

            // let derivative = ode.eval(0., state_vec.clone());
            // write_vector_to_csv(
            //     &derivative,
            //     &format!(
            //         "I:/Ikabur/gos/tmp/{:03}_{:03}_derivative.csv",
            //         k, step_count
            //     ),
            // )
            // .unwrap();

            // write_vector_to_csv(
            //     &next,
            //     &format!("I:/Ikabur/gos/tmp/{:03}_{:03}_state.csv", k, step_count),
            // )
            // .unwrap();
        }
        state
    }
}

// fn write_vector_to_csv<D, S>(
//     vec: &nalgebra::Vector<f64, D, S>,
//     path: &str,
// ) -> Result<(), Box<dyn Error>>
// where
//     D: nalgebra::Dim,
//     S: nalgebra::Storage<f64, D>,
//     nalgebra::DefaultAllocator: nalgebra::allocator::Allocator<D>,
// {
//     let mut file = File::create(path)?;
//     let mut iter = vec.iter();
//     if let Some(first) = iter.next() {
//         write!(file, "{}", first)?;
//         for v in iter {
//             write!(file, ",{}", v)?;
//         }
//     }
//     writeln!(file)?;
//     Ok(())
// }

/// Wrapper around IntMap which implements Dom and thus can be used with ODE solvers.
#[derive(Clone, Debug)]
pub struct DataColumn<T>(pub IntMap<T>);

// impl<T> DataColumn<T> {
//     pub fn from_count<F>(count: usize, f: F) -> Self
//     where
//         F: Fn(usize) -> T,
//     {
//         DataColumn(IntMap::from_count(count, f))
//     }

//     pub fn pipe_count(&self) -> usize {
//         self.0.slot_count()
//     }

//     pub fn clear(&mut self) {
//         self.0.clear()
//     }
// }

impl<T> Index<PipeId> for DataColumn<T> {
    type Output = T;

    fn index(&self, id: PipeId) -> &T {
        &self.0[*id]
    }
}

impl<T> IndexMut<PipeId> for DataColumn<T> {
    fn index_mut(&mut self, id: PipeId) -> &mut T {
        &mut self.0[*id]
    }
}

impl<T> Dom for DataColumn<T> where
    T: Clone + Mul<f64, Output = T> + Sub<Output = T> + Add<Output = T>
{
}

impl<T> Add for DataColumn<T>
where
    T: Clone + Add<T, Output = T>,
{
    type Output = Self;

    fn add(self, rhs: DataColumn<T>) -> Self {
        DataColumn(IntMap::from_iter(
            IntMap::zip_iter(&self.0, &rhs.0).filter_map(|(i, a, b)| match (a, b) {
                (Some(a), Some(b)) => Some((i, a.clone() + b.clone())),
                (Some(x), None) | (None, Some(x)) => Some((i, x.clone())),
                (None, None) => None,
            }),
        ))
    }
}

impl<T> Sub for DataColumn<T>
where
    T: Clone + Sub<T, Output = T>,
{
    type Output = Self;

    fn sub(self, rhs: DataColumn<T>) -> Self {
        DataColumn(IntMap::from_iter(
            IntMap::zip_iter(&self.0, &rhs.0).filter_map(|(i, a, b)| match (a, b) {
                (Some(a), Some(b)) => Some((i, a.clone() - b.clone())),
                (Some(x), None) | (None, Some(x)) => Some((i, x.clone())),
                (None, None) => None,
            }),
        ))
    }
}

impl<T> Mul<f64> for DataColumn<T>
where
    T: Clone + Mul<f64, Output = T>,
{
    type Output = Self;

    fn mul(self, rhs: f64) -> Self {
        DataColumn(self.0.map(|x| x.clone() * rhs))
    }
}
