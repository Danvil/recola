use num_traits as nt;
use std::ops::Neg;

/// Reference: https://pmc.ncbi.nlm.nih.gov/articles/PMC4699875/pdf/nihms718079.pdf
/// Note: unit M is moles per liters
pub struct HemoglobinO2CO2<K> {
    fractional_water_space_of_plasma: K,
    k_prime: [K; 4],
    k_sec: [K; 6],
}

impl<K> HemoglobinO2CO2<K>
where
    K: Copy + nt::Num + nt::FromPrimitive + nt::Pow<K, Output = K> + Neg<Output = K>,
{
    pub fn new() -> Self {
        HemoglobinO2CO2 {
            fractional_water_space_of_plasma: K::from_f64(0.94).unwrap(),
            k_prime: [cast(1.4e-3), cast(23.65), cast(14.7), cast(2.04e5)],
            k_sec: [
                cast(5.5e-4),
                cast(1e-6),
                cast(1e-6),
                cast(0.0),
                cast(2.64e-8),
                cast(1.56e-8),
            ],
        }
    }

    /// Saturation of substance [%] of concentration [M] based on equilibrium constant [1/M]
    fn saturation(&self, equilibrium_constant: K, concentration: K) -> K {
        let a = equilibrium_constant * concentration;
        a / (K::one() + a)
    }

    /// Equilibrium constant for HbO₂ [1/M] given CO₂ concentration and pH of plasma []
    fn equilibrium_constant_hbo2(&self, concentration_co2: K, ph_plasma: K) -> K {
        let [h1, h2, h3, h4] = self.equilibrium_constant_phi(ph_plasma);
        let [_, k2, k3, k4] = self.k_prime;
        (k4 * (k3 * concentration_co2 * h2 + h4)) / (k2 * concentration_co2 * h1 + h3)
    }

    /// Equilibrium constant for HbCO₂ [1/M] given O₂ concentration and pH of plasma []
    fn equilibrium_constant_hbco2(&self, concentration_o2: K, ph_plasma: K) -> K {
        let [h1, h2, h3, h4] = self.equilibrium_constant_phi(ph_plasma);
        let [_, k2, k3, k4] = self.k_prime;
        (k2 * h1 + k3 * k4 * concentration_o2 * h2) / (h3 + k4 * concentration_o2 * h4)
    }

    /// Constants used by equilibrium_constant_hbo2
    fn equilibrium_constant_phi(&self, ph_plasma: K) -> [K; 4] {
        let ph = self.ph_rbs(ph_plasma);
        let [_, k2, k3, _, k5, k6] = self.k_sec;
        let h = cast::<K>(10.0).pow(-ph);
        [
            K::one() + k2 / h,
            K::one() + k3 / h,
            K::one() + k5,
            K::one() + k6,
        ]
    }

    /// pH of RBCs [] given pH of plasma []
    fn ph_rbs(&self, ph_plasma: K) -> K {
        cast::<K>(0.796) * ph_plasma + cast::<K>(1.357)
    }

    /// Solubility of O₂ [M/mmHg] in blood at given temperature [°C]
    fn solubility_o2(&self, temperature: K) -> K {
        let a = self.body_temperature_poly(
            temperature,
            [
                K::from_f64(1.37).unwrap(),
                K::from_f64(-1.37e-2).unwrap(),
                K::from_f64(5.80e-4).unwrap(),
            ],
        );
        let b = K::from_f64(1e-6).unwrap() / self.fractional_water_space_of_plasma;
        a * b
    }

    /// Solubility of CO₂ [M/mmHg] in blood at given temperature [°C]
    fn solubility_co2(&self, temperature: K) -> K {
        let a = self.body_temperature_poly(
            temperature,
            [
                K::from_f64(3.07).unwrap(),
                K::from_f64(-5.70e-2).unwrap(),
                K::from_f64(2.00e-4).unwrap(),
            ],
        );
        let b = K::from_f64(1e-5).unwrap() / self.fractional_water_space_of_plasma;
        a * b
    }

    /// Concentration [M] of substance with solubility [M/mmHg] at partial_pressure [mmHg]
    fn partial_pressure_to_concentration(&self, solubility: K, partial_pressure: K) -> K {
        solubility * partial_pressure
    }

    /// Helper function to compute quantity based on temperature [°C] difference from nominal body
    /// temperature.
    fn body_temperature_poly(&self, temperature: K, [a0, a1, a2]: [K; 3]) -> K {
        let d = temperature - K::from_f64(37.0).unwrap();
        a0 + (a1 + a2 * d) * d
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rust_decimal::prelude::*;

    // #[test]
    // fn test_saturation() {
    //     let model = HemoglobinO2CO2::new();
    //     let x = model.saturation(dec!(0.1), dec!(0.1));
    // }

    #[test]
    fn test_body_temperature_poly() {
        let model = HemoglobinO2CO2::new();

        let y = model.body_temperature_poly(dec!(37), [dec!(1), dec!(2), dec!(3)]);
        assert_eq!(y, dec!(1));

        let y = model.body_temperature_poly(dec!(39), [dec!(1), dec!(2), dec!(3)]);
        assert_eq!(y, dec!(17));
    }
}

fn cast<K: nt::FromPrimitive>(x: f64) -> K {
    K::from_f64(x).unwrap()
}
