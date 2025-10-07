use crate::Mix;
use gems::{
    DENSITY_BLOOD, DENSITY_HEMOGLOBIN, DENSITY_OXYGEN, DENSITY_WATER, MOLAR_MASS_HEMOGLOBIN,
    VISCOSITY_BLOOD, VISCOSITY_WATER,
};

#[derive(Clone, Debug, Default)]
pub struct FluidComposition {
    /// Volume of water
    pub water: f64,

    /// Volume of oxygen
    pub oxygen: f64,

    /// Mol of oxygenated red blood cells
    pub red_rbc: f64,

    /// Mol of de-oxygenated red blood cells
    pub blue_rbc: f64,
}

impl FluidComposition {
    pub fn mass_and_volume(&self) -> FluidMassVolume {
        let volume = self.volume();
        let mass = self.mass();
        FluidMassVolume { volume, mass }
    }

    pub fn density_and_viscosity(&self) -> FluidDensityViscosity {
        let volume = self.volume();
        let mass = self.mass();
        let viscosity = self.viscosity();
        FluidDensityViscosity {
            density: safe_density(mass, volume),
            viscosity,
        }
    }

    pub fn volume(&self) -> f64 {
        self.water
            + self.oxygen
            + (self.red_rbc + self.blue_rbc) * MOLAR_MASS_HEMOGLOBIN / DENSITY_HEMOGLOBIN
    }

    pub fn mass(&self) -> f64 {
        self.water * DENSITY_WATER
            + self.oxygen * DENSITY_OXYGEN
            + (self.red_rbc + self.blue_rbc) * MOLAR_MASS_HEMOGLOBIN
    }

    pub fn density(&self) -> f64 {
        safe_density(self.mass(), self.volume())
    }

    pub fn viscosity(&self) -> f64 {
        VISCOSITY_WATER // FIXME
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
            red_rbc: 0.45 * volume * DENSITY_HEMOGLOBIN / MOLAR_MASS_HEMOGLOBIN, // FIXME
            blue_rbc: 0.,
        }
    }
}

fn safe_density(mass: f64, volume: f64) -> f64 {
    let volume = volume;
    if volume == 0. { 1e3 } else { mass / volume }
}

impl Mix for FluidComposition {
    fn mix(a: &FluidComposition, b: &FluidComposition) -> FluidComposition {
        FluidComposition {
            water: a.water + b.water,
            oxygen: a.oxygen + b.oxygen,
            red_rbc: a.red_rbc + b.red_rbc,
            blue_rbc: a.blue_rbc + b.blue_rbc,
        }
    }

    fn scale(a: &FluidComposition, s: f64) -> FluidComposition {
        FluidComposition {
            water: s * a.water,
            oxygen: s * a.oxygen,
            red_rbc: s * a.red_rbc,
            blue_rbc: s * a.blue_rbc,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FluidMassVolume {
    pub volume: f64,
    pub mass: f64,
}

impl Mix for FluidMassVolume {
    fn mix(a: &FluidMassVolume, b: &FluidMassVolume) -> FluidMassVolume {
        let volume = a.volume + b.volume;
        let mass = a.mass + b.mass;
        FluidMassVolume { volume, mass }
    }

    fn scale(a: &FluidMassVolume, s: f64) -> FluidMassVolume {
        FluidMassVolume {
            volume: s * a.volume,
            mass: s * a.mass,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FluidDensityViscosity {
    pub density: f64,
    pub viscosity: f64,
}

impl FluidDensityViscosity {
    pub fn blood() -> Self {
        Self {
            density: DENSITY_BLOOD,
            viscosity: VISCOSITY_BLOOD,
        }
    }

    pub fn water() -> Self {
        Self {
            density: DENSITY_WATER,
            viscosity: VISCOSITY_WATER,
        }
    }
}
