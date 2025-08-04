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

    pub fn volume(&self) -> f64 {
        disk_area(self.radius) * self.length
    }
}
