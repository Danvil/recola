//! Various equations related to blood oxygen
//! Reference: https://www.researchgate.net/publication/41454158_Erratum_to_Blood_HbO2_and_HbCO2_dissociation_curves_at_varied_O2_CO2_pH_23-DPG_and_temperature_levels

use num_traits as nt;

/// Plasma fractional water content []
pub const PLASMA_FRACTIONAL_WATER_SPACE: f64 = 0.94;

/// Red blood cell (RBC) fractional water content []
pub const RBC_FRACTIONAL_WATER_SPACE: f64 = 0.65;

/// Hemoglobin molar concentration in red blood cells (RBC) [mM] = [mmol/L]
pub const HB_RBC_MOLAR_CONCENTRATION: f64 = 5.18;

/// Number of oxygen molecules bound per hemoglobin
pub const HB_OXYGEN_COUNT: u32 = 4;

/// Partial pressure of O2 [mmHg] at which hemoglobin is 50% saturated (approx.)
pub const BLOOD_P50_STD: f64 = 26.8;

/// Hill exponent
pub const HILL_EXPONENT: f64 = 2.7;

/// Molar mass of O2 [g/mol]
pub const O2_MOLAR_MASS: f64 = 31.9988;

/// Molar mass of hemoglobin [g/mol]
pub const HB_MOLAR_MASS: f64 = 64458.0;

/// Density of O2 [g/L]
pub const O2_DENSITY: f64 = 1.429;

/// Solubility of O2 in blood under standard condition [mol/L/mmHg]
pub const BLOOD_O2_SOLUBILITY_STD: f64 = 1.46e-6;

/// Relation between hemoglobin saturation and blood oxygen partial pressure after Hill
#[derive(Clone)]
pub struct HemoglobinOxygenSaturationHill<K> {
    p50: K,
    n: K,
}

impl<K> Default for HemoglobinOxygenSaturationHill<K>
where
    K: nt::Num + nt::FromPrimitive,
{
    fn default() -> Self {
        Self {
            p50: from_f64(BLOOD_P50_STD),
            n: from_f64(HILL_EXPONENT),
        }
    }
}

impl<K> HemoglobinOxygenSaturationHill<K>
where
    K: Copy + nt::Num + nt::Pow<K, Output = K> + PartialOrd + std::fmt::Debug,
{
    /// Hemoglobin O₂ saturation [%] as a function of O₂ partial pressure [mmHg]
    pub fn pressure_into_saturation(&self, partial_pressure: K) -> K {
        assert!(K::zero() <= partial_pressure);

        let q = (partial_pressure / self.p50).pow(self.n);
        q / (K::one() + q)
    }

    /// Blood O₂ partial pressure [mmHg] as a function of hemoglobin O₂ saturation [%]
    pub fn saturation_into_pressure(&self, saturation: K) -> K {
        assert!(
            K::zero() <= saturation && saturation < K::one(),
            "{saturation:?}"
        );

        self.p50 * (saturation / (K::one() - saturation)).pow(K::one() / self.n)
    }
}

pub struct BloodGasSolubility<K> {
    /// Coefficients to compute O₂ solubility in blood
    coeffs_o2: [K; 3],

    /// Coefficients to compute CO₂ solubility in blood
    coeffs_co2: [K; 3],

    /// Plasma fraction water content
    w_pl: K,

    /// Nominal body temperature
    nominal_body_temperature: K,
}

impl<K> Default for BloodGasSolubility<K>
where
    K: nt::Num + nt::FromPrimitive,
{
    fn default() -> Self {
        Self {
            coeffs_o2: [from_f64(1.37), from_f64(-1.37e-2), from_f64(5.80e-4)],
            coeffs_co2: [from_f64(3.07), from_f64(-5.70e-2), from_f64(2.00e-4)],
            w_pl: from_f64(PLASMA_FRACTIONAL_WATER_SPACE),
            nominal_body_temperature: from_f64(37.0),
        }
    }
}

impl<K> BloodGasSolubility<K>
where
    K: Copy + nt::Num + nt::FromPrimitive + nt::Pow<K, Output = K>,
{
    /// Solubility of O₂ [M/mmHg] in blood at given temperature [°C]
    pub fn solubility_o2(&self, temperature: K) -> K {
        let a = self.body_temperature_poly(temperature, &self.coeffs_o2);
        let b = from_f64::<K>(1e-6) / self.w_pl;
        a * b
    }

    /// Solubility of CO₂ [M/mmHg] in blood at given temperature [°C]
    pub fn solubility_co2(&self, temperature: K) -> K {
        let a = self.body_temperature_poly(temperature, &self.coeffs_co2);
        let b = from_f64::<K>(1e-5) / self.w_pl;
        a * b
    }

    /// Helper function for temperature [°C] dependency based on nominal body temperature.
    fn body_temperature_poly(&self, temperature: K, &[a0, a1, a2]: &[K; 3]) -> K {
        let d = temperature - self.nominal_body_temperature;
        a0 + (a1 + a2 * d) * d
    }
}

/// Computes blood oxygen "content" as mL oxygen per liter of blood [L/L]. Note that in clinical
/// context the unit [mL/dL] is used. Oxygen is mostly bound to hemoglobin. A small quantity of
/// oxygen, approximatively two orders of magnitude less, is dissolved in the water content of
/// blood.
/// The following quantities are used as input:
/// - hematocrit, i.e. percentage of RBC in blood [%]
/// - solubility of O₂ [mol/L/mmHg]
/// - partial pressure of O₂ [mmHg]
/// - saturation of hemoglobin with O₂ [%].
pub struct BloodOxygenContent<K> {
    hb_rbc: K,
    w_pl: K,
    w_rbc: K,
    hb_o2_cnt: K,

    // O2 g/L / g/mol = mol/L
    o2_mol_per_liter: K,
}

impl<K> Default for BloodOxygenContent<K>
where
    K: Copy + nt::Num + nt::FromPrimitive,
{
    fn default() -> Self {
        Self {
            hb_rbc: from_f64(HB_RBC_MOLAR_CONCENTRATION),
            w_pl: from_f64(PLASMA_FRACTIONAL_WATER_SPACE),
            w_rbc: from_f64(RBC_FRACTIONAL_WATER_SPACE),
            hb_o2_cnt: K::from_u32(HB_OXYGEN_COUNT).unwrap(),
            o2_mol_per_liter: from_f64(O2_DENSITY / O2_MOLAR_MASS),
        }
    }
}

impl<K> BloodOxygenContent<K>
where
    K: Copy + nt::Num + nt::FromPrimitive + nt::Pow<K, Output = K>,
{
    /// Blood O₂ content [mL/L] dissolved in blood
    pub fn dissolved_content(&self, hematocrit: K, o2_solubility: K, o2_partial_pressure: K) -> K {
        // fractional water space of blood
        let w_bl = (K::one() - hematocrit) * self.w_pl + hematocrit * self.w_rbc;

        w_bl * o2_solubility * o2_partial_pressure / self.o2_mol_per_liter
    }

    /// Blood O₂ content [mL/L] bound to hemoglobin
    pub fn hemoglobin_bound(&self, hematocrit: K, hemoglobin_o2_saturation: K) -> K {
        hematocrit * hemoglobin_o2_saturation * self.hb_o2_cnt * self.hb_rbc
            / self.o2_mol_per_liter
            / from_f64::<K>(1000.)
    }

    /// Computes hemoglobin O2 saturation [%] from hemoglobin O2 content [L/L]
    pub fn hb_o2_content_into_so2(&self, hematocrit: K, o2_content: K) -> K {
        o2_content / (hematocrit * self.hb_o2_cnt * self.hb_rbc)
            * self.o2_mol_per_liter
            * from_f64::<K>(1000.)
    }

    /// Total blood O₂ content [mL/L] (dissolved and bound to hemoglobin)
    pub fn total(
        &self,
        hematocrit: K,
        o2_solubility: K,
        o2_partial_pressure: K,
        hemoglobin_o2_saturation: K,
    ) -> K {
        let dissolved = self.dissolved_content(hematocrit, o2_solubility, o2_partial_pressure);
        let bound = self.hemoglobin_bound(hematocrit, hemoglobin_o2_saturation);
        dissolved + bound
    }
}

fn from_f64<K: nt::FromPrimitive>(x: f64) -> K {
    K::from_f64(x).unwrap()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_blood_oxygen_content() {
        let blood_oxygen_content = BloodOxygenContent::<f64>::default();
        let x = blood_oxygen_content.total(0.45, BLOOD_O2_SOLUBILITY_STD, 100., 0.97222);
        assert_eq!(x, 0.20563352075609798);
    }
}
