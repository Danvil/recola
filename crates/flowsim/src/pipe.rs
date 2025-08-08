use crate::{
    models::{Bundle, HoopTubePressureModel},
    FluidDensityViscosity, PortMap, PortTag,
};
use gems::Cylinder;
use std::ops::{Add, Mul, Sub};

#[derive(Clone, Debug)]
pub struct PipeDef {
    pub shape: Bundle<Cylinder>,

    /// Fluid stored in the pipe
    pub fluid: FluidDensityViscosity,

    /// Pressure applied externally on ports A and B. Positive pressure is pressing liquid out of
    /// the tube. For example a pump pumping liquid from port A to B could apply a positive
    /// pressure on port A.
    pub external_port_pressure: PortMap<f64>,

    /// Pressure applied externally over the whole hull. Positive pressure is contracting the hull.
    pub elasticity_pressure_model: Bundle<HoopTubePressureModel>,

    /// Angle of pipe relative to ground surface. Positive angle means port B is higher
    pub ground_angle: f64,

    /// Factor for turbulent flow, e.g. 64/2500
    pub darcy_factor: f64,

    /// Additional dampening factor
    pub dampening: f64,

    /// The port area is scaled with this factor. If set to 0 the port is closed.
    // FIXME not implemented
    pub port_area_factor: [f64; 2],
}

impl PipeDef {
    pub fn strand_count(&self) -> f64 {
        self.elasticity_pressure_model.count
    }
}

#[derive(Clone, Debug, Default)]
pub struct PipeState {
    /// Volume stored in the pipe
    pub volume: f64,

    /// Flow velocity at ports (positive flows inwards)
    pub velocity: PortMap<f64>,
}

impl PipeState {
    pub fn inflow_velocity(&self) -> f64 {
        inwards(self.velocity[PortTag::A], self.velocity[PortTag::B])
    }

    pub fn throughflow_velocity(&self) -> f64 {
        through(self.velocity[PortTag::A], self.velocity[PortTag::B])
    }
}

fn inwards(a: f64, b: f64) -> f64 {
    a - b
}

fn through(a: f64, b: f64) -> f64 {
    a.max(-b).min(0.) + a.min(-b).max(0.)
}

impl Add for PipeState {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            volume: self.volume + rhs.volume,
            velocity: self.velocity + rhs.velocity,
        }
    }
}

impl Sub for PipeState {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            volume: self.volume - rhs.volume,
            velocity: self.velocity - rhs.velocity,
        }
    }
}

impl Mul<f64> for PipeState {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self {
        Self {
            volume: self.volume * rhs,
            velocity: self.velocity * rhs,
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct PipeScratch {
    /// Radius of a single pipe in the bundle (at current volume)
    pub strand_radius: f64,

    /// Number of strands in the bundle. All properties not prefixed with strand_ are are wrt to
    /// the whole bundel.
    pub strand_count: f64,

    /// Cross section area of the pipe bundle (at current volume)
    pub cross_section_area: f64,

    /// Mass of liquid contained in the pipe
    pub volume: f64,

    /// Mass of liquid contained in the pipe
    pub mass: f64,

    pub pump_force: [f64; 2],
    pub grav_force: [f64; 2],
    pub elas_force: f64,
    pub visc_force: [f64; 2],
    pub turb_force: [f64; 2],
    pub damp_force: [f64; 2],

    /// Force acting on the ports of a pipe. Positive force pushes inwards
    pub force: [f64; 2],

    /// Junction pressure at ports
    pub junction_pressure: [Option<f64>; 2],
}

/// Solution variables for one pipe
#[derive(Clone, Debug, Default)]
pub struct PipeSolution {
    /// Flow speed at each port
    pub velocity: PortMap<f64>,

    /// Volume of fluid which flowed through each port during the timestep
    pub delta_volume: PortMap<f64>,
}
