/// Newton's algorithm to find the root of a function, i.e. find x s.t. f(x) = 0
pub fn newton_root_solver<F, DF>(
    x0: f64,
    accuracy: f64,
    max_iterations: usize,
    mut obj_f: F,
    mut dx_f: DF,
) -> Result<f64, f64>
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
            return Err(x);
        }

        let dx = obj_f(x) / m;
        x = x - dx;
    }
    Err(x)
}
