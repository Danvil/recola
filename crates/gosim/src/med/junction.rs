use crate::{
    newton_root_solver, Cylinder, HoopTubePressureModel, NewtonRootSolverError, TurbulentFlowModel,
};
use std::{collections::HashMap, hash::Hash};

/// Solves pressure at a pipe junction which leads to net zero flow between pipes.
#[derive(Debug, Clone)]
pub struct JunctionPressureSolver<E> {
    /// Connected pipes
    ports: HashMap<E, Port>,

    /// Internal state to avoid making mistakes
    state: JunctionPressureSolverState,

    /// Timestep for which to solve
    dt: f64,

    /// Maximum relative change of volume per substep. This is also the maxium distance relative
    /// to pipe length the fluid can move per substep.
    /// For example, if set to 0.1 the timestep is reduced such that flow distance per substep is
    /// at most 10% of pipe length.
    max_substep_rel: f64,

    /// Maximum number of substeps used to improve solution accuracy.
    max_substeps: usize,
}

#[derive(Clone, Default, PartialEq, Debug)]
enum JunctionPressureSolverState {
    /// Solver is being prepared
    #[default]
    Prepare,

    /// Solver has computed a solution
    Solved {
        equalized_pressure: f64,
        num_stubsteps: usize,
    },

    /// Solver failed
    SolveFailed(JunctionPressureSolverError),
}

/// Accuracy threshold for flow conservation
pub const FLOW_CONSERVATION_THRESHOLD: f64 = 1e-8; // 0.01 mL/s

impl<E> Default for JunctionPressureSolver<E> {
    fn default() -> Self {
        Self {
            ports: HashMap::new(),
            state: JunctionPressureSolverState::Prepare,
            dt: 0.050,
            max_substep_rel: 0.001,
            max_substeps: 1000,
        }
    }
}

impl<E> JunctionPressureSolver<E> {
    /// Resets internal state in preparation for a new solver round
    pub fn reset(&mut self, dt: f64) {
        self.ports.clear();
        self.state = JunctionPressureSolverState::Prepare;
        self.dt = dt;
    }

    pub fn add_port(
        &mut self,
        entity: E,
        cylinder: Cylinder,
        current_volume: f64,
        pressure_baseline: f64,
        pressure_model: HoopTubePressureModel,
        flow_model: TurbulentFlowModel,
    ) where
        E: Eq + Hash,
    {
        assert!(self.state == JunctionPressureSolverState::Prepare);
        self.ports.insert(
            entity,
            Port {
                cylinder,
                initial_volume: current_volume,
                current_volume,
                flow_model,
                pressure_baseline,
                pressure_model,
            },
        );
    }

    pub fn solve(&mut self) -> Result<f64, JunctionPressureSolverError>
    where
        E: core::fmt::Debug,
    {
        // println!("{:?}", self);
        let min_sub_dt = self.dt / self.max_substeps as f64;

        let mut remaining_dt = self.dt;
        let mut i = 0;
        loop {
            i += 1;

            // println!("{i}: {} / {}", self.dt - remaining_dt, self.dt);
            let junction_pressure = match self.solve_step() {
                Ok(p) => p,
                Err(err) => {
                    self.state = JunctionPressureSolverState::SolveFailed(err.clone());
                    return Err(err);
                }
            };

            // rel_vel = velocity / length
            // dt = max_rel_dist / rel_vel
            let mut sub_dt: f64 = remaining_dt;
            for port in self.ports.values() {
                let port_sub_dt = self.max_substep_rel * port.cylinder.length
                    / port.velocity(junction_pressure).abs();
                // println!(
                //     "port_sub_dt: {port_sub_dt}, v={}, L={}",
                //     port.velocity(junction_pressure).abs(),
                //     port.cylinder.length
                // );
                sub_dt = sub_dt.min(port_sub_dt);
            }

            sub_dt = sub_dt.max(min_sub_dt);
            // println!(">> dt={sub_dt}");

            let is_finished = if sub_dt >= remaining_dt {
                sub_dt = remaining_dt;
                remaining_dt = 0.;
                true
            } else {
                remaining_dt -= sub_dt;
                false
            };

            // println!("+{sub_dt}: {junction_pressure}",);
            for (_, port) in self.ports.iter_mut() {
                port.on_substep(junction_pressure, sub_dt);
            }

            if is_finished {
                break;
            }
        }

        // println!("substeps taken: {i}");
        // for (id, port) in self.ports.iter() {
        //     println!("{id:?}: q={}", volume_to_liters(port.total_flow(self.dt)));
        // }

        match self.solve_step() {
            Ok(p) => {
                self.state = JunctionPressureSolverState::Solved {
                    equalized_pressure: p,
                    num_stubsteps: i,
                };
                Ok(p)
            }
            Err(err) => {
                self.state = JunctionPressureSolverState::SolveFailed(err.clone());
                Err(err)
            }
        }
    }

    /// Computes equalized pressure between N ports such that total flow over all ports is zero.
    ///
    /// Find x s.t. sum_i flow(x - pressure(v_i + dt*q_i)) = 0
    /// with: v_i, dt fixed
    /// q_i =
    fn solve_step(&self) -> Result<f64, JunctionPressureSolverError>
    where
        E: core::fmt::Debug,
    {
        if self.ports.is_empty() {
            return Err(JunctionPressureSolverError::NoPorts);
        }

        // Use Poiseuille flow as initial guess
        let p_poiseuille = match self.solve_poiseuille() {
            Some(p) => p,
            None => return Err(JunctionPressureSolverError::NoConductance),
        };
        // println!("poiseuille: {p_poiseuille}");

        // Total flow and derivative by equalized pressure
        let obj_f = |x| -> f64 { self.ports.values().map(|port| port.flow(x)).sum() };
        let dx_f = |x| -> f64 { self.ports.values().map(|port| port.flow_dx(x)).sum() };

        match newton_root_solver(p_poiseuille, FLOW_CONSERVATION_THRESHOLD, 125, obj_f, dx_f) {
            Ok(p) => Ok(p),
            Err(err) => Err(JunctionPressureSolverError::DidNotConverge(err)),
        }
    }

    /// Compute equalized pressure assuming laminar flow model
    fn solve_poiseuille(&self) -> Option<f64> {
        let total_conductance = self
            .ports
            .iter()
            .map(|(_, port)| port.flow_model.poiseuille_conductance())
            .sum::<f64>();

        (total_conductance > 0.).then(|| {
            self.ports
                .iter()
                .map(|(_, port)| port.flow_model.poiseuille_conductance() * port.pressure())
                .sum::<f64>()
                / total_conductance
        })
    }

    /// Gets predicted flow for given port
    pub fn flow(&self, index: E) -> Result<f64, JunctionPressureFlowError>
    where
        E: Eq + Hash,
    {
        match self.ports.get(&index) {
            None => Err(JunctionPressureFlowError::InvalidIndex),
            Some(port) => Ok(port.total_flow(self.dt)),
        }
    }

    pub fn pressure(&self) -> Result<f64, JunctionPressureSolverError> {
        match self.state {
            JunctionPressureSolverState::Solved {
                equalized_pressure, ..
            } => Ok(equalized_pressure),
            JunctionPressureSolverState::SolveFailed(err) => Err(err),
            JunctionPressureSolverState::Prepare => panic!("invalid sequence"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JunctionPressureSolverError {
    NoPorts,
    DidNotConverge(NewtonRootSolverError),
    NoConductance,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JunctionPressureFlowError {
    InvalidIndex,
    SolverError(JunctionPressureSolverError),
}

#[derive(Clone, Debug)]
struct Port {
    /// Shape of the pipe behind the port
    cylinder: Cylinder,

    /// Initial volume applying pressure
    initial_volume: f64,

    /// Current volume increased during sub-stepping
    current_volume: f64,

    /// Base pressure
    pressure_baseline: f64,

    /// Model for additional pressure based on volume
    pressure_model: HoopTubePressureModel,

    /// Model to compute flow based on pressure differential
    flow_model: TurbulentFlowModel,
}

impl Port {
    pub fn flow(&self, outside_pressure: f64) -> f64 {
        self.flow_model.flow(outside_pressure - self.pressure())
    }

    pub fn flow_dx(&self, outside_pressure: f64) -> f64 {
        self.flow_model.flow_dx(outside_pressure - self.pressure())
    }

    pub fn pressure(&self) -> f64 {
        self.pressure_baseline + self.pressure_model.pressure(self.current_volume)
    }

    /// Flow velocity under given target pressure: v = Q / A
    pub fn velocity(&self, outside_pressure: f64) -> f64 {
        self.flow(outside_pressure) / self.cylinder.cross_section_area()
    }

    pub fn on_substep(&mut self, outside_pressure: f64, sub_dt: f64) {
        // println!("  P: {} | {}", self.pressure(), outside_pressure);
        // println!("{:?}", self.flow_model);
        let q = self.flow(outside_pressure);
        // println!("  flow: {}", volume_to_liters(q));
        // println!("  velocity: {}", self.velocity(outside_pressure));
        // println!("  volume 1: {}", volume_to_liters(self.current_volume));
        self.current_volume = (self.current_volume + sub_dt * q).max(0.);
        // println!("  volume 2: {}", volume_to_liters(self.current_volume));
        // println!("  average flow: {}", volume_to_liters(self.flow));
        // println!("  pressure: {}", self.pressure());
    }

    pub fn total_flow(&self, dt: f64) -> f64 {
        (self.current_volume - self.initial_volume) / dt
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ElasticTubeBundle, DENSITY_BLOOD, VISCOSITY_BLOOD};

    #[test]
    fn test_junction_pressure_solver() {
        let mut solver = JunctionPressureSolver::default();

        let mut add_model_f = |i: usize,
                               radius: f64,
                               length: f64,
                               conductance_factor: f64,
                               baseline_pressure: f64| {
            let tubes = ElasticTubeBundle::default()
                .with_radius(radius)
                .with_length(length);
            let qm = TurbulentFlowModel::new(
                tubes.cylinder(),
                DENSITY_BLOOD,
                VISCOSITY_BLOOD,
                conductance_factor,
            );
            let pm = HoopTubePressureModel::new(tubes.clone(), -1_000.);
            let v0 = pm.tubes().nominal_volume();
            solver.add_port(i, tubes.cylinder(), v0, baseline_pressure, pm, qm)
        };

        // add_model_f(0, 0.001, 0.05, 1.0, 1_000.0);
        // add_model_f(1, 0.003, 0.10, 1.0, 2_000.0);
        // add_model_f(2, 0.012, 0.35, 0.1, 3_000.0);
        add_model_f(0, 0.005, 0.100, 1.0, 1_000.0);
        add_model_f(1, 0.005, 0.100, 1.0, 2_000.0);
        add_model_f(2, 0.005, 0.100, 1.0, 3_000.0);

        solver.solve().unwrap();

        approx::assert_relative_eq!(solver.pressure().unwrap(), 2228.301, max_relative = 1e-4);

        let q1 = solver.flow(0).unwrap();
        let q2 = solver.flow(1).unwrap();
        let q3 = solver.flow(2).unwrap();

        approx::assert_relative_eq!(q1, 1.873836926587475e-6, max_relative = 1e-4);
        approx::assert_relative_eq!(q2, 0.432390739038312e-6, max_relative = 1e-4);
        approx::assert_relative_eq!(q3, -2.306236688875294e-6, max_relative = 1e-4);
        approx::assert_abs_diff_eq!(q1 + q2 + q3, 0., epsilon = FLOW_CONSERVATION_THRESHOLD);
    }

    // #[test]
    // fn test_junction_pressure_solver_zero_conductance() {
    //     let mut solver = JunctionPressureSolver::default();

    //     let mut add_model_f = |i: usize,
    //                            radius: f64,
    //                            length: f64,
    //                            conductance_factor: f64,
    //                            baseline_pressure: f64| {
    //         let tubes = ElasticTubeBundle::default()
    //             .with_radius(radius)
    //             .with_length(length);
    //         let qm = TurbulentFlowModel::new(
    //             tubes.cylinder(),
    //             DENSITY_BLOOD,
    //             VISCOSITY_BLOOD,
    //             conductance_factor,
    //         );
    //         let pm = HoopTubePressureModel::new(tubes.clone(), -1_000.);
    //         let v0 = pm.tubes().nominal_volume();
    //         solver.add_port(i, tubes.cylinder(), v0, baseline_pressure, pm, qm)
    //     };

    //     // add_model_f(0, 0.001, 0.05, 1.0, 1_000.0);
    //     // add_model_f(1, 0.003, 0.10, 1.0, 2_000.0);
    //     // add_model_f(2, 0.012, 0.35, 0.1, 3_000.0);
    //     add_model_f(0, 0.005, 0.100, 1.0, 1_000.0);
    //     add_model_f(1, 0.005, 0.100, 1.0, 2_000.0);
    //     add_model_f(2, 0.005, 0.100, 1.0, 3_000.0);

    //     solver.solve().unwrap();

    //     approx::assert_relative_eq!(solver.pressure().unwrap(), 2000., max_relative = 1e-4);

    //     let q1 = solver.flow(0).unwrap();
    //     let q2 = solver.flow(1).unwrap();
    //     let q3 = solver.flow(2).unwrap();

    //     approx::assert_relative_eq!(q1, 0.00031891257811654126, max_relative = 1e-4);
    //     approx::assert_relative_eq!(q2, 0., max_relative = 1e-4);
    //     approx::assert_relative_eq!(q3, -0.00031891257811654126, max_relative = 1e-4);
    //     approx::assert_abs_diff_eq!(q1 + q2 + q3, 0., epsilon = FLOW_CONSERVATION_THRESHOLD);
    // }
}
