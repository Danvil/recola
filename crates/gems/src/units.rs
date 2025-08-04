/// mmHg (millimiter Mercury) to Pascal
pub const MM_HG_TO_PA: f64 = 133.322;

pub fn volume_from_liters(liters: f64) -> f64 {
    liters * 1e-3
}

pub fn volume_from_milli_liters(milli_liters: f64) -> f64 {
    milli_liters * 1e-6
}

pub fn volume_to_liters(volume: f64) -> f64 {
    volume * 1e3
}

pub fn volume_to_milli_liters(volume: f64) -> f64 {
    volume * 1e6
}

pub fn pressure_from_mm_hg(mmhg: f64) -> f64 {
    mmhg * MM_HG_TO_PA
}

pub fn pressure_to_mm_hg(pressure: f64) -> f64 {
    pressure / MM_HG_TO_PA
}

pub fn pressure_from_atm(atm: f64) -> f64 {
    atm * 101325.
}
