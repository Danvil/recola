/// How long it takes for the heart rate to achieve the target.
const CARDIAC_BPM_ADOPTION_HALFTIME: f64 = 1.50;

#[derive(Clone)]
pub struct CardiacCycle {
    target_rate: f64,
    current_rate: f64,
    stage: CardiacCycleStage,
    stage_time: f64,
    stage_percent: f64,
    beat: bool,
}

impl Default for CardiacCycle {
    fn default() -> Self {
        Self {
            target_rate: 60.,
            current_rate: 60.,
            stage: CardiacCycleStage::DiastolePhase1,
            stage_time: 0.,
            stage_percent: 0.,
            beat: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CardiacCycleStage {
    /// No contraction (diastole without arterial systole)
    #[default]
    DiastolePhase1,

    /// Atrium contracts
    ArterialSystole,

    /// Ventricle contracts
    Systole,
}

impl CardiacCycleStage {
    pub fn next(&self) -> Self {
        match self {
            CardiacCycleStage::DiastolePhase1 => CardiacCycleStage::ArterialSystole,
            CardiacCycleStage::ArterialSystole => CardiacCycleStage::Systole,
            CardiacCycleStage::Systole => CardiacCycleStage::DiastolePhase1,
        }
    }
}

impl CardiacCycle {
    pub fn set_target_rate(&mut self, target_rate: f64) {
        self.target_rate = target_rate;
    }

    pub fn current_rate(&self) -> f64 {
        self.current_rate
    }

    pub fn set_target_bpm(&mut self, target_bpm: f64) {
        self.target_rate = target_bpm / 60.;
    }

    pub fn current_bpm(&self) -> f64 {
        self.current_rate * 60.
    }

    pub fn step(&mut self, dt: f64) {
        let alpha = 1.0 - (-core::f64::consts::LN_2 * dt / CARDIAC_BPM_ADOPTION_HALFTIME).exp();
        self.current_rate += alpha * (self.target_rate - self.current_rate);

        self.stage_time += dt;
        let target = match self.stage {
            CardiacCycleStage::DiastolePhase1 => 0.40,
            CardiacCycleStage::ArterialSystole => 0.15,
            CardiacCycleStage::Systole => systole_duration(self.current_rate),
        };

        self.stage_percent = self.stage_time / target;

        self.beat = false;
        if self.stage_time >= target {
            self.stage_time = 0.;
            self.stage_percent = 0.;
            self.stage = self.stage.next();
            if self.stage == CardiacCycleStage::Systole {
                self.beat = true;
            }
        }
    }

    pub fn stage(&self) -> (CardiacCycleStage, f64) {
        (self.stage, self.stage_percent)
    }

    pub fn beat(&self) -> bool {
        self.beat
    }
}

/// Reference: https://pmc.ncbi.nlm.nih.gov/articles/PMC7328879/
pub fn systole_duration(heart_rate: f64) -> f64 {
    0.383451 + (1. / heart_rate).powf(0.3558)
}

/// Guesstimate
pub fn arterial_systole_duration(heart_rate: f64) -> f64 {
    0.1 / heart_rate
}

/// Guesstimate
pub fn diastole_phase_1_duration(heart_rate: f64) -> f64 {
    0.9 / heart_rate - systole_duration(heart_rate)
}
