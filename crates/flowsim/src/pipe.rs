use crate::{
    models::{Bundle, HoopTubePressureModel},
    FluidDensityViscosity, PortMap, PortTag,
};
use gems::Cylinder;
use simplecs::prelude::*;
use std::ops::{Add, Mul, Sub};

#[derive(Component, Clone, Debug)]
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
    pub port_area_factor: PortMap<f64>,
}

impl PipeDef {
    pub fn strand_count(&self) -> f64 {
        self.elasticity_pressure_model.count
    }
}

#[derive(Component, Clone, Debug, Default)]
pub struct PipeState {
    /// Volume stored in the pipe
    pub volume: f64,

    /// Flow velocity at ports (positive flows inwards)
    pub velocity: PortMap<f64>,
}

#[derive(Component, Clone, Debug, Default)]
pub struct PipeStateDerivative {
    /// Change of volume stored in the pipe (total out/inflow over both ports)
    pub flow: f64,

    /// Total liquid acceleration on ports
    pub accel: PortMap<f64>,
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
    a + b
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

#[derive(Component, Default, Clone, Debug)]
pub struct PipeScratch {
    /// Number of strands in the bundle. All properties not prefixed with strand_ are are wrt to
    /// the whole bundel.
    pub strand_count: f64,

    /// Radius of a single pipe in the bundle over most of the length of the tube
    pub tube_strand_radius: f64,

    /// Cross section area of the pipe bundle at the ports
    pub port_cross_section_area: [f64; 2],

    /// Cross section area of the pipe bundle over most of the length of the tube
    pub tube_cross_section_area: f64,

    /// Area/Mass used for port pressure equalization
    pub area_per_mass: f64,

    /// Mass of liquid contained in the pipe
    pub volume: f64,

    /// Mass of liquid contained in the pipe
    pub mass: f64,

    pub elas_pressure: f64,

    pub pump_accel: [f64; 2],
    pub grav_accel: [f64; 2],
    pub elas_accel: f64,
    pub visc_force: [f64; 2],
    pub turb_force: [f64; 2],
    pub damp_force: [f64; 2],

    /// Liquid acceleration on ports based on external effects like gravity or pump.
    pub ext_accels: [f64; 2],

    /// Sum of drag forcesopposing liquid movement.
    pub drag_forces: [f64; 2],

    /// Junction pressure at ports
    pub junction_pressure: [Option<f64>; 2],
}

/// Solution variables for one pipe
#[derive(Component, Clone, Debug, Default)]
pub struct PipeSolution {
    /// Flow speed at each port
    pub velocity: PortMap<f64>,

    /// Volume of fluid which flowed through each port during the timestep
    pub delta_volume: PortMap<f64>,
}
#[derive(Component, Default)]
pub struct SolutionDeltaVolume {
    pub delta_volume: PortMap<f64>,
}

#[derive(Component, Default, Clone, Debug)]
pub struct JunctionScratch {
    pub(crate) pressure: Option<f64>,
    pub(crate) supply_count: usize,
    pub(crate) demand_count: usize,
    pub(crate) supply: f64,
    pub(crate) demand: f64,
    pub(crate) supply_fullfillment: f64,
    pub(crate) demand_fullfillment: f64,
}

/// Indicates that a pipe is connected to a junction and provides the pipe port.
#[derive(Component)]
pub struct PipeJunctionPort(pub PortTag);
