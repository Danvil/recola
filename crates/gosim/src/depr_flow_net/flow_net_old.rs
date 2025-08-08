use crate::{
    volume_from_milli_liters, volume_to_milli_liters, Arg, ElasticTubeBundle, EntityBuilder,
    FlecsQueryRelationHelpers, HoopTubePressureModel, JunctionPressureFlowError,
    JunctionPressureSolver, JunctionPressureSolverError, This, Time, TimeModule,
    TurbulentFlowModel, DENSITY_BLOOD, FLOW_CONSERVATION_THRESHOLD, VISCOSITY_BLOOD,
};
use flecs_ecs::prelude::*;
use gems::{Ema, Lerp};
use std::{
    collections::{HashMap, VecDeque},
    marker::PhantomData,
};

/// Flow nets pump fluid through compliant pipes using a basic pressure model.
#[derive(Component)]
pub struct FlowNetModule;

#[derive(Component)]
pub struct FlowNetConfig {
    /// Fluid flow per second and unit of pressure difference
    pub flow_factor: f64,
}

/// Maximum relative volume a pump can remove from the intake per tick
pub const PUMP_MAX_REL_VOL_PER_TICK: f64 = 0.5;

/// Stats for an elastic fluid pipe
#[derive(Component, Clone, Debug)]
pub struct PipeGeometry {
    pub tubes: ElasticTubeBundle,

    /// Minimal pressure for tube law used when radius is smaller than nominal
    pub collapse_pressure: f64,

    /// Factor applied to conductance computed after Poiseuille. Poiseuille assumes laminar
    /// flow and this can compensate for this.
    pub conductance_factor: f64,
}

impl PipeGeometry {
    pub fn pressure_model(&self) -> HoopTubePressureModel {
        HoopTubePressureModel::new(self.tubes.clone(), self.collapse_pressure)
    }

    pub fn flow_model(&self, density: f64, viscosity: f64) -> TurbulentFlowModel {
        TurbulentFlowModel::new(
            self.tubes.cylinder(),
            density,
            viscosity,
            self.conductance_factor,
        )
    }
}

// /// Internal state used for computation of liquid flow
// #[derive(Component, Clone, Default, Debug)]
// pub struct PipeFlowState {
//     /// Current volume of liquid in the pipe
//     current_volume: f64,

//     /// Current pressure model for the pipe
//     pressure_model: HoopTubePressureModel,

//     /// Current flow model for each port (based on pipe conductance)
//     flow_model: [TurbulentFlowModel; 2],

//     /// Extrinsic pressure at ports: external pressure, pump
//     extrinsic_pressure: [f64; 2],

//     /// Intrinsic pressure on the pipe due to elasticity
//     intrinsic_pressure: f64,

//     /// Total pressure at ports
//     total_pressure: [f64; 2],

//     /// Junction pressure for each port.
//     junction_pressure: [f64; 2],

//     /// Flow into the pipe through port A and B
//     flow: [f64; 2],
// }

// impl PipeFlowState {
//     pub fn pipe_pressure(&self, port: PortTag) -> f64 {
//         self.total_pressure[port.index()]
//     }

//     pub fn mean_pipe_pressure(&self) -> f64 {
//         0.5 * (self.total_pressure[0] + self.total_pressure[1])
//     }

//     /// Pressure differential over the pipe
//     pub fn pipe_pressure_differential(&self, direction: FlowDirection) -> f64 {
//         let [i1, i2] = direction.indices();
//         self.total_pressure[i1] - self.total_pressure[i2]
//     }

//     pub fn flow(&self, port: PortTag) -> f64 {
//         self.flow[port.index()]
//     }

//     /// Combined flow into the pipe increasing it's stored volume
//     pub fn storage_flow(&self) -> f64 {
//         let [a, b] = [self.flow[0], self.flow[1]];
//         a + b
//     }

//     /// Flow through the pipe from port A to port B
//     pub fn through_flow(&self) -> f64 {
//         let [a, b] = [self.flow[0], self.flow[1]];
//         a.max(-b).min(0.) + a.min(-b).max(0.)
//     }
// }

/// Statistics for pipe
#[derive(Component, Clone, Default)]
pub struct PipeFlowStats {
    /// EMA of pressure at ports
    pressure_ema: [Ema; 2],

    /// EMA of flow through ports
    flow_ema: [Ema; 2],
}

impl PipeFlowStats {
    /// Pressure acting on the port
    pub fn pressure_ema(&self, port: PortTag) -> f64 {
        self.pressure_ema[port.index()].value()
    }

    /// Pressure differential over the pipe
    pub fn pressure_differential_ema(&self, direction: FlowDirection) -> f64 {
        let [i1, i2] = direction.indices();
        self.pressure_ema[i1].value() - self.pressure_ema[i2].value()
    }

    /// Combined flow into the pipe increasing it's stored volume
    pub fn storage_flow_ema(&self) -> f64 {
        let [a, b] = [self.flow_ema[0].value(), self.flow_ema[1].value()];
        a + b
    }

    /// Flow through the pipe from port A to port B
    pub fn through_flow_ema(&self) -> f64 {
        let [a, b] = [self.flow_ema[0].value(), self.flow_ema[1].value()];
        a.max(-b).min(0.) + a.min(-b).max(0.)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FlowDirection {
    AtoB,
    BtoA,
}

impl FlowDirection {
    pub fn ports(&self) -> [PortTag; 2] {
        match self {
            FlowDirection::AtoB => [PortTag::A, PortTag::B],
            FlowDirection::BtoA => [PortTag::B, PortTag::A],
        }
    }

    pub fn indices(&self) -> [usize; 2] {
        let [a, b] = self.ports();
        [a.index(), b.index()]
    }
}

/// Additional pressure applied to a pipe
#[derive(Component, Clone, Debug)]
pub struct ExternalPipePressure(pub f64);

/// A valve inside a pipe can block flow. The valve blocks both through flow and storage flow.
#[derive(Component, Clone, Default)]
pub struct ValveDef {
    /// If the valve is closed pipe conductance is reduced by this factor (must be non-zero)
    pub conductance_factor_closed: f64,

    /// If set the valve opens and closes ports automatically based on port pressure difference.
    pub kind: ValveKind,

    /// Hysteresis control value for opening and closing port valves. If 0 hysteresis is disabled.
    pub hysteresis: f64,
}

/// Active if current exceeds threshold and deactivates if current falls below threshold.
fn hysteresis(active: bool, current: f64, threshold: f64, factor: f64) -> bool {
    if active {
        if current < threshold / (1. + factor) {
            false
        } else {
            true
        }
    } else {
        if current > threshold * (1. + factor) {
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Default, PartialEq)]
pub enum ValveKind {
    /// Both ports are open for flow in both directions
    #[default]
    Open,

    /// No flow allowed through either port
    Closed,

    /// Only flow in the given direction is allowed.
    Throughflow(FlowDirection),

    /// Only inflow is allowed
    Inflow,

    /// Only outflow is allowed
    Outflow,
}

#[derive(Clone, PartialEq)]
pub enum PortFlowKind {
    Open,
    Closed,
    Outflow,
    Inflow,
}

impl ValveKind {
    pub fn port_kind(&self) -> [PortFlowKind; 2] {
        match self {
            ValveKind::Closed => [PortFlowKind::Closed, PortFlowKind::Closed],
            ValveKind::Open => [PortFlowKind::Open, PortFlowKind::Open],
            ValveKind::Throughflow(FlowDirection::AtoB) => {
                [PortFlowKind::Inflow, PortFlowKind::Outflow]
            }
            ValveKind::Throughflow(FlowDirection::BtoA) => {
                [PortFlowKind::Outflow, PortFlowKind::Inflow]
            }
            ValveKind::Inflow => [PortFlowKind::Inflow, PortFlowKind::Inflow],
            ValveKind::Outflow => [PortFlowKind::Outflow, PortFlowKind::Outflow],
        }
    }
}

/// State of a valve
#[derive(Component, Clone, Default)]
pub struct ValveState {
    /// Indicates whether ports are open
    pub is_open: [bool; 2],
}

/// A junction connects multiple pipes together. The junction itself is massless and does not store
/// material. Material flows between ports enforcing mass conservation.
///
/// Find junction pressure P s.t. total flow is zero.
/// Flow over i-th connector with conductance G_i and pressure p_i: Q_i = G_i * (P_i - P)
/// Mass conversation: sum Q_i = 0
/// Solve for P to get P = sum_i G_i P_i / sum_i G_i
#[derive(Component)]
pub struct Junction;

#[derive(Component, Default)]
struct JunctionState {
    solver: JunctionPressureSolver<Entity>,
}

/// Indicates the junction to which port A of a pipe is connected. There can only be one junction
/// per port.
#[derive(Component)]
pub struct PortAJunction;

/// Indicates the junction to which port B of a pipe is connected.
#[derive(Component)]
pub struct PortBJunction;

/// Automatically creates junctions when connecting pipes
pub struct PipeConnectionHelper<T>
where
    T: 'static + Send + Sync + Clone + Lerp<f64>,
{
    pipe_to_junc: HashMap<(Entity, PortTag), Entity>,
    builder: JunctionBuilder<T>,
}

impl<T> Default for PipeConnectionHelper<T>
where
    T: 'static + Send + Sync + Clone + Lerp<f64>,
{
    fn default() -> Self {
        Self {
            pipe_to_junc: HashMap::default(),
            builder: JunctionBuilder::default(),
        }
    }
}

impl<T> PipeConnectionHelper<T>
where
    T: 'static + Send + Sync + Clone + Lerp<f64>,
{
    /// Gets junction to which a pipe port is connected.
    pub fn junction(&self, pipe: Entity, port: PortTag) -> Option<Entity> {
        self.pipe_to_junc.get(&(pipe, port)).copied()
    }

    /// Joins the second junction into the first one thus connecting all pipe ports they are
    /// connected to.
    pub fn join_junctions(&mut self, world: &World, j1: Entity, j2: Entity) {
        // Find all pipe-ports connected to J2
        let j2_pps: Vec<(Entity, PortTag)> = self
            .pipe_to_junc
            .iter()
            .filter_map(|(k, v)| (**v == *j2).then(|| *k))
            .collect::<Vec<_>>();

        // Connect them to J1 instead
        for pp in j2_pps {
            let pe = world.entity_from_id(pp.0);
            Self::connect_f(pe, pp.1, j1);
            self.pipe_to_junc.insert(pp, j1);
        }

        // Delete J2
        world.remove(j2);
    }

    /// Connect a pipe port to a junction
    pub fn connect_to_junction<'a>(&mut self, p: (EntityView<'a>, PortTag), j: Entity) {
        self.pipe_to_junc.insert((*p.0, p.1), j);
    }

    /// Connect a pipe port to a new junction
    pub fn connect_to_new_junction<'a>(&mut self, p: (EntityView<'a>, PortTag)) -> Entity {
        let key = (*p.0, p.1);
        match self.pipe_to_junc.get(&key) {
            Some(junc) => *junc,
            None => {
                let world = p.0.world();
                let j = self.builder.build_unamed(&world);
                self.pipe_to_junc.insert(key, *j);
                *j
            }
        }
    }

    fn connect_f<'b>(e: EntityView<'b>, p: PortTag, j: Entity) {
        match p {
            PortTag::A => e.add((PortAJunction, j)),
            PortTag::B => e.add((PortBJunction, j)),
        };
    }

    /// Connects two ports of a pipe at a junction, building and merging junctions as necessary.
    pub fn connect<'a>(&mut self, p1: (EntityView<'a>, PortTag), p2: (EntityView<'a>, PortTag)) {
        let key1 = (*p1.0, p1.1);
        let key2 = (*p2.0, p2.1);

        let world = p1.0.world();

        match (
            self.pipe_to_junc.get(&key1).cloned(),
            self.pipe_to_junc.get(&key2).cloned(),
        ) {
            (None, None) => {
                // Neither pipe port is connected to a junction yet: create a new junction.

                let j = self.builder.build_unamed(&world);

                Self::connect_f(p1.0, p1.1, *j);
                self.pipe_to_junc.insert(key1, *j);

                Self::connect_f(p2.0, p2.1, *j);
                self.pipe_to_junc.insert(key2, *j);
            }
            (Some(j), None) => {
                // First pipe is connected to a junction already: also connect the other one.
                let j = world.entity_from_id(*j);
                Self::connect_f(p2.0, p2.1, *j);
                self.pipe_to_junc.insert(key2, *j);
            }
            (None, Some(j)) => {
                // Second pipe is connected to a junction already: also connect the other one.
                let j = world.entity_from_id(*j);
                Self::connect_f(p1.0, p1.1, *j);
                self.pipe_to_junc.insert(key1, *j);
            }
            (Some(j1), Some(j2)) => {
                // Both pipes are connected to a junction already: merge the junctions into one.
                self.join_junctions(&world, j1, j2);
            }
        }
    }

    /// Forms a chain of pipes. Ports are connected in their naturally order:
    ///   P1-B  A-P2-B  A-P3-B  ..  A-PN
    pub fn connect_chain<'a>(&mut self, p: &[EntityView<'a>]) {
        for ab in p.windows(2) {
            self.connect((ab[0], PortTag::B), (ab[1], PortTag::A));
        }
    }

    /// Forms a loop of pipes. Ports are connected in their naturally order:
    ///   P1-B  A-P2-B  A-P3-B  ..  A-PN-B  A-P1
    pub fn connect_loop<'a>(&mut self, p: &[EntityView<'a>]) {
        self.connect_chain(p);

        if let (Some(pn), Some(p1)) = (p.last(), p.first()) {
            self.connect((*pn, PortTag::B), (*p1, PortTag::A));
        }
    }

    /// Writes the pipe network as a Graphviz `.dot` file.
    pub fn write_dot(&self, world: &World, path: &str) -> std::io::Result<()> {
        use std::io::Write;

        let mut file = std::fs::File::create(path)?;
        writeln!(file, "graph PipeNetwork {{")?;

        // Collect which junctions each pipe connects
        let mut pipe_to_junctions: HashMap<Entity, Vec<Entity>> = HashMap::new();
        for ((pipe, _port), junc) in &self.pipe_to_junc {
            pipe_to_junctions.entry(*pipe).or_default().push(*junc);
        }

        // Emit all junction nodes with labels (entity name)
        let mut junctions: Vec<Entity> = self.pipe_to_junc.values().copied().collect();
        junctions.sort();
        junctions.dedup();
        for junc in &junctions {
            let junc_name = world.entity_from_id(**junc).name();
            writeln!(
                file,
                "    {} [label=\"{} ({})\"];",
                **junc as u64, junc_name, **junc as u64
            )?;
        }

        // Emit edges for each pipe
        for (pipe, junctions) in &pipe_to_junctions {
            let pipe_entity = world.entity_from_id(**pipe);
            let pipe_label = format!("{} ({})", pipe_entity.name(), **pipe as u64);

            match junctions.as_slice() {
                [j1, j2] => {
                    writeln!(
                        file,
                        "    {} -- {} [label=\"{}\"];",
                        **j1 as u64, **j2 as u64, pipe_label
                    )?;
                }
                [j1] => {
                    // Single-ended pipe: draw as a self-loop
                    writeln!(
                        file,
                        "    {} -- {} [label=\"{}\"];",
                        **j1 as u64, **j1 as u64, pipe_label
                    )?;
                }
                _ => {
                    // Handle unexpected >2 port pipes
                    for window in junctions.windows(2) {
                        let (a, b) = (window[0], window[1]);
                        writeln!(
                            file,
                            "    {} -- {} [label=\"{}\"];",
                            *a as u64, *b as u64, pipe_label
                        )?;
                    }
                }
            }
        }

        writeln!(file, "}}")?;
        Ok(())
    }
}

/// A pump creates a pressure differential on a pipe.
/// Positive pressure means liquid is pumped from port A to port B.
#[derive(Component, Clone)]
pub struct PumpDef {
    /// Maximum pressure at 0 flow
    pub max_pressure_differential: f64,

    /// Maximum flow after which pump pressure differential goes to 0
    pub max_flow: f64,

    /// Exponent used for flow-pressure curve; typically between 1 and 2
    pub flow_pressure_curve_exponential: f64,

    /// Liquid is pushed towards this port of the pipe
    pub outlet: PortTag,
}

impl PumpDef {
    /// Computes the effective pressure applied to Port A and B
    pub fn effective_pressure(&self, flow: f64) -> [f64; 2] {
        let flow = match self.outlet {
            PortTag::A => (-flow).max(0.),
            PortTag::B => flow.max(0.),
        };

        let dp = self.max_pressure_differential
            * (1. - (flow / self.max_flow).powf(self.flow_pressure_curve_exponential));

        let mut out = [0.; 2];
        out[self.outlet.index()] = dp;
        out
    }

    /// Pressure differential between port A and B
    pub fn effective_pressure_differential(&self, flow: f64) -> f64 {
        let [a, b] = self.effective_pressure(flow);
        a - b
    }
}

/// Factor on pump output. This is a simple model which scaled max pressure with the given factor.
#[derive(Component, Default, Clone)]
pub struct PumpPowerFactor(pub f64);

/// Pump state
#[derive(Component, Default, Clone)]
struct PumpState {
    /// Current pressure differential on ports
    dp: [f64; 2],
}

/// Pump statistics
#[derive(Component, Default, Clone)]
pub struct PumpStats {
    dummy: u32,
}

pub fn setup_flow_net<T: 'static + Send + Sync + Clone + Lerp<f64>>(world: &World) {
    world.component::<Vessel<T>>();
    world.component::<Pipe<T>>();

    // Operate pumps
    world
        .system_named::<(&PumpDef, &PipeFlowState, &mut PumpState)>("OperatePumps")
        .each(|(pump_def, pipe_state, pump_state)| {
            let flow = pipe_state.through_flow();
            pump_state.dp = pump_def.effective_pressure(flow);
        });

    // Limit power to pumps
    world
        .system_named::<(&PumpPowerFactor, &mut PumpState)>("LimitPumpPower")
        .each(|(ppf, pump_state)| {
            let a = ppf.0.clamp(0., 1.);
            pump_state.dp[0] *= a;
            pump_state.dp[1] *= a;
        });

    // Prepare pipe flow state
    world
        .system_named::<(&PipeGeometry, &Pipe<T>, &mut PipeFlowState)>("PipePreFlowPressureUpdate")
        .each(|(pipe_def, pipe, state)| {
            state.current_volume = pipe.volume();

            state.pressure_model =
                HoopTubePressureModel::new(pipe_def.tubes.clone(), pipe_def.collapse_pressure);

            // intrinsic pressure assuming no flow happens
            state.intrinsic_pressure = state.pressure_model.pressure(state.current_volume);

            state.extrinsic_pressure[0] = 0.;
            state.extrinsic_pressure[1] = 0.;

            let flow_model = pipe_def.flow_model(DENSITY_BLOOD, VISCOSITY_BLOOD); // FIXME
            state.flow_model[0] = flow_model.clone();
            state.flow_model[1] = flow_model;
        });

    // Adapt flow model based on valve state
    world
        .system_named::<(&ValveDef, &mut PipeFlowState, &mut ValveState)>("ValveFlowModel")
        .each(|(valve_def, pipe_state, valve_state)| {
            let factor = valve_def.conductance_factor_closed.max(0.);

            for i in 0..2 {
                if !valve_state.is_open[i] {
                    pipe_state.flow_model[i].apply_conductance_factor(factor);
                }
            }
        });

    // External pressure on pipes
    world
        .system_named::<(&ExternalPipePressure, &mut PipeFlowState)>("PipeExternalPressure")
        .each(|(ext, state)| {
            state.extrinsic_pressure[0] += ext.0;
            state.extrinsic_pressure[1] += ext.0;
        });

    // Pumps create pressure differential
    world
        .system_named::<(&PumpState, &mut PipeFlowState)>("PumpPressure")
        .each(|(pump_state, pipe_state)| {
            pipe_state.extrinsic_pressure[0] += pump_state.dp[0];
            pipe_state.extrinsic_pressure[1] += pump_state.dp[1];
        });

    // Compute junction pressure based on nominal pipe conductance

    fn junction_pressure(world: &World, tag: &str) {
        world
            .system_named::<(&mut Time, &mut JunctionState)>(&format!("JunctionStateReset{tag}"))
            .singleton_at(0)
            .each(|(time, junc_state)| {
                junc_state.solver.reset(time.sim_dt_f64());
            });

        fn junction_pressure_impl<R>(world: &World, rel: R, port: PortTag, name: &str)
        where
            Access: FromAccessArg<R>,
        {
            world
                .system_named::<(&PipeGeometry, &PipeFlowState, &mut JunctionState)>(name)
                .related(This, rel, "$junc")
                .tagged("$junc", Arg(2))
                .each_entity(move |pipe_entity, (pipe_geo, pipe_state, junc_state)| {
                    let pix = port.index();
                    junc_state.solver.add_port(
                        *pipe_entity,
                        pipe_geo.tubes.cylinder(),
                        pipe_state.current_volume,
                        pipe_state.extrinsic_pressure[pix],
                        pipe_geo.pressure_model(),
                        pipe_state.flow_model[pix].clone(),
                    );
                });
        }

        junction_pressure_impl(
            world,
            PortAJunction,
            PortTag::A,
            &format!("JunctionStateAddPortA{tag}"),
        );

        junction_pressure_impl(
            world,
            PortBJunction,
            PortTag::B,
            &format!("JunctionStateAddPortB{tag}"),
        );

        world
            .system_named::<(&mut JunctionState,)>(&format!("JunctionStateSolver{tag}"))
            .each(|(junc_state,)| match junc_state.solver.solve() {
                Ok(_)
                | Err(JunctionPressureSolverError::NoPorts)
                | Err(JunctionPressureSolverError::NoConductance) => {}
                Err(err) => {
                    log::warn!(
                        "junction pressure solver failed: {err:?}\n{:?}",
                        junc_state.solver
                    );
                }
            });
    }

    junction_pressure(world, "");

    // Store junction pressure at ports
    fn store_junction_pressure(world: &World, tag: &str) {
        fn store_junction_pressure_impl<R>(world: &World, rel: R, port: PortTag, name: &str)
        where
            Access: FromAccessArg<R>,
        {
            world
                .system_named::<(&mut PipeFlowState, &JunctionState)>(name)
                .related("$pipe", rel, This)
                .tagged("$pipe", Arg(0))
                .each(move |(pipe_state, junc_state)| {
                    let pix = port.index();
                    // Ignore errors as we complained about solution failures earlier already.
                    pipe_state.junction_pressure[pix] = junc_state
                        .solver
                        .pressure()
                        .unwrap_or(pipe_state.total_pressure[pix]);
                });
        }
        store_junction_pressure_impl(
            world,
            PortAJunction,
            PortTag::A,
            &format!("StoreJunctionPressurePortA{tag}"),
        );
        store_junction_pressure_impl(
            world,
            PortBJunction,
            PortTag::B,
            &format!("StoreJunctionPressurePortB{tag}"),
        );
    }
    store_junction_pressure(world, "");

    // Compute flow and exchange volume

    // TODO: Limit flow to avoid oscillations
    // The flow cannot be limited individually otherwise junctions would not preserve mass.
    // Needs further investigation.

    fn junction_inflow_impl<T, R>(world: &World, rel: R, port: PortTag, name: &str)
    where
        T: 'static + Send + Sync + Clone + Lerp<f64>,
        Access: FromAccessArg<R> + FromAccessArg<Junction>,
    {
        let pix = port.index();

        world
            .system_named::<(
                &Time,
                &mut Vessel<T>,
                &mut JunctionState,
                &mut Pipe<T>,
                &mut PipeFlowState,
            )>(name)
            .singleton_at(0)
            .related(This, rel, "$junc")
            .tagged("$junc", Junction)
            .tagged("$junc", Arg(1))
            .tagged("$junc", Arg(2))
            .each_entity(
                move |pipe_entity, (time, junc, junc_state, pipe, pipe_state)| {
                    let dt = time.sim_dt_f64();

                    // Compute flow based on pressure differential.
                    // Note that conductance is doubled as liquid exchange between pipe and junction
                    // (not throughflow!) has to travel on average only half the pipe length.
                    let flow = 2.0
                        * match junc_state.solver.flow(*pipe_entity) {
                            Ok(flow) => flow,
                            Err(JunctionPressureFlowError::InvalidIndex) => {
                                // shouldn't happen ..
                                log::warn!("invalid entity");
                                0.
                            }
                            Err(JunctionPressureFlowError::SolverError(err)) => {
                                // In case something went wrong just disable flow over port
                                log::warn!("flow error: {err:?}, pipe: {pipe_entity}");
                                0.
                            }
                        };
                    pipe_state.flow[pix] = flow;

                    // println!("{}: {pix} {:?}", pipe_entity.name(), volume_to_liters(flow));

                    // Phase 1: move liquid from pipes into junction buffer
                    if flow < 0. {
                        let volume = -dt * flow;
                        for chunk in pipe.drain(port, volume) {
                            junc.fill(chunk);
                        }
                    }
                },
            );
    }

    junction_inflow_impl::<T, _>(world, PortAJunction, PortTag::A, "JunctionInflowPortA");
    junction_inflow_impl::<T, _>(world, PortBJunction, PortTag::B, "JunctionInflowPortB");

    fn junction_outflow_impl<T, R>(world: &World, rel: R, port: PortTag, name: &str)
    where
        T: 'static + Send + Sync + Clone + Lerp<f64>,
        Access: FromAccessArg<R> + FromAccessArg<Junction>,
    {
        world
            .system_named::<(&Time, &mut Vessel<T>, &mut Pipe<T>, &mut PipeFlowState)>(name)
            .singleton_at(0)
            .with(Junction)
            .related("$pipe", rel, This)
            .tagged("$pipe", Arg(2))
            .tagged("$pipe", Arg(3))
            .each(move |(time, junc, pipe, pipe_state)| {
                let flow = pipe_state.flow[port.index()];

                // Phase 2: move liquid from junction buffer into pipes
                if flow > 0. {
                    let volume = time.sim_dt_f64() * flow;
                    if let Some(chunk) = junc.drain(volume) {
                        pipe.fill(port, chunk);
                    }
                }
            });
    }
    junction_outflow_impl::<T, _>(world, PortAJunction, PortTag::A, "JunctionOutflowPortA");
    junction_outflow_impl::<T, _>(world, PortBJunction, PortTag::B, "JunctionOutflowPortB");

    world
        .system_named::<(&Pipe<T>, &mut PipeFlowState)>("UpdatePressure")
        .each(|(pipe, pipe_state)| {
            pipe_state.current_volume = pipe.volume();
            pipe_state.intrinsic_pressure = pipe_state
                .pressure_model
                .pressure(pipe_state.current_volume);

            pipe_state.total_pressure[0] =
                pipe_state.extrinsic_pressure[0] + pipe_state.intrinsic_pressure;
            pipe_state.total_pressure[1] =
                pipe_state.extrinsic_pressure[1] + pipe_state.intrinsic_pressure;

            println!(
                "{:8.3?} {:8.3?} {:8.3?} {:8.3?}",
                volume_to_milli_liters(pipe_state.current_volume),
                pipe_state.intrinsic_pressure,
                pipe_state.junction_pressure,
                [
                    volume_to_milli_liters(pipe_state.flow[0]),
                    volume_to_milli_liters(pipe_state.flow[1])
                ]
            );
        });

    world
        .system_named::<(&mut Vessel<T>,)>("ClearJunctionBuffers")
        .with(Junction)
        .each(|(junc,)| {
            let chunk = junc.drain_all();
            let leftover = chunk.map_or(0., |c| c.volume);
            if leftover > FLOW_CONSERVATION_THRESHOLD {
                log::error!("fluid flow not preserved at function: leftover={leftover}");
            }
        });

    // Operate valves based on pressure differential
    world
        .system_named::<(&ValveDef, &mut PipeFlowState, &mut ValveState)>("OperateValves")
        .each(|(valve_def, pipe_state, valve_state)| {
            let port_flow_kind = valve_def.kind.port_kind();

            for i in 0..2 {
                let hf = |active| {
                    hysteresis(
                        active,
                        pipe_state.total_pressure[i],
                        pipe_state.junction_pressure[i],
                        valve_def.hysteresis,
                    )
                };

                let is_open = match port_flow_kind[i] {
                    PortFlowKind::Open => true,
                    PortFlowKind::Closed => false,
                    PortFlowKind::Outflow => hf(valve_state.is_open[i]),
                    PortFlowKind::Inflow => !hf(!valve_state.is_open[i]),
                };

                valve_state.is_open[i] = is_open;
            }
        });

    // Flow estimation for pipes
    world
        .system_named::<(&Time, &PipeFlowState, &mut PipeFlowStats)>("PipeStatistics")
        .singleton_at(0)
        .each(|(t, state, stats)| {
            let dt = t.sim_dt.as_secs_f64();
            stats.pressure_ema[0].step(dt, state.total_pressure[0]);
            stats.pressure_ema[1].step(dt, state.total_pressure[1]);
            stats.flow_ema[0].step(dt, state.flow[0]);
            stats.flow_ema[1].step(dt, state.flow[1]);
        });
}

pub struct PipeBuilder<'a, T> {
    pub geometry: &'a PipeGeometry,
    pub data: &'a T,

    /// The pipe is filled with liquid to establish this pressure [Pa]
    /// TODO this is not very accurate at the moment and needs further work.
    pub target_pressure: f64,
}

impl<T> EntityBuilder for PipeBuilder<'_, T>
where
    T: 'static + Send + Sync + Clone + Lerp<f64>,
{
    fn build<'a>(&self, _world: &'a World, entity: EntityView<'a>) -> EntityView<'a> {
        let pressure_model = self.geometry.pressure_model();

        let volume = match pressure_model.volume(self.target_pressure) {
            Ok(v) => v,
            Err(err) => {
                log::warn!(
                    "failed to compute volume to reach target pressure: {err:?}, P={}\n{:?}",
                    self.target_pressure,
                    self.geometry
                );
                err.best_guess()
            }
        };

        entity
            .set(self.geometry.clone())
            .set(
                Pipe::new()
                    .filled(
                        PortTag::A,
                        FluidChunk {
                            volume,
                            data: self.data.clone(),
                        },
                    )
                    .with_min_chunk_volume(volume_from_milli_liters(50.)),
            )
            .set(PipeFlowState::default())
            .set(PipeFlowStats::default())
    }
}

pub struct JunctionBuilder<T>(PhantomData<T>);

impl<T> Default for JunctionBuilder<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T> EntityBuilder for JunctionBuilder<T>
where
    T: 'static + Send + Sync + Clone + Lerp<f64>,
{
    fn build<'a>(&self, _world: &'a World, entity: EntityView<'a>) -> EntityView<'a> {
        entity
            .add(Junction)
            .set(Vessel::<T>::default())
            .set(JunctionState::default())
    }
}

pub struct PumpBuilder<'a> {
    pub def: &'a PumpDef,
}

impl EntityBuilder for PumpBuilder<'_> {
    fn build<'a>(&self, _world: &'a World, entity: EntityView<'a>) -> EntityView<'a> {
        entity
            .set(self.def.clone())
            .set(PumpState::default())
            .set(PumpStats::default())
    }
}

pub struct ValveBuilder<'a> {
    pub def: &'a ValveDef,
}

impl EntityBuilder for ValveBuilder<'_> {
    fn build<'a>(&self, _world: &'a World, entity: EntityView<'a>) -> EntityView<'a> {
        entity.set(self.def.clone()).set(ValveState::default())
    }
}

impl Module for FlowNetModule {
    fn module(world: &World) {
        world.module::<FlowNetModule>("FlowNetModule");

        world.import::<TimeModule>();

        world.component::<FlowNetConfig>();

        world.component::<PipeGeometry>();
        world.component::<PipeFlowState>();
        world.component::<PipeFlowStats>();
        world.component::<ExternalPipePressure>();

        world.component::<ValveDef>();
        world.component::<ValveState>();

        world.component::<Junction>();
        world.component::<JunctionState>();
        world
            .component::<PortAJunction>()
            .add_trait::<flecs::Exclusive>();
        world
            .component::<PortBJunction>()
            .add_trait::<flecs::Exclusive>();

        world.component::<PumpDef>();
        world.component::<PumpPowerFactor>();
        world.component::<PumpState>();
        world.component::<PumpStats>();

        world.set(FlowNetConfig {
            flow_factor: 0.0002,
        });
    }
}
