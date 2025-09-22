use crate::{EntityBuilder, ecs::prelude::*};
use candy_time::{CandyTimeMocca, Time};
use flowsim::{
    FlowNetSolver, FluidChunk, FluidComposition, FluidDensityViscosity, PipeDef, PipeJunctionPort,
    PipeScratch, PipeState, PipeStateDerivative, PipeVessel, PortMap, PortTag, ReservoirVessel,
    SolutionDeltaVolume,
    models::{Bundle, ElasticTube, HoopTubePressureModel, PressureModel},
};
use gems::{Ema, RateEma, VolumeModel, volume_from_milli_liters};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

pub struct FlowSimMocca;

/// Statistics for pipe
#[derive(Component, Clone, Default)]
pub struct PipeFlowStats {
    /// EMA of pressure at ports
    pressure_ema: [Ema; 2],

    /// EMA of flow through ports
    flow_ema: [RateEma; 2],
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
    pub fn flow_ema(&self, side: PortTag) -> f64 {
        self.flow_ema[side.index()].value()
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

/// Internal state used for computation of liquid flow
#[derive(Component, Clone, Default, Debug)]
pub struct PipeFlowState {
    // /// Extrinsic pressure at ports: external pressure, pump
    // extrinsic_pressure: [f64; 2],

    // /// Intrinsic pressure on the pipe due to elasticity
    // intrinsic_pressure: f64,
    /// Total pressure at ports
    pressure: PortMap<f64>,

    /// Junction pressure for each port.
    junction_pressure: PortMap<Option<f64>>,

    /// Flow into the pipe through port A and B
    flow: PortMap<f64>,
}

impl PipeFlowState {
    pub fn pressure(&self, port: PortTag) -> f64 {
        self.pressure[port.index()]
    }

    pub fn mean_pressure(&self) -> f64 {
        0.5 * (self.pressure[0] + self.pressure[1])
    }

    /// Pressure differential over the pipe
    pub fn pressure_differential(&self, direction: FlowDirection) -> f64 {
        let [i1, i2] = direction.indices();
        self.pressure[i1] - self.pressure[i2]
    }

    pub fn flow(&self, port: PortTag) -> f64 {
        self.flow[port.index()]
    }

    /// Combined flow into the pipe increasing it's stored volume
    pub fn storage_flow(&self) -> f64 {
        let [a, b] = [self.flow[0], self.flow[1]];
        a + b
    }

    /// Flow through the pipe from port A to port B
    pub fn through_flow(&self) -> f64 {
        let [a, b] = [self.flow[0], self.flow[1]];
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
pub struct ExternalPipePressure(pub PortMap<f64>);

impl ExternalPipePressure {
    pub fn ubiquous(p: f64) -> Self {
        ExternalPipePressure(PortMap::from_array([p, p]))
    }
}

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
    /// Allow flow in both directions
    Open,

    /// Do not allow flow in any direction
    Closed,

    /// Only allow flow into the pipe (positive flow)
    Inflow,

    /// Only allow flow out of the pipe (negative flow)
    Outflow,
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
pub struct JunctionState {
    pub dummy: u32,
}

/// Automatically creates junctions when connecting pipes
pub struct PipeConnectionHelper {
    pipe_to_junc: HashMap<(Entity, PortTag), Entity>,
    builder: JunctionBuilder,
}

impl Default for PipeConnectionHelper {
    fn default() -> Self {
        Self {
            pipe_to_junc: HashMap::default(),
            builder: JunctionBuilder::default(),
        }
    }
}

impl PipeConnectionHelper {
    /// Gets junction to which a pipe port is connected.
    pub fn junction(&self, pipe: Entity, port: PortTag) -> Option<Entity> {
        self.pipe_to_junc.get(&(pipe, port)).copied()
    }

    /// Joins the second junction into the first one thus connecting all pipe ports they are
    /// connected to.
    pub fn join_junctions(&mut self, world: &mut World, j1: Entity, j2: Entity) {
        // Find all pipe-ports connected to J2
        let j2_pps: Vec<(Entity, PortTag)> = self
            .pipe_to_junc
            .iter()
            .filter_map(|(k, v)| (*v == j2).then(|| *k))
            .collect::<Vec<_>>();

        // Connect them to J1 instead
        for pp in j2_pps {
            Self::connect_f(world, pp.0, pp.1, j1);
            self.pipe_to_junc.insert(pp, j1);
        }

        // Delete J2
        world.despawn(j2);
    }

    /// Connect a pipe port to a junction
    pub fn connect_to_junction<'a>(&mut self, p: (Entity, PortTag), j: Entity) {
        self.pipe_to_junc.insert((p.0, p.1), j);
    }

    /// Connect a pipe port to a new junction
    pub fn connect_to_new_junction<'a>(
        &mut self,
        world: &mut World,
        p: (Entity, PortTag),
    ) -> Entity {
        let key = (p.0, p.1);
        match self.pipe_to_junc.get(&key) {
            Some(junc) => *junc,
            None => {
                let j = self.builder.build_unamed(world);
                self.pipe_to_junc.insert(key, j.id());
                j.id()
            }
        }
    }

    fn connect_f<'b>(world: &mut World, e: Entity, p: PortTag, j: Entity) {
        world.entity(e).unwrap().add((PipeJunctionPort(p), j))
    }

    /// Connects two ports of a pipe at a junction, building and merging junctions as necessary.
    pub fn connect<'a>(&mut self, world: &mut World, p1: (Entity, PortTag), p2: (Entity, PortTag)) {
        let key1 = (p1.0, p1.1);
        let key2 = (p2.0, p2.1);

        match (
            self.pipe_to_junc.get(&key1).cloned(),
            self.pipe_to_junc.get(&key2).cloned(),
        ) {
            (None, None) => {
                // Neither pipe port is connected to a junction yet: create a new junction.

                let j = self.builder.build_unamed(world).id();

                Self::connect_f(world, p1.0, p1.1, j);
                self.pipe_to_junc.insert(key1, j);

                Self::connect_f(world, p2.0, p2.1, j);
                self.pipe_to_junc.insert(key2, j);
            }
            (Some(j), None) => {
                // First pipe is connected to a junction already: also connect the other one.
                Self::connect_f(world, p2.0, p2.1, j);
                self.pipe_to_junc.insert(key2, j);
            }
            (None, Some(j)) => {
                // Second pipe is connected to a junction already: also connect the other one.
                Self::connect_f(world, p1.0, p1.1, j);
                self.pipe_to_junc.insert(key1, j);
            }
            (Some(j1), Some(j2)) => {
                // Both pipes are connected to a junction already: merge the junctions into one.
                self.join_junctions(world, j1, j2);
            }
        }
    }

    /// Forms a chain of pipes. Ports are connected in their naturally order:
    ///   P1-B  A-P2-B  A-P3-B  ..  A-PN
    pub fn connect_chain(&mut self, world: &mut World, pipes: &[Entity]) {
        for ab in pipes.windows(2) {
            self.connect(world, (ab[0], PortTag::B), (ab[1], PortTag::A));
        }
    }

    /// Forms a loop of pipes. Ports are connected in their naturally order:
    ///   P1-B  A-P2-B  A-P3-B  ..  A-PN-B  A-P1
    pub fn connect_loop(&mut self, world: &mut World, pipes: &[Entity]) {
        self.connect_chain(world, pipes);

        if let (Some(pn), Some(p1)) = (pipes.last(), pipes.first()) {
            self.connect(world, (*pn, PortTag::B), (*p1, PortTag::A));
        }
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

pub struct PipeBuilder {
    pub tube: ElasticTube,
    pub strand_count: f64,
    pub fluid: FluidComposition,

    /// The pipe is filled with liquid to establish this pressure [Pa]
    /// TODO this is not very accurate at the moment and needs further work.
    pub target_pressure: f64,
}

impl EntityBuilder for PipeBuilder {
    fn build<'a>(&self, entity: EntityWorldMut<'a>) -> EntityWorldMut<'a> {
        let shape = Bundle {
            model: self.tube.shape.clone(),
            count: self.strand_count,
        };

        let elasticity_pressure_model = Bundle {
            model: HoopTubePressureModel::new(self.tube.clone(), -1000.0),
            count: self.strand_count,
        };

        let volume =
            match elasticity_pressure_model.volume(self.target_pressure, shape.nominal_volume()) {
                Ok(v) => v,
                Err(err) => {
                    println!("error");
                    log::warn!(
                        "failed to compute volume to reach target pressure: {err:?}, P={}\n{:?}",
                        self.target_pressure,
                        elasticity_pressure_model
                    );
                    err.best_guess()
                }
            };

        let pipe = PipeDef {
            shape,
            fluid: FluidDensityViscosity::blood(),
            external_port_pressure: PortMap::from_array([0., 0.]),
            elasticity_pressure_model,
            ground_angle: 0.,
            darcy_factor: 64. / 2000., // e.g. 64/Re
            dampening: 0.0,
            port_area_factor: PortMap::from_array([1., 1.]),
        };

        entity
            .and_set(pipe)
            .and_set(PipeState {
                volume,
                velocity: PortMap::default(),
            })
            .and_set(
                PipeVessel::new()
                    .filled(
                        PortTag::A,
                        FluidChunk::from_fluid_with_volume(self.fluid.clone(), volume),
                    )
                    .with_min_chunk_volume(volume_from_milli_liters(50.)),
            )
            .and_set(PipeFlowState::default())
            .and_set(PipeFlowStats::default())
    }
}

#[derive(Default)]
pub struct JunctionBuilder;

impl EntityBuilder for JunctionBuilder {
    fn build<'a>(&self, entity: EntityWorldMut<'a>) -> EntityWorldMut<'a> {
        entity
            .and_add(Junction)
            .and_set(ReservoirVessel::default())
            .and_set(JunctionState::default())
    }
}

pub struct PumpBuilder<'a> {
    pub def: &'a PumpDef,
}

impl EntityBuilder for PumpBuilder<'_> {
    fn build<'a>(&self, entity: EntityWorldMut<'a>) -> EntityWorldMut<'a> {
        entity
            .and_set(self.def.clone())
            .and_set(PumpState::default())
            .and_set(PumpStats::default())
    }
}

pub struct ValveBuilder<'a> {
    pub def: &'a ValveDef,
}

impl EntityBuilder for ValveBuilder<'_> {
    fn build<'a>(&self, entity: EntityWorldMut<'a>) -> EntityWorldMut<'a> {
        entity
            .and_set(self.def.clone())
            .and_set(ValveState::default())
    }
}

#[derive(Singleton, Default, Clone)]
pub struct FlowSimConfig {
    /// If set writes pipe statistics to CSV file
    pub pipe_stats_csv_path: Option<PathBuf>,

    /// If set writes the topology of the flow net to a DOT file
    pub graph_topology_path: Option<PathBuf>,

    /// If enabled prints the flow sim ODE to console after solving
    pub debug_print_ode_solution: bool,
}

impl Mocca for FlowSimMocca {
    fn load(mut dep: MoccaDeps) {
        dep.depends_on::<CandyTimeMocca>();
    }

    fn register_components(world: &mut World) {
        world.register_component::<FlowSimConfig>();

        world.register_component::<PipeDef>();
        world.register_component::<PipeState>();
        world.register_component::<PipeVessel>();
        world.register_component::<ReservoirVessel>();
        world.register_component::<PipeFlowState>();
        world.register_component::<PipeFlowStats>();
        world.register_component::<ExternalPipePressure>();

        world.register_component::<ValveDef>();
        world.register_component::<ValveState>();

        world.register_component::<Junction>();
        world.register_component::<JunctionState>();
        world.register_component::<PipeJunctionPort>();

        world.register_component::<PumpDef>();
        world.register_component::<PumpPowerFactor>();
        world.register_component::<PumpState>();
        world.register_component::<PumpStats>();
    }

    fn start(world: &mut World) -> Self {
        world.set_singleton(FlowSimConfig::default());

        Self
    }

    fn step(&mut self, world: &mut World) {
        // Apply external pressure  on pipes
        world
            .query::<(&ExternalPipePressure, &mut PipeDef)>()
            .each_mut(|(ext, state)| {
                state.external_port_pressure = ext.0;
            });

        // solve flow net
        let dt = world.singleton::<Time>().sim_dt_f64();
        FlowNetSolver::new().step(world, dt);

        if world.singleton::<FlowSimConfig>().debug_print_ode_solution {
            world.run(flowsim::print_junction_overview);
            world.run(flowsim::print_pipe_overview);
        }

        // Phase 1: liquid flows from pipes vessels into the transionary junction vessels
        world
            .query_filtered::<(
                &SolutionDeltaVolume,
                &mut PipeVessel,
                (This, &PipeJunctionPort, E1),
                (&mut ReservoirVessel, E1),
            ), (With<(Junction, E1)>,)>()
            .each_mut(
                |(delta, pipe_vessel, &PipeJunctionPort(port), junc_vessel)| {
                    let delta_volume = delta.delta_volume[port];
                    if delta_volume < 0. {
                        for chunk in pipe_vessel.drain(port, -delta_volume) {
                            junc_vessel.fill(chunk);
                        }
                    }
                },
            );

        // Phase 2: liquid flow from junction vessels into pipes
        world
            .query_filtered::<(
                &SolutionDeltaVolume,
                &mut PipeVessel,
                (This, &PipeJunctionPort, E1),
                (&mut ReservoirVessel, E1),
            ), (With<(Junction, E1)>,)>()
            .each_mut(
                |(delta, pipe_vessel, &PipeJunctionPort(port), junc_vessel)| {
                    let delta_volume = delta.delta_volume[port];
                    if delta_volume > 0. {
                        if let Some(chunk) = junc_vessel.drain(delta_volume) {
                            pipe_vessel.fill(port, chunk);
                        }
                    }
                },
            );

        // write back state
        world
            .query::<(
                &PipeDef,
                &PipeScratch,
                &PipeStateDerivative,
                &SolutionDeltaVolume,
                &mut PipeFlowState,
                &mut PipeFlowStats,
            )>()
            .each_mut(|(_def, scr, derivative, delta, state, stats)| {
                state.junction_pressure = PortMap::from_array(scr.junction_pressure);

                for i in [0, 1] {
                    // P = F/A = a / (A/m)
                    let pressure = derivative.accel[i] / scr.area_per_mass;
                    state.pressure[i] = pressure;
                    stats.pressure_ema[i].step(dt, pressure);

                    let delta_volume = delta.delta_volume[i];
                    let flow = delta_volume / dt;
                    state.flow[i] = flow;
                    stats.flow_ema[i].step(dt, delta_volume);
                }
            });

        // Operate valves based on pressure differential
        world
            .query::<(&ValveDef, &mut ValveState, &mut PipeDef, &mut PipeFlowState)>()
            .each_mut(|(valve_def, valve_state, pipe_def, pipe_state)| {
                let port_flow_kind = valve_def.kind.port_kind();

                for i in 0..2 {
                    let is_open = if let Some(junction_pressure) = pipe_state.junction_pressure[i] {
                        // println!("{}/{}", pipe_state.pressure[i], junction_pressure);

                        match port_flow_kind[i] {
                            PortFlowKind::Open => true,
                            PortFlowKind::Closed => false,
                            PortFlowKind::Inflow => hysteresis(
                                valve_state.is_open[i],
                                pipe_state.pressure[i],
                                -junction_pressure,
                                valve_def.hysteresis,
                            ),
                            PortFlowKind::Outflow => hysteresis(
                                valve_state.is_open[i],
                                -pipe_state.pressure[i],
                                junction_pressure,
                                valve_def.hysteresis,
                            ),
                        }
                    } else {
                        false
                    };

                    valve_state.is_open[i] = is_open;
                    pipe_def.port_area_factor[i] = if is_open { 1. } else { 0. };
                    // println!("VALVE: {}", is_open);
                }
            });

        // Write pipe data to CSV
        if let Some(path) = &world.singleton::<FlowSimConfig>().pipe_stats_csv_path {
            let step = world.singleton::<Time>().frame_number();
            let file_path = path.join(format!("flow_net_pipes_{:05}.csv", step.as_u64()));

            write_flow_net_pipes_csv(world, &file_path).ok();
        }

        // Write graph topology to CSV
        if let Some(path) = &world.singleton::<FlowSimConfig>().graph_topology_path {
            let step = world.singleton::<Time>().frame_number();
            let file_path = path.join(format!("topology_{:05}.csv", step.as_u64()));

            write_flow_net_topology_dot(world, &file_path).ok();
        }
    }
}

/// Active if current exceeds threshold and deactivates if current falls below threshold.
fn hysteresis(active: bool, current: f64, threshold: f64, factor: f64) -> bool {
    if active {
        // stay on for longer
        current >= threshold / (1. + factor)
    } else {
        // turn on a bit later
        current > threshold * (1. + factor)
    }
}

fn write_flow_net_pipes_csv(world: &mut World, file_path: &Path) -> std::io::Result<()> {
    use std::{
        fs::File,
        io::{BufWriter, Write},
    };

    let file = File::create(file_path)?;
    let mut writer = BufWriter::new(file);

    // Write header
    writeln!(
        writer,
        "entity,name,volume,length,pressure_a,pressure_b,junction_a,junction_b,flow_a,flow_b"
    )?;

    world
        .query::<(This, &PipeDef, &PipeFlowState, &PipeVessel, Option<&Name>)>()
        .each(|(entity, def, state, vessel, name)| {
            let volume = vessel.volume();
            let length = def.shape.model.length;
            let pressure_a = state.pressure[PortTag::A];
            let pressure_b = state.pressure[PortTag::B];
            let junction_a = state.junction_pressure[PortTag::A]
                .map(|p| p.to_string())
                .unwrap_or_else(|| String::new());
            let junction_b = state.junction_pressure[PortTag::B]
                .map(|p| p.to_string())
                .unwrap_or_else(|| String::new());
            let flow_a = state.flow[PortTag::A];
            let flow_b = state.flow[PortTag::B];
            let open_a = def.port_area_factor[PortTag::A];
            let open_b = def.port_area_factor[PortTag::B];

            writeln!(
                writer,
                "{},{:?},{volume},{length},{pressure_a},{pressure_b},{junction_a},{junction_b},{flow_a},{flow_b},{open_a},{open_b}",
                entity, name.unwrap_or_str("N/A")
            )
            .unwrap();
        });

    Ok(())
}

fn write_flow_net_topology_dot(world: &mut World, file_path: &Path) -> std::io::Result<()> {
    use std::{
        collections::HashSet,
        fs::File,
        io::{BufWriter, Write},
    };

    // Helper: stable Graphviz ID for an entity and a safe label.
    fn gv_id(e: Entity) -> String {
        format!("e{}", e)
    }

    fn gv_label(e: Entity, name: Option<&Name>) -> String {
        // Use the entity's name if present; otherwise fall back to its id.
        match name {
            Some(n) => {
                let n = n.as_str();
                if !n.is_empty() {
                    n.replace('"', r#"\""#)
                } else {
                    format!("id={}", e)
                }
            }
            _ => format!("id={}", e),
        }
    }

    let file = File::create(file_path)?;
    let mut w = BufWriter::new(file);

    writeln!(w, "graph FlowNet {{")?;
    writeln!(w, "  graph [overlap=false];")?;
    writeln!(w, "  node  [fontname=\"Helvetica\"];")?;

    let mut seen_pipes: HashSet<Entity> = HashSet::new();
    let mut seen_juncs: HashSet<Entity> = HashSet::new();
    let mut seen_edges: HashSet<(Entity, Entity)> = HashSet::new();

    world.query_filtered::<(This, E1, Option<&Name>), (
        With<(PipeVessel, This)>,
        With<(This, PipeJunctionPort, E1)>,
        With<(Junction, E1)>,
        With<(ReservoirVessel, E1)>,
    )>().each(|(pipe, junc,pipe_name,)| {
            let pipe_gv_id = gv_id(pipe);
            let pipe_gv_label = gv_label(pipe, pipe_name);
            let junc_gv_id = gv_id(junc);

            // Node declarations (once).
            if seen_pipes.insert(pipe) {
                writeln!(
                    w,
                    "  {} [label=\"{}\", shape=box, style=rounded, penwidth=1.2];",
                    pipe_gv_id,
                    pipe_gv_label
                ).ok();
            }
            if seen_juncs.insert(junc) {
                writeln!(
                    w,
                    "  {} [label=\"\", shape=circle, width=0.15, fixedsize=true, style=filled, fillcolor=\"#666666\"];",
                    junc_gv_id
                ).ok();
            }

            // Edges (once across both PortA/PortB passes).
            if seen_edges.insert((pipe, junc)) {
                writeln!(w, "  {} -- {};", pipe_gv_id, junc_gv_id).ok();
            }
        });

    writeln!(w, "}}")?;
    Ok(())
}
