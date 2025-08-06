use crate::Mix;
use gems::{
    DENSITY_HEMOGLOBIN, DENSITY_OXYGEN, DENSITY_WATER, MOLAR_MASS_HEMOGLOBIN, VISCOSITY_WATER,
};

#[derive(Clone, Debug, Default)]
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
            + (self.red_rbc + self.blue_rbc) * MOLAR_MASS_HEMOGLOBIN / DENSITY_HEMOGLOBIN
    }

    pub fn mass(&self) -> f64 {
        self.water * DENSITY_WATER
            + self.oxygen * DENSITY_OXYGEN
            + (self.red_rbc + self.blue_rbc) * MOLAR_MASS_HEMOGLOBIN
    }

    pub fn density(&self) -> f64 {
        self.mass() / self.volume()
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
