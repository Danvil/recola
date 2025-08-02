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
