use crate::{
    models::{Bundle, HoopTubePressureModel, TurbulentFlowModel},
    PipeVessel,
};
use gems::Cylinder;

#[derive(Clone, Debug)]
pub struct PipeBundle {
    pub shape: Cylinder,

    pub vessel: PipeVessel,

    pub port_velocity: [f64; 2],

    /// Pressure applied externally on ports A and B. Positive pressure is pushing into the tube.
    pub external_port_pressure: [f64; 2],

    /// Pressure applied externally over the whole hull. Positive pressure is contracting the hull.
    pub elasticity_pressure_model: Bundle<HoopTubePressureModel>,

    pub flow_model: Bundle<TurbulentFlowModel>,

    /// Angle of pipe relative to ground surface. Positive angle means port B is higher
    pub ground_angle: f64,

    /// Factor for turbulent flow, e.g. 64/2500
    pub darcy_factor: f64,

    /// Additional dampening factor
    pub dampening: f64,
}

impl PipeBundle {
    pub fn strand_count(&self) -> f64 {
        self.elasticity_pressure_model.count
    }

    pub fn inflow_velocity(&self) -> f64 {
        inwards(self.port_velocity[0], self.port_velocity[1])
    }

    pub fn flow_velocity(&self) -> f64 {
        through(self.port_velocity[0], self.port_velocity[1])
    }
}

fn inwards(a: f64, b: f64) -> f64 {
    a - b
}

fn through(a: f64, b: f64) -> f64 {
    a.max(b).min(0.) + a.min(b).max(0.)
}
