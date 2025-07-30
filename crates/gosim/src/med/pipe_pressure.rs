/// Pressure model for an elastic tube under positive pressure
///
/// Consider a tube with radius r0, wall thickness τ0 and constant length L.
/// The wall is a curved plate with cross section area `Ac = τ L` and length `l = 2 π r`.
/// The stress strain relation of the plate is E = σ/ε.
/// The strain in the plate is ε = (r - r0) / r0 by definition.
/// When the tube expands the wall thickens proportionally: τ = τ0 r / r0
/// Thus stress in the plate is: σ = E ε.
/// The stress is defines as σ = Ft / Ac, thus: Ft = E τ L ε
/// The correponding normal force is: Fn = Ft 2 π
/// The tube surface area is: An = 2 π r L
/// Pressure P = Fn / An = E τ L ε 2 π / (2 π r L)
///            = E τ0 r0 (r - r0) / r^3
pub fn elastic_tube_pressure(r: f64, r0: f64, t: f64, ym: f64) -> f64 {
    ym * t * (r0 / r) * ((r - r0) / (r * r))
}

/// Maximum pressure possible in the elastic tube
/// The maximum pressure is achived at r = 3/2 r0
pub fn elastic_tube_max_pressure(r0: f64, t: f64, ym: f64) -> f64 {
    4. * t * ym / (27. * r0)
}

/// Approximation Derivative of elastic_tube_pressure after r at r=r0
pub fn elastic_tube_inv_approx(p: f64, r0: f64, t: f64, ym: f64) -> f64 {
    let p = p.min(elastic_tube_max_pressure(r0, t, ym));

    r0 + p * r0 * r0 / (ym * t) * 1.5
    // Note the factor 1.5 just makes it a bit better on average ..
}

/// Tube law to model negative pressure
///
/// P = P0 * ((r/r0)^{2/n} - 1)
///
/// The exponent `n` is computed s.t. the tangent at r=r0 is the same as the tangent of the
/// elastic_tube_pressure law.
pub fn tube_law_pressure(r: f64, r0: f64, t: f64, ym: f64, pmin: f64) -> f64 {
    let n = -2. * pmin * r0 / (ym * t);
    pmin * (1. - (r / r0).powf(2. / n))
}
