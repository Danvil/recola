use crate::{
    models::{Bundle, HoopTubePressureModel, TurbulentFlowModel},
    PipeVessel,
};
use gems::Cylinder;

#[derive(Clone, Debug)]
pub struct Pipe {
    pub shape: Cylinder,
    pub vessel: PipeVessel,
    pub external_pressure: [f64; 2],
    pub elasticity_pressure_model: Bundle<HoopTubePressureModel>,
    pub flow_model: Bundle<TurbulentFlowModel>,
}
