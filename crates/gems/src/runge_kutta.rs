//! Runge-Kutta methods for solving ODE's
//!
//! Given an ODE y'(t) = f(t, y) compute the next value for y under given timestep h.
//!
//! Reference: https://en.wikipedia.org/wiki/Runge%E2%80%93Kutta_methods

// /// An ODE f in the form y' = f(t, y)
// pub trait CanonicalODE {
//     fn evaluate(&self, t: f64, y: f64) -> f64;
// }

// impl<F: CanonicalODE> CanonicalODE for &F {
//     fn evaluate(&self, t: f64, y: f64) -> f64 {
//         (*self).evaluate(t, y)
//     }
// }

use std::ops::{Add, Mul, Sub};

/// Simple forward integration - fast but inaccurate
pub fn forward_integrate<D: Dom>(t: f64, y: D, h: f64, f: impl ODE<D>) -> D {
    let d = f.eval(t, y.clone());
    y + d * h
}

/// Runge Kutta 4 (RK4)
pub fn runge_kutta_4<D: Dom>(t: f64, y: D, h: f64, f: impl ODE<D>) -> D {
    let h2: f64 = h * 0.5;
    let k1 = f.eval(t, y.clone());
    let k2 = f.eval(t + h2, y.clone() + k1.clone() * h2);
    let k3 = f.eval(t + h2, y.clone() + k2.clone() * h2);
    let k4 = f.eval(t + h, y.clone() + k3.clone() * h);
    y + (k1 + (k2 + k3) * 2. + k4) * (h / 6.)
}

/// Runge Kutta 3/8-rule: a little bit more accurate than RK4 but uses more flops
pub fn runge_kutta_3_8<D: Dom>(t: f64, y: D, h: f64, f: impl ODE<D>) -> D {
    let h3: f64 = h / 3.;
    let k1 = f.eval(t, y.clone());
    let k2 = f.eval(t + h3, y.clone() + k1.clone() * h3);
    let k3 = f.eval(t + 2. * h3, y.clone() - k1.clone() * h3 + k2.clone() * h);
    let k4 = f.eval(
        t + h,
        y.clone() + (k1.clone() - k2.clone() + k3.clone()) * h,
    );
    y + (k1 + (k2 + k3) * 3. + k4) * (h / 8.)
}

// Ralston method: less accurate and cheaper than RK4
pub fn runge_kutta_ralston<D: Dom>(t: f64, y: D, h: f64, f: impl ODE<D>) -> D {
    let h23 = h * (2. / 3.);
    let k1 = f.eval(t, y.clone());
    let k2 = f.eval(t + h23, y.clone() + k1.clone() * h23);
    y + (k1 + k2 * 3.) * (h / 4.)
}

pub trait Dom:
    Clone + Add<Self, Output = Self> + Sub<Self, Output = Self> + Mul<f64, Output = Self>
{
}

impl Dom for f64 {}

impl<const N: usize> Dom for nalgebra::SVector<f64, N> {}

impl Dom for nalgebra::DVector<f64> {}

pub trait ODE<D> {
    fn eval(&self, time: f64, state: D) -> D;

    // fn solve(&self, time: f64, state: D, dt: f64, method: Method) -> D
    // where
    //     D: Dom,
    //     Self: Sized,
    // {
    //     match method {
    //         Method::RK_4 => runge_kutta_4(time, state, dt, self),
    //         Method::RK_3_8 => runge_kutta_3_8(time, state, dt, self),
    //         Method::Ralston => runge_kutta_ralston(time, state, dt, self),
    //     }
    // }
}

// pub enum Method {
//     RK_4,
//     RK_3_8,
//     Ralston,
// }

pub struct FnODE<F>(F);

impl<D, F> ODE<D> for FnODE<F>
where
    F: Fn(f64, D) -> D,
{
    fn eval(&self, time: f64, state: D) -> D {
        (self.0)(time, state)
    }
}

// impl<D, F> ODE<D> for F
// where
//     F: Fn(f64, D) -> D,
// {
//     fn eval(&self, time: f64, state: D) -> D {
//         (*self)(time, state)
//     }
// }

impl<D: Dom, X: ODE<D>> ODE<D> for &X {
    fn eval(&self, time: f64, state: D) -> D {
        (*self).eval(time, state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::Vector2;

    #[test]
    fn test_runge_kutta_4() {
        let actual = runge_kutta_4(0., 1., 0.025, FnODE(|_, y: f64| y.tan() + 1.));
        approx::assert_abs_diff_eq!(actual, 1.066970994, epsilon = 1e-9);
    }

    #[test]
    fn test_runge_kutta_3_8() {
        let actual = runge_kutta_3_8(0., 1., 0.025, FnODE(|_, y: f64| y.tan() + 1.));
        approx::assert_abs_diff_eq!(actual, 1.066970909, epsilon = 1e-9);
    }

    #[test]
    fn test_runge_kutta_ralston() {
        let actual = runge_kutta_ralston(0., 1., 0.025, FnODE(|_, y: f64| y.tan() + 1.));
        approx::assert_abs_diff_eq!(actual, 1.066869388, epsilon = 1e-9);
    }

    #[test]
    fn test_runge_kutta_flow() {
        let accel = |_, xv: Vector2<f64>| -> Vector2<f64> {
            use core::f64::consts::PI;
            let pressure = 10_000.;
            let r = 0.01;
            let density = 1e3;
            let viscosity = 1e-3;
            let darcy = 64. / 2500.;
            let x = xv[0];
            let v = xv[1];
            let dx = v;
            let dv = pressure / (x * density)
                - PI * darcy / (4. * r) * v * v
                - 8. * viscosity / (r * r * density) * v.abs();
            Vector2::new(dx, dv)
        };
        let mut v = Vector2::new(0.10, 0.);
        for i in 0..10 {
            v = runge_kutta_3_8((i as f64) * 0.020, v, 0.020, FnODE(accel));
        }
        approx::assert_abs_diff_eq!(v[0], 0.730643, epsilon = 1e-4);
        approx::assert_abs_diff_eq!(v[1], 3.13828, epsilon = 1e-4);
    }
}
