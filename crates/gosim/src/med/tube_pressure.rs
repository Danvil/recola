use crate::{
    cylinder_radius, cylinder_volume, newton_root_solver, Cylinder, NewtonRootSolverError,
};

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
pub fn balloon_tube_pressure(r: f64, r0: f64, t: f64, ym: f64) -> f64 {
    ym * t * (r0 / r) * ((r - r0) / (r * r))
}

/// Maximum pressure possible in the elastic tube
/// The maximum pressure is achived at r = 3/2 r0
pub fn balloon_tube_max_pressure(r0: f64, t: f64, ym: f64) -> f64 {
    4. * t * ym / (27. * r0)
}

/// The volume of the tube when maximum pressure is reached
pub fn balloon_tube_volume_at_max_pressure(r0: f64, length: f64) -> f64 {
    9. / 4. * core::f64::consts::PI * length * r0 * r0
}

/// Approximation Derivative of balloon_tube_pressure after r at r=r0
pub fn balloon_tube_inv_approx(p: f64, r0: f64, t: f64, ym: f64) -> f64 {
    let p = p.min(balloon_tube_max_pressure(r0, t, ym));

    r0 + p * r0 * r0 / (ym * t) * 1.5
    // Note the factor 1.5 just makes it a bit better on average ..
}

pub fn hoop_tube_pressure(r: f64, r0: f64, t: f64, ym: f64) -> f64 {
    (t * ym * (r - r0)) / (r * r0)
}

/// Tube law to model negative pressure
///
/// P = P0 * ((r/r0)^{2/n} - 1)
///
/// The exponent `n` is computed s.t. the tangent at r=r0 is the same as the tangent of the
/// balloon_tube_pressure law.
pub fn tube_law_pressure(r: f64, r0: f64, t: f64, ym: f64, pmin: f64) -> f64 {
    let n = -2. * pmin * r0 / (ym * t);
    pmin * (1. - (r / r0).powf(2. / n))
}

/// Geometry and mechanical properties of a bundle of elastic tubes
#[derive(Clone, Debug)]
pub struct ElasticTubeBundle {
    /// Radius of one tube
    pub radius: f64,

    /// Length of tubes
    pub length: f64,

    /// Wall thickness of one tube
    pub wall_thickness: f64,

    /// Young's modulus describing elasticity of the tube wall
    pub youngs_modulus: f64,

    /// Number of parallel tubes in the bundle. More tubes for example allow higher storage volume
    /// without affecting tube conductance.
    pub count: f64,
}

impl Default for ElasticTubeBundle {
    fn default() -> Self {
        Self {
            radius: 0.005,
            length: 1.000,
            wall_thickness: 0.001,
            youngs_modulus: 1_000_000.0,
            count: 1.,
        }
    }
}

impl ElasticTubeBundle {
    pub fn cylinder(&self) -> Cylinder {
        Cylinder {
            radius: self.radius,
            length: self.length,
        }
    }

    /// Nominal volume of all tubes in the bundle
    pub fn nominal_volume(&self) -> f64 {
        self.radius_to_volume(self.radius)
    }

    /// Computes radius based on given volume
    pub fn volume_to_radius(&self, volume: f64) -> f64 {
        cylinder_radius(volume / self.count, self.length)
    }

    /// Computes volume based on given radius
    pub fn radius_to_volume(&self, radius: f64) -> f64 {
        cylinder_volume(radius, self.length) * self.count
    }

    pub fn with_cylinder(mut self, cylinder: Cylinder) -> Self {
        self.radius = cylinder.radius;
        self.length = cylinder.length;
        self
    }

    pub fn with_radius(mut self, radius: f64) -> Self {
        self.radius = radius;
        self
    }

    pub fn with_length(mut self, length: f64) -> Self {
        self.length = length;
        self
    }

    /// Sets the tube count such that all tubes in the bundle combined store given volume
    pub fn with_count_from_total_volume(mut self, volume: f64) -> Self {
        self.count = volume / cylinder_volume(self.radius, self.length);
        self
    }
}

/// Hoop stress is P r / t
#[derive(Default, Clone, Debug)]
pub struct HoopTubePressureModel {
    tubes: ElasticTubeBundle,
    collapse_pressure: f64,
}

impl HoopTubePressureModel {
    pub fn new(tubes: ElasticTubeBundle, collapse_pressure: f64) -> Self {
        Self {
            tubes,
            collapse_pressure,
        }
    }

    pub fn tubes(&self) -> &ElasticTubeBundle {
        &self.tubes
    }

    pub fn pressure(&self, volume: f64) -> f64 {
        let current_radius = self.tubes.volume_to_radius(volume);

        if current_radius < self.tubes.radius {
            tube_law_pressure(
                current_radius,
                self.tubes.radius,
                self.tubes.wall_thickness,
                self.tubes.youngs_modulus,
                self.collapse_pressure,
            )
        } else {
            hoop_tube_pressure(
                current_radius,
                self.tubes.radius,
                self.tubes.wall_thickness,
                self.tubes.youngs_modulus,
            )
        }
    }

    /// Derivative of `pressure`
    pub fn pressure_dx(&self, volume: f64) -> f64 {
        let dv = (volume * 1e-4).max(1e-9);

        let p1 = self.pressure(volume);
        let p2 = self.pressure(volume + dv);

        (p2 - p1) / dv
    }

    /// Computes volume which generates given pressure.
    pub fn volume(&self, pressure: f64) -> Result<f64, NewtonRootSolverError> {
        // TODO there is a closed form solution
        // solve P(V) - P0 == 0 for V
        let obj_f = |v| self.pressure(v) - pressure;
        let dx_f = |v| self.pressure_dx(v);
        let v0 = self.tubes.nominal_volume();
        let sol = newton_root_solver(v0, 1e-3, 25, obj_f, dx_f);
        sol
    }
}

/// Pressure model assuming an elastic wall which thins due to expansion. Similar to inflating a
/// balloon. Under this model pressure drops again after a certain volume has been reached.
#[derive(Default, Clone, Debug)]
pub struct BalloonTubePressureModel {
    tubes: ElasticTubeBundle,
    collapse_pressure: f64,
}

impl BalloonTubePressureModel {
    pub fn new(tubes: ElasticTubeBundle, collapse_pressure: f64) -> Self {
        Self {
            tubes,
            collapse_pressure,
        }
    }

    pub fn tubes(&self) -> &ElasticTubeBundle {
        &self.tubes
    }

    pub fn pressure(&self, volume: f64) -> f64 {
        let current_radius = self.tubes.volume_to_radius(volume);

        if current_radius < self.tubes.radius {
            tube_law_pressure(
                current_radius,
                self.tubes.radius,
                self.tubes.wall_thickness,
                self.tubes.youngs_modulus,
                self.collapse_pressure,
            )
        } else {
            balloon_tube_pressure(
                current_radius,
                self.tubes.radius,
                self.tubes.wall_thickness,
                self.tubes.youngs_modulus,
            )
        }
    }

    /// Derivative of `pressure`
    pub fn pressure_dx(&self, volume: f64) -> f64 {
        let dv = (volume * 1e-4).max(1e-9);

        let p1 = self.pressure(volume);
        let p2 = self.pressure(volume + dv);

        (p2 - p1) / dv
    }

    /// Maximum pressure possible
    pub fn max_pressure(&self) -> f64 {
        balloon_tube_max_pressure(
            self.tubes.radius,
            self.tubes.wall_thickness,
            self.tubes.youngs_modulus,
        )
    }

    /// Volume when maximum pressure is reached. Higher volume will lead to pressure drop.
    pub fn volume_at_max_pressure(&self) -> f64 {
        balloon_tube_volume_at_max_pressure(self.tubes.radius, self.tubes.length) * self.tubes.count
    }

    /// Computes volume which generates given pressure. Note that this might not have a solution
    /// for high pressure as pressure drops again with high volume. If multiple solutions exist the
    /// one with smaller volume is given.
    pub fn volume(&self, pressure: f64) -> Result<f64, f64> {
        let max_pressure = self.max_pressure();
        if pressure > max_pressure {
            Err(self.volume_at_max_pressure())
        } else {
            // solve P(V) - P0 == 0 for V
            let obj_f = |v| self.pressure(v) - pressure;
            let dx_f = |v| self.pressure_dx(v);
            let v0 = self.tubes.nominal_volume();
            let sol = newton_root_solver(v0, 1e-3, 25, obj_f, dx_f);
            sol.map_err(|err| err.best_guess())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balloon_tube_pressure_model_volume() {
        let m = BalloonTubePressureModel::new(ElasticTubeBundle::default(), -1_000.);

        let v0 = m.tubes.nominal_volume();

        // checked with mathematica
        let pmax = m.max_pressure();
        approx::assert_relative_eq!(pmax, 29629.6, max_relative = 1e-4);

        // checked with mathematica
        let vmax = m.volume_at_max_pressure();
        approx::assert_relative_eq!(vmax, 0.176715e-3, max_relative = 1e-4);

        // 0 pressure gives nominal volume
        approx::assert_relative_eq!(m.volume(0.).unwrap(), v0, max_relative = 1e-4);

        // max pressure gives max volume
        approx::assert_relative_eq!(m.volume(pmax).unwrap(), vmax, max_relative = 1e-4);

        // checked with mathematica
        approx::assert_relative_eq!(
            m.volume(0.5 * pmax).unwrap(),
            9.47010620333547e-5,
            max_relative = 1e-4
        );

        // pressure(vol(P)) == P
        for q in [0.01, 0.2, 0.35, 0.67, 0.99] {
            let expected = q * pmax;
            let v = m.volume(q * pmax).unwrap();
            let actual = m.pressure(v);
            approx::assert_relative_eq!(actual, expected, max_relative = 1e-4);
        }
    }
}
