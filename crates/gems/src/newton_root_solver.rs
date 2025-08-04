/// Newton's algorithm to find the root of a function, i.e. find x s.t. f(x) = 0
pub fn newton_root_solver<F, DF>(
    x0: f64,
    accuracy: f64,
    max_iterations: usize,
    mut obj_f: F,
    mut dx_f: DF,
) -> Result<f64, NewtonRootSolverError>
where
    F: FnMut(f64) -> f64,
    DF: FnMut(f64) -> f64,
{
    let mut x = x0;
    for _i in 0..max_iterations {
        let y = obj_f(x);
        let m = dx_f(x);

        // println!("{i}: x: {x}, y: {y}, m: {m}");

        if y.abs() < accuracy {
            return Ok(x);
        }

        if m.abs() < 1e-9 {
            return Err(NewtonRootSolverError::Plateau { x, y, m });
        }

        x -= y / m;
    }
    Err(NewtonRootSolverError::IterationCountExceeded { x })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NewtonRootSolverError {
    Plateau { x: f64, y: f64, m: f64 },
    IterationCountExceeded { x: f64 },
}

impl NewtonRootSolverError {
    pub fn best_guess(&self) -> f64 {
        match *self {
            NewtonRootSolverError::Plateau { x, .. } => x,
            NewtonRootSolverError::IterationCountExceeded { x } => x,
        }
    }
}
