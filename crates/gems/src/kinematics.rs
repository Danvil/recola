/// Velocity after joining two bodies
/// Kinetic energy: m/2 v^2
pub fn joint_velocity(v1: f64, m1: f64, v2: f64, m2: f64) -> f64 {
    let e = kinetic_energy(v1, m1) + kinetic_energy(v2, m2);
    let m = m1 + m2;
    if m > 0. { (2. * e / m).sqrt() } else { 0. }
}

pub fn kinetic_energy(v: f64, m: f64) -> f64 {
    0.5 * m * v * v
}

pub const GRAVITY_CONSTANT: f64 = 9.81;
