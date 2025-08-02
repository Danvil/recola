use crate::{newton_root_solver, ElasticTubePressureModel, TurbulentFlowModel};
use std::{collections::HashMap, hash::Hash};

/// Solves pressure at a pipe junction which leads to net zero flow between pipes.
#[derive(Clone, Default)]
pub struct JunctionPressureSolver<E> {
    /// Connected pipes
    ports: HashMap<E, Port>,

    /// Internal state to avoid making mistakes
    state: JunctionPressureSolverState,

    /// If set pressure is equalized assuming predicited flow for a certain timestep
    prediction: Option<f64>,
}

#[derive(Clone, Default, PartialEq)]
enum JunctionPressureSolverState {
    /// Solver is being prepared
    #[default]
    Prepare,

    /// Solver has computed a solution
    Solved(f64),

    /// Solver failed
    SolveFailed(JunctionPressureSolverError),
}

/// Accuracy threshold for flow conservation
pub const FLOW_CONSERVATION_THRESHOLD: f64 = 1e-8; // 0.01 mL/s

impl<E> JunctionPressureSolver<E> {
    pub fn new() -> Self {
        Self {
            ports: HashMap::new(),
            prediction: None,
            state: JunctionPressureSolverState::Prepare,
        }
    }

    /// Resets internal state in preparation for a new solver round
    pub fn reset(&mut self) {
        self.ports.clear();
        self.state = JunctionPressureSolverState::Prepare;
    }

    pub fn add_port(
        &mut self,
        entity: E,
        flow_model: TurbulentFlowModel,
        pressure_baseline: f64,
        pressure_model: ElasticTubePressureModel,
        current_volume: f64,
    ) where
        E: Eq + Hash,
    {
        assert!(self.state == JunctionPressureSolverState::Prepare);
        let current_pressure = pressure_baseline + pressure_model.pressure(current_volume);
        self.ports.insert(
            entity,
            Port {
                flow_model,
                pressure_baseline,
                pressure_model,
                current_volume,
                current_pressure,
            },
        );
    }

    /// Compute equalized pressure assuming laminar flow and without flow prediction
    fn solve_poiseuille(&mut self) -> f64 {
        self.ports
            .iter()
            .map(|(_, port)| port.flow_model.poiseuille_conductance() * port.current_pressure)
            .sum::<f64>()
            / self
                .ports
                .iter()
                .map(|(_, port)| port.flow_model.poiseuille_conductance())
                .sum::<f64>()
    }

    /// Computes equalized pressure between N ports such that total flow over all ports is zero.
    ///
    /// Find x s.t. sum_i flow(x - pressure(v_i + dt*q_i)) = 0
    /// with: v_i, dt fixed
    /// q_i =
    pub fn solve(&mut self) -> Result<(), JunctionPressureSolverError>
    where
        E: core::fmt::Debug,
    {
        if self.ports.is_empty() {
            self.state =
                JunctionPressureSolverState::SolveFailed(JunctionPressureSolverError::NoPorts);
            return Err(JunctionPressureSolverError::NoPorts);
        }

        // println!("{:?}", self.ports);

        // Use Poiseuille flow as initial guess
        let p_poiseuille = self.solve_poiseuille();
        // println!("{:?}", p_poiseuille);

        // Total flow and derivative by equalized pressure
        let obj_f = |x: f64| -> f64 {
            self.ports
                .iter()
                .map(|(_, port)| port.flow(x, self.prediction))
                .sum::<f64>()
        };
        let dx_f = |x: f64| -> f64 {
            self.ports
                .iter()
                .map(|(_, port)| port.flow_dx(x, self.prediction))
                .sum::<f64>()
        };

        match newton_root_solver(p_poiseuille, FLOW_CONSERVATION_THRESHOLD, 25, obj_f, dx_f) {
            Ok(p) => {
                self.state = JunctionPressureSolverState::Solved(p);
                Ok(())
            }
            Err(_) => {
                self.state = JunctionPressureSolverState::SolveFailed(
                    JunctionPressureSolverError::DidNotConverge,
                );
                Err(JunctionPressureSolverError::DidNotConverge)
            }
        }
    }

    /// Gets predicted flow for given port
    pub fn flow(&self, index: E) -> Option<f64>
    where
        E: Eq + Hash,
    {
        let peq = self.pressure()?;

        self.ports
            .get(&index)
            .map(|port| port.flow(peq, self.prediction))
    }

    pub fn pressure(&self) -> Option<f64> {
        match self.state {
            JunctionPressureSolverState::Solved(peq) => Some(peq),
            JunctionPressureSolverState::SolveFailed(_) => None,
            JunctionPressureSolverState::Prepare => panic!("invalid sequence"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum JunctionPressureSolverError {
    NoPorts,
    DidNotConverge,
}

#[derive(Clone, Debug)]
struct Port {
    flow_model: TurbulentFlowModel,

    pressure_baseline: f64,
    pressure_model: ElasticTubePressureModel,

    current_volume: f64,
    current_pressure: f64,
}

impl Port {
    pub fn flow(&self, outside_pressure: f64, prediction: Option<f64>) -> f64 {
        match prediction {
            Some(dt) => self.predicted_flow(outside_pressure, dt),
            None => self.estimated_flow(outside_pressure),
        }
    }

    pub fn flow_dx(&self, outside_pressure: f64, prediction: Option<f64>) -> f64 {
        match prediction {
            Some(dt) => self.predicted_flow_dx(outside_pressure, dt),
            None => self.estimated_flow_dx(outside_pressure),
        }
    }

    pub fn estimated_flow(&self, outside_pressure: f64) -> f64 {
        let current_pressure = self.current_pressure();
        self.flow_model.flow(outside_pressure - current_pressure)
    }

    pub fn estimated_flow_dx(&self, outside_pressure: f64) -> f64 {
        let current_pressure = self.current_pressure();
        self.flow_model.flow_dx(outside_pressure - current_pressure)
    }

    pub fn predicted_flow(&self, outside_pressure: f64, dt: f64) -> f64 {
        // println!("eval: ");
        let predicted_pressure = self.predicted_pressure(outside_pressure, dt);

        // Compute flow based on predicted pressure
        let q = self.flow_model.flow(outside_pressure - predicted_pressure);

        let q_corrected = ((self.current_volume + dt * q).max(0.) - self.current_volume) / dt;
        // println!("q: {q}, corr: {q_corrected}");

        q_corrected
    }

    fn current_pressure(&self) -> f64 {
        self.pressure_baseline + self.pressure_model.pressure(self.current_volume)
    }

    fn predicted_pressure(&self, outside_pressure: f64, dt: f64) -> f64 {
        // Compute flow based on current pressure differential
        let predicted_flow = self
            .flow_model
            .flow(outside_pressure - self.current_pressure);

        // Predicted volume change and corresponding change in pressure
        let predicted_volume = (self.current_volume + dt * predicted_flow).max(0.);
        let predicted_pressure =
            self.pressure_baseline + self.pressure_model.pressure(predicted_volume);

        // println!("Q': {predicted_flow}, V': {predicted_volume}, P': {predicted_pressure}",);

        predicted_pressure
    }

    pub fn predicted_flow_dx(&self, outside_pressure: f64, dt: f64) -> f64 {
        let delta = 1e-3; // Pa
        let q1 = self.predicted_flow(outside_pressure, dt);
        let q2 = self.predicted_flow(outside_pressure + delta, dt);
        (q2 - q1) / delta

        // let delta = 1e-3;

        // let p1 = self.predicted_pressure(outside_pressure, dt);
        // let p2 = self.predicted_pressure(outside_pressure + delta, dt);
        // let dp_dx = (p2 - p1) / delta;

        // self.flow_model.flow_dx(outside_pressure - p1) * (1. - dp_dx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ElasticTubeBundle, DENSITY_BLOOD, VISCOSITY_BLOOD};

    #[test]
    fn test_junction_pressure_solver() {
        let mut solver = JunctionPressureSolver::new();

        let mut add_model_f = |i: usize,
                               radius: f64,
                               length: f64,
                               conductance_factor: f64,
                               baseline_pressure: f64| {
            let tubes = ElasticTubeBundle::default()
                .with_radius(radius)
                .with_length(length);
            let qm = TurbulentFlowModel::new(
                tubes.radius,
                tubes.length,
                DENSITY_BLOOD,
                VISCOSITY_BLOOD,
                conductance_factor,
            );
            let pm = ElasticTubePressureModel::new(tubes, -1_000.);
            let v0 = pm.tubes().nominal_volume();
            solver.add_port(i, qm, baseline_pressure, pm, v0)
        };

        // add_model_f(0, 0.001, 0.05, 1.0, 1_000.0);
        // add_model_f(1, 0.003, 0.10, 1.0, 2_000.0);
        // add_model_f(2, 0.012, 0.35, 0.1, 3_000.0);
        add_model_f(0, 0.005, 0.10, 1.0, 1_000.0);
        add_model_f(1, 0.005, 0.10, 1.0, 2_000.0);
        add_model_f(2, 0.005, 0.10, 1.0, 3_000.0);

        solver.solve().unwrap();

        approx::assert_relative_eq!(solver.pressure().unwrap(), 2000., max_relative = 1e-4);

        let q1 = solver.flow(0).unwrap();
        let q2 = solver.flow(1).unwrap();
        let q3 = solver.flow(2).unwrap();

        approx::assert_relative_eq!(q1, 0.00031891257811654126, max_relative = 1e-4);
        approx::assert_relative_eq!(q2, 0., max_relative = 1e-4);
        approx::assert_relative_eq!(q3, -0.00031891257811654126, max_relative = 1e-4);
        approx::assert_abs_diff_eq!(q1 + q2 + q3, 0., epsilon = FLOW_CONSERVATION_THRESHOLD);
    }
}
