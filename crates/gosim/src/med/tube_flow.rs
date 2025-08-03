use crate::Cylinder;

/// Poiseuille model for laminar flow
#[derive(Clone, Default, Debug)]
pub struct PoiseuilleFlowModel {
    poiseuille_conductance: f64,
}

impl PoiseuilleFlowModel {
    pub fn new(cylinder: Cylinder, viscosity: f64, conductance_factor: f64) -> Self {
        assert!(cylinder.is_non_zero());
        assert!(viscosity > 0.);
        assert!(conductance_factor >= 0.);

        let poiseuille_conductance =
            poiseuille_pipe_conductance(cylinder.radius, cylinder.length, viscosity)
                * conductance_factor;

        Self {
            poiseuille_conductance,
        }
    }

    pub fn apply_conductance_factor(&mut self, factor: f64) {
        self.poiseuille_conductance *= factor;
    }

    pub fn poiseuille_conductance(&self) -> f64 {
        self.poiseuille_conductance
    }

    /// Flow for given pressure differential
    pub fn flow(&self, pressure_difference: f64) -> f64 {
        self.poiseuille_conductance * pressure_difference
    }

    /// Derivative of flow
    pub fn flow_dx(&self, _pressure_difference: f64) -> f64 {
        self.poiseuille_conductance
    }
}

/// Conductance of a cylindrical pipe (Poiseuille's law)
pub fn poiseuille_pipe_conductance(radius: f64, length: f64, viscosity: f64) -> f64 {
    core::f64::consts::PI * radius.powi(4) / (8. * length * viscosity)
}

/// Resistance of a cylindrical pipe (Poiseuille's law)
pub fn poiseuille_pipe_resistance(radius: f64, length: f64, viscosity: f64) -> f64 {
    8. * length * viscosity / (core::f64::consts::PI * radius.powi(4))
}

/// A non-physical model which follows Poiseuille until critical pressure and afterwards is
/// proportional to sqrt(dP)
#[derive(Clone, Default, Debug)]
pub struct TurbulentFlowModel {
    poiseuille_conductance: f64,
    critical_pressure: f64,
}

impl TurbulentFlowModel {
    /// Reynolds number around which laminar flow transitions to turbulent flow
    pub const CRITICAL_REYNOLDS: f64 = 1500.;

    pub fn new(cylinder: Cylinder, density: f64, viscosity: f64, conductance_factor: f64) -> Self {
        assert!(cylinder.is_non_zero());
        assert!(density > 0.);
        assert!(viscosity > 0.);
        assert!(conductance_factor >= 0.);

        let poiseuille_conductance =
            poiseuille_pipe_conductance(cylinder.radius, cylinder.length, viscosity)
                * conductance_factor;

        let critical_pressure = critical_pressure(
            Self::CRITICAL_REYNOLDS,
            cylinder.radius,
            cylinder.length,
            density,
            viscosity,
        ) * conductance_factor;

        Self {
            poiseuille_conductance,
            critical_pressure,
        }
    }

    pub fn apply_conductance_factor(&mut self, factor: f64) {
        self.poiseuille_conductance *= factor;
        self.critical_pressure *= factor;
    }

    pub fn poiseuille_conductance(&self) -> f64 {
        self.poiseuille_conductance
    }

    /// Computes flow for given pressure differential
    pub fn flow(&self, pressure_difference: f64) -> f64 {
        self.poiseuille_conductance * Self::curve(pressure_difference, self.critical_pressure)
    }

    /// Computes derivative of flow
    pub fn flow_dx(&self, pressure_difference: f64) -> f64 {
        self.poiseuille_conductance * Self::curve_dx(pressure_difference, self.critical_pressure)
    }

    fn curve(x: f64, x0: f64) -> f64 {
        if x < 0. {
            -Self::curve(-x, x0)
        } else {
            if x <= x0 {
                x
            } else {
                ((2. * x - x0) * x0).sqrt()
            }
        }
    }

    fn curve_dx(x: f64, x0: f64) -> f64 {
        if x0 <= 0. {
            0.
        } else {
            let x = x.abs();
            if x <= x0 {
                1.
            } else {
                x0 / ((2. * x - x0) * x0).sqrt()
            }
        }
    }
}

/// Computes pressure (after poiseuille) which realizes given Reynolds number
pub fn critical_pressure(
    reynolds_number: f64,
    radius: f64,
    length: f64,
    density: f64,
    viscosity: f64,
) -> f64 {
    reynolds_number * 4. * viscosity * viscosity * length / (radius.powi(3) * density)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DENSITY_BLOOD, VISCOSITY_BLOOD};

    #[test]
    fn test_turbulent_flow() {
        let m = TurbulentFlowModel::new(
            Cylinder {
                radius: 0.001,
                length: 0.05,
            },
            DENSITY_BLOOD,
            VISCOSITY_BLOOD,
            1.0,
        );
        approx::assert_relative_eq!(m.flow(1_000.), 2.24399e-6, max_relative = 1e-4);

        let m = TurbulentFlowModel::new(
            Cylinder {
                radius: 0.012,
                length: 0.35,
            },
            DENSITY_BLOOD,
            VISCOSITY_BLOOD,
            0.1,
        );
        approx::assert_relative_eq!(m.flow(1_000.), 3.4971556257424075e-5, max_relative = 1e-4);
    }

    #[test]
    fn test_turbulent_flow_dx() {
        let model = TurbulentFlowModel::new(
            Cylinder {
                radius: 0.010,
                length: 1.000,
            },
            DENSITY_BLOOD,
            VISCOSITY_BLOOD,
            1.,
        );

        for x in [-1000., -100., 0., 100., 1000.] {
            let y1 = model.flow(x);
            let dx = 0.001;
            let y2 = model.flow(x + dx);
            let expected = (y2 - y1) / dx;
            let actual = model.flow_dx(x + 0.5 * dx);
            approx::assert_relative_eq!(actual, expected, max_relative = 1e-4);
        }
    }
}
