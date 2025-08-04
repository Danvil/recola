use crate::models::{FlowModel, PressureModel};

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
}
