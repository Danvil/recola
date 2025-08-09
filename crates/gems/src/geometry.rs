pub fn disk_area(r: f64) -> f64 {
    r * r * core::f64::consts::PI
}

pub fn disk_circumfence(r: f64) -> f64 {
    2. * r * core::f64::consts::PI
}

pub fn cylinder_volume(radius: f64, length: f64) -> f64 {
    disk_area(radius) * length
}

pub fn cylinder_radius(volume: f64, length: f64) -> f64 {
    (volume / (core::f64::consts::PI * length)).sqrt()
}

pub fn cylinder_area(radius: f64, length: f64) -> f64 {
    disk_circumfence(radius) * length
}

#[derive(Clone, Debug)]
pub struct Cylinder {
    pub radius: f64,
    pub length: f64,
}

impl Cylinder {
    pub fn is_non_zero(&self) -> bool {
        self.radius > 0. && self.length > 0.
    }

    pub fn cross_section_area(&self) -> f64 {
        disk_area(self.radius)
    }

    pub fn surface_area(&self) -> f64 {
        disk_circumfence(self.radius) * self.length
    }
}

/// A volumetric shape
pub trait VolumeModel {
    fn nominal_volume(&self) -> f64;
}

/// A model relating area and volume of a volumetric shape
pub trait AreaVolumeModel {
    fn volume(&self, area: f64) -> f64;
    fn area(&self, volume: f64) -> f64;
}

impl VolumeModel for Cylinder {
    fn nominal_volume(&self) -> f64 {
        self.cross_section_area() * self.length
    }
}

impl AreaVolumeModel for Cylinder {
    fn area(&self, volume: f64) -> f64 {
        volume / self.length
    }

    fn volume(&self, area: f64) -> f64 {
        area * self.length
    }
}

// /// A cylinder with fixed area at the ends and thinning in the middle. Volume will not fall
// /// below a minimum
// pub struct CapitalCylinder {
//     pub cylinder: Cylinder,
//     pub min_rel_vol: f64,
// }

// impl VolumeModel for CapitalCylinder {
//     fn nominal_volume(&self) -> f64 {
//         self.cylinder.nominal_volume()
//     }
// }

// impl AreaVolumeModel for CapitalCylinder {
//     fn area(&self, volume: f64) -> f64 {
//         let volume_0 = self.nominal_volume();
//         let veff = (volume - volume_0 * self.min_rel_vol).max(0.) / (1. - self.min_rel_vol);
//         veff / self.cylinder.length
//     }

//     fn volume(&self, area: f64) -> f64 {
//         let area_0 = self.cylinder.cross_section_area();
//         self.cylinder.length * ((1. - self.min_rel_vol) * area + self.min_rel_vol * area_0)
//     }
// }

#[cfg(test)]
mod test {
    use crate::{
        geometry::{AreaVolumeModel, VolumeModel},
        Cylinder,
    };
    use std::f64::consts::PI;

    #[test]
    fn test_cylinder() {
        let c = Cylinder {
            radius: 0.01,
            length: 0.5,
        };
        const VOL_0: f64 = 0.00005 * PI;
        const AREA_0: f64 = 0.0001 * PI;
        approx::assert_abs_diff_eq!(c.cross_section_area(), AREA_0);
        approx::assert_abs_diff_eq!(c.surface_area(), 0.01 * PI);
        approx::assert_abs_diff_eq!(c.nominal_volume(), VOL_0);
        approx::assert_abs_diff_eq!(c.area(0.), 0.);
        approx::assert_abs_diff_eq!(c.volume(0.), 0.);
        approx::assert_abs_diff_eq!(c.area(VOL_0), AREA_0);
        approx::assert_abs_diff_eq!(c.volume(AREA_0), VOL_0);
        approx::assert_abs_diff_eq!(c.area(2. * VOL_0), 2. * AREA_0);
        approx::assert_abs_diff_eq!(c.volume(2. * AREA_0), 2. * VOL_0);
    }

    // #[test]
    // fn test_capital_cylinder() {
    //     let c = CapitalCylinder {
    //         cylinder: Cylinder {
    //             radius: 0.01,
    //             length: 0.5,
    //         },
    //         min_rel_vol: 0.1,
    //     };
    //     const VOL_0: f64 = 0.00005 * PI;
    //     const AREA_0: f64 = 0.0001 * PI;
    //     const VOL_MIN: f64 = 0.1 * VOL_0;
    //     approx::assert_abs_diff_eq!(c.nominal_volume(), VOL_0);
    //     approx::assert_abs_diff_eq!(c.area(0.), 0.);
    //     approx::assert_abs_diff_eq!(c.volume(0.), VOL_MIN);
    //     approx::assert_abs_diff_eq!(c.area(VOL_0), AREA_0);
    //     approx::assert_abs_diff_eq!(c.volume(AREA_0), VOL_0);
    //     approx::assert_abs_diff_eq!(c.area(2. * VOL_0 + VOL_MIN), 2. * AREA_0);
    //     approx::assert_abs_diff_eq!(c.volume(2. * AREA_0), 2. * VOL_0);
    // }
}
