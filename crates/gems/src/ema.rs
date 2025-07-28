#[derive(Clone, Debug)]
pub struct RateEma {
    rate: Option<f64>,
    halflife: f64,
}

impl Default for RateEma {
    fn default() -> Self {
        Self::from_halflife(1.0) // default halflife of 1 second
    }
}

impl RateEma {
    pub fn from_halflife(halflife: f64) -> Self {
        assert!(halflife > 0.0);
        Self {
            rate: None,
            halflife,
        }
    }

    pub fn step(&mut self, dt: f64, dx: f64) {
        assert!(dt > 0.0);
        let alpha = 1.0 - (-core::f64::consts::LN_2 * dt / self.halflife).exp();
        let instant_rate = dx / dt;
        self.rate = Some(match self.rate {
            Some(rate) => rate + alpha * (instant_rate - rate),
            None => instant_rate,
        });
    }

    pub fn value(&self) -> f64 {
        self.rate.unwrap_or(0.)
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
