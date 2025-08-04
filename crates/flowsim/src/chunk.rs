use gems::DENSITY_HEMOGLOBIN;

use gems::MOLAR_MASS_HEMOGLOBIN;

#[derive(Clone, Debug)]
pub struct FluidChunk {
    pub volume: f64,
    pub fluid: Fluid,
}

impl FluidChunk {
    pub fn from_fluid(fluid: Fluid) -> Self {
        Self {
            volume: fluid.volume(),
            fluid,
        }
    }
}

impl Mix for FluidChunk {
    fn mix(a: &FluidChunk, b: &FluidChunk) -> FluidChunk {
        FluidChunk {
            volume: a.volume + b.volume,
            fluid: Fluid::mix(&a.fluid, &b.fluid),
        }
    }

    fn scale(a: &FluidChunk, s: f64) -> FluidChunk {
        FluidChunk {
            volume: s * a.volume,
            fluid: Fluid::scale(&a.fluid, s),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Fluid {
    /// Volume of water
    pub water: f64,

    /// Volume of oxygen
    pub oxygen: f64,

    /// Mol of oxygenated red blood cells
    pub red_rbc: f64,

    /// Mol of de-oxygenated red blood cells
    pub blue_rbc: f64,
}

impl Fluid {
    pub fn volume(&self) -> f64 {
        self.water
            + self.oxygen
            + (self.red_rbc + self.blue_rbc) * MOLAR_MASS_HEMOGLOBIN * DENSITY_HEMOGLOBIN
    }

    pub fn water(volume: f64) -> Self {
        Self {
            water: volume,
            oxygen: 0.,
            red_rbc: 0.,
            blue_rbc: 0.,
        }
    }

    pub fn blood(volume: f64) -> Self {
        Self {
            water: 0.55 * volume,
            oxygen: 0.,
            red_rbc: 0.45 * volume / DENSITY_HEMOGLOBIN / MOLAR_MASS_HEMOGLOBIN, // FIXME
            blue_rbc: 0.,
        }
    }
}

impl Mix for Fluid {
    fn mix(a: &Fluid, b: &Fluid) -> Fluid {
        Fluid {
            water: a.water + b.water,
            oxygen: a.oxygen + b.oxygen,
            red_rbc: a.red_rbc + b.red_rbc,
            blue_rbc: a.blue_rbc + b.blue_rbc,
        }
    }

    fn scale(a: &Fluid, s: f64) -> Fluid {
        Fluid {
            water: s * a.water,
            oxygen: s * a.oxygen,
            red_rbc: s * a.red_rbc,
            blue_rbc: s * a.blue_rbc,
        }
    }
}

pub trait Mix: Sized {
    fn mix(a: &Self, b: &Self) -> Self;

    fn scale(a: &Self, s: f64) -> Self;

    fn split(&self, q: f64) -> (Self, Self) {
        (Self::scale(self, q), Self::scale(self, 1. - q))
    }

    fn mix_many<'a, I>(mut iter: I) -> Option<Self>
    where
        I: Iterator<Item = &'a Self>,
        Self: Clone + 'a,
    {
        let mut a = iter.next()?.clone();
        for x in iter {
            a = Self::mix(&a, x);
        }
        Some(a)
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
