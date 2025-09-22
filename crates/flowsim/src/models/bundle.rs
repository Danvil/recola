use crate::models::{FlowModel, PressureModel};
use gems::{AreaVolumeModel, VolumeModel};
use magi_gems::NewtonRootSolverError;

/// Bundle of models in parallel
#[derive(Clone, Debug)]
pub struct Bundle<M> {
    pub model: M,
    pub count: f64,
}

impl<M: FlowModel> FlowModel for Bundle<M> {
    fn flow(&self, pressure_difference: f64) -> f64 {
        self.model.flow(pressure_difference) * self.count
    }

    fn flow_dx(&self, pressure_difference: f64) -> f64 {
        self.model.flow_dx(pressure_difference) * self.count
    }

    fn poiseuille_conductance(&self) -> f64 {
        self.model.poiseuille_conductance() * self.count
    }
}

impl<M: PressureModel> PressureModel for Bundle<M> {
    fn pressure(&self, volume: f64) -> f64 {
        self.model.pressure(volume / self.count)
    }

    fn pressure_dx(&self, volume: f64) -> f64 {
        self.model.pressure_dx(volume / self.count) / self.count
    }

    fn volume(&self, pressure: f64, guess: f64) -> Result<f64, NewtonRootSolverError> {
        self.model
            .volume(pressure, guess / self.count)
            .map(|v| v * self.count)
    }
}

impl<M: VolumeModel> VolumeModel for Bundle<M> {
    fn nominal_volume(&self) -> f64 {
        self.model.nominal_volume() * self.count
    }
}

impl<M: AreaVolumeModel> AreaVolumeModel for Bundle<M> {
    fn area(&self, volume: f64) -> f64 {
        self.model.area(volume / self.count) * self.count
    }

    fn volume(&self, area: f64) -> f64 {
        self.model.volume(area / self.count) * self.count
    }
}
