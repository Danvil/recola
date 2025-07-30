#[derive(Clone, Debug)]
pub struct Ema {
    value: Option<f64>,
    halflife: f64,
}

impl Default for Ema {
    fn default() -> Self {
        Self::from_halflife(1.0) // default halflife of 1 second
    }
}

impl Ema {
    pub fn from_halflife(halflife: f64) -> Self {
        assert!(halflife > 0.0);
        Self {
            value: None,
            halflife,
        }
    }

    pub fn step(&mut self, dt: f64, dx: f64) {
        assert!(dt > 0.0);
        let alpha = 1.0 - (-core::f64::consts::LN_2 * dt / self.halflife).exp();
        self.value = Some(match self.value {
            Some(value) => value + alpha * (dx - value),
            None => dx,
        });
    }

    pub fn value(&self) -> f64 {
        self.value.unwrap_or(0.)
    }
}

#[derive(Clone, Debug)]
pub struct RateEma(Ema);

impl Default for RateEma {
    fn default() -> Self {
        Self::from_halflife(1.0) // default halflife of 1 second
    }
}

impl RateEma {
    pub fn from_halflife(halflife: f64) -> Self {
        Self(Ema::from_halflife(halflife))
    }

    pub fn step(&mut self, dt: f64, dx: f64) {
        self.0.step(dt, dx / dt)
    }

    pub fn value(&self) -> f64 {
        self.0.value()
    }
}

#[derive(Default, Clone)]
pub struct BeatEma {
    rate: RateEma,
}

impl BeatEma {
    pub fn from_halflife(halflife: f64) -> Self {
        Self {
            rate: RateEma::from_halflife(halflife),
        }
    }

    pub fn step(&mut self, dt: f64, beat: bool) {
        assert!(dt > 0.);

        self.rate.step(dt, if beat { 1. } else { 0. })
    }

    pub fn value(&self) -> f64 {
        self.rate.value()
    }
}
