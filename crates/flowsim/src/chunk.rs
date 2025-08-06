use crate::{Fluid, Mix};

/// A chunk of fluid
#[derive(Clone, Debug, Default)]
pub struct FluidChunk {
    fluid: Fluid,

    volume: f64,
    mass: f64,
    density: f64,
    viscosity: f64,
}

impl FluidChunk {
    pub fn from_fluid(fluid: Fluid) -> Self {
        let volume = fluid.volume();
        let mass = fluid.mass();
        let viscosity = fluid.viscosity();
        Self {
            fluid,
            volume,
            mass,
            density: mass / volume,
            viscosity,
        }
    }

    pub fn fluid(&self) -> &Fluid {
        &self.fluid
    }

    pub fn volume(&self) -> f64 {
        self.volume
    }

    pub fn mass(&self) -> f64 {
        self.mass
    }

    pub fn density(&self) -> f64 {
        self.density
    }

    pub fn viscosity(&self) -> f64 {
        self.viscosity
    }

    pub fn clone_with_volume(&self, new_volume: f64) -> Self {
        Self::scale(self, new_volume / self.volume)
    }

    pub fn split_by_volume(self, first_volume: f64) -> (Self, Self) {
        let second_volume = (self.volume - first_volume).max(0.);
        let first_volume = self.volume - second_volume;
        self.split(first_volume / self.volume)
    }

    pub fn split_off_by_volume(&mut self, split_off_volume: f64) -> Self {
        let remaining_volume = (self.volume - split_off_volume).max(0.);
        let (remaining, other) = self.split(remaining_volume / self.volume);
        *self = remaining;
        other
    }
}

impl Mix for FluidChunk {
    fn mix(a: &FluidChunk, b: &FluidChunk) -> FluidChunk {
        let fluid = Fluid::mix(&a.fluid, &b.fluid);
        let volume = a.volume + b.volume;
        let mass = a.mass + b.mass;
        let viscosity = fluid.viscosity();
        FluidChunk {
            fluid,
            volume,
            mass,
            density: mass / volume,
            viscosity,
            // velocity: joint_velocity(a.velocity, a.mass, b.velocity, b.mass),
        }
    }

    fn scale(a: &FluidChunk, s: f64) -> FluidChunk {
        let fluid = Fluid::scale(&a.fluid, s);
        let viscosity = fluid.viscosity();
        FluidChunk {
            fluid,
            volume: s * a.volume,
            mass: s * a.mass,
            density: a.density,
            viscosity,
            // velocity: a.velocity,
        }
    }
}

// impl Add<&FluidChunk> for &FluidChunk {
//     type Output = FluidChunk;

//     fn add(self, other: &FluidChunk) -> FluidChunk {
//         FluidChunk::mix(self, other)
//     }
// }

// impl Add<&FluidChunk> for FluidChunk {
//     type Output = FluidChunk;

//     fn add(self, other: &FluidChunk) -> FluidChunk {
//         FluidChunk::mix(&self, other)
//     }
// }

// impl Add<FluidChunk> for &FluidChunk {
//     type Output = FluidChunk;

//     fn add(self, other: FluidChunk) -> FluidChunk {
//         FluidChunk::mix(self, &other)
//     }
// }

// impl Add<FluidChunk> for FluidChunk {
//     type Output = FluidChunk;

//     fn add(self, other: FluidChunk) -> FluidChunk {
//         FluidChunk::mix(&self, &other)
//     }
// }

// impl Mul<f64> for &FluidChunk {
//     type Output = FluidChunk;

//     fn mul(self, s: f64) -> FluidChunk {
//         FluidChunk::scale(self, s)
//     }
// }

// impl Mul<f64> for FluidChunk {
//     type Output = FluidChunk;

//     fn mul(self, s: f64) -> FluidChunk {
//         FluidChunk::scale(&self, s)
//     }
// }

// impl Add<&Fluid> for &Fluid {
//     type Output = Fluid;

//     fn add(self, other: &Fluid) -> Fluid {
//         Fluid::mix(self, other)
//     }
// }

// impl Add<&Fluid> for Fluid {
//     type Output = Fluid;

//     fn add(self, other: &Fluid) -> Fluid {
//         Fluid::mix(&self, other)
//     }
// }

// impl Add<Fluid> for &Fluid {
//     type Output = Fluid;

//     fn add(self, other: Fluid) -> Fluid {
//         Fluid::mix(self, &other)
//     }
// }

// impl Add<Fluid> for Fluid {
//     type Output = Fluid;

//     fn add(self, other: Fluid) -> Fluid {
//         Fluid::mix(&self, &other)
//     }
// }

// impl Mul<f64> for &Fluid {
//     type Output = Fluid;

//     fn mul(self, s: f64) -> Fluid {
//         Fluid::scale(self, s)
//     }
// }

// impl Mul<f64> for Fluid {
//     type Output = Fluid;

//     fn mul(self, s: f64) -> Fluid {
//         Fluid::scale(&self, s)
//     }
// }
