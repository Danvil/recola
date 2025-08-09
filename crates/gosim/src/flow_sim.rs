use crate::{utils::FlecsQueryRelationHelpers, Arg, EntityBuilder, This, Time, TimeMocca};
use flecs_ecs::prelude::{World, *};
use flowsim::{
    models::{Bundle, ElasticTube, HoopTubePressureModel, PressureModel},
    FlowNet, FlowNetSolver, FluidChunk, FluidComposition, FluidDensityViscosity, JuncId, PipeDef,
    PipeId, PipeSolution, PipeState, PipeVessel, PortMap, PortTag, ReservoirVessel,
};
use gems::{volume_from_milli_liters, Ema, IntMap, RateEma, VolumeModel};
use mocca::{Mocca, MoccaDeps};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

pub struct FlowSimMocca;

#[derive(Component, Clone, Debug)]
pub struct FlowNetPipeDef(pub flowsim::PipeDef);

#[derive(Component, Clone, Debug)]
pub struct FlowNetPipeState(pub flowsim::PipeState);

#[derive(Component, Clone, Debug)]
pub struct FlowNetPipeVessel(pub flowsim::PipeVessel);

#[derive(Component, Clone, Debug)]
pub struct FlowNetReservoirVessel(pub flowsim::ReservoirVessel);

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

/// Indicates the junction to which port A of a pipe is connected. There can only be one junction
/// per port.
#[derive(Component)]
pub struct PortAJunction;

/// Indicates the junction to which port B of a pipe is connected.
#[derive(Component)]
pub struct PortBJunction;

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
    fn build<'a>(&self, _world: &'a World, entity: EntityView<'a>) -> EntityView<'a> {
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
            name: entity.name(),
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
            .set(FlowNetPipeDef(pipe))
            .set(FlowNetPipeState(PipeState {
                volume,
                velocity: PortMap::default(),
            }))
            .set(FlowNetPipeVessel(
                PipeVessel::new()
                    .filled(
                        PortTag::A,
                        FluidChunk::from_fluid_with_volume(self.fluid.clone(), volume),
                    )
                    .with_min_chunk_volume(volume_from_milli_liters(50.)),
            ))
            .set(PipeFlowState::default())
            .set(PipeFlowStats::default())
    }
}

#[derive(Default)]
pub struct JunctionBuilder;

impl EntityBuilder for JunctionBuilder {
    fn build<'a>(&self, _world: &'a World, entity: EntityView<'a>) -> EntityView<'a> {
        entity
            .add(Junction)
            .set(FlowNetReservoirVessel(ReservoirVessel::default()))
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

#[derive(Component, Default, Clone)]
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
        dep.dep::<TimeMocca>();
    }

    fn register_components(world: &World) {
        world.component::<FlowSimConfig>();

        world.component::<FlowNetPipeDef>();
        world.component::<FlowNetPipeState>();
        world.component::<FlowNetPipeVessel>();
        world.component::<FlowNetReservoirVessel>();
        // world.component::<PipeFlowState>();
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
    }

    fn start(world: &World) -> Self {
        world.set(FlowSimConfig::default());

        Self
    }

    fn step(&mut self, world: &World) {
        // Apply external pressure  on pipes
        world.each::<(&ExternalPipePressure, &mut FlowNetPipeDef)>(|(ext, state)| {
            state.0.external_port_pressure = ext.0;
        });

        // Extract flow net topology and pipe state

        let mut net = FlowNet::new();
        let mut net_pipe_state = IntMap::new();
        let mut pipe_entity_id_map = HashMap::new();
        let mut junc_entity_id_map = HashMap::new();

        world
            .query::<(&FlowNetPipeDef, &FlowNetPipeState)>()
            .build()
            .each_entity(|epipe, (def, state)| {
                let id = net.insert_pipe(def.0.clone());
                pipe_entity_id_map.insert(*epipe, id);
                net_pipe_state.set(*id, state.0.clone());
            });

        // println!("net: {:?}", net);
        // println!("pipe_entity_id_map: {:?}", pipe_entity_id_map);

        fn build_flow_net<R>(
            world: &World,
            net: &mut FlowNet,
            pipe_entity_id_map: &mut HashMap<Entity, PipeId>,
            junc_entity_id_map: &mut HashMap<Entity, JuncId>,
            rel: R,
            side: PortTag,
        ) where
            Access: FromAccessArg<R>,
        {
            let topo_query = world
                .query::<(&FlowNetPipeDef, &mut JunctionState)>()
                .related(This, rel, "$junc")
                .tagged("$junc", Arg(1))
                .build();

            let junc_var = topo_query.find_var("junc").unwrap();

            topo_query.run(|mut it| {
                while it.next() {
                    for i in it.iter() {
                        let epipe = it.entity(i).unwrap();
                        let pipe_id = pipe_entity_id_map.get(&epipe).unwrap();
                        let ejunc = it.get_var(junc_var);

                        match junc_entity_id_map.get(&*ejunc) {
                            Some(junc_id) => {
                                net.topology.connect_to_junction((*pipe_id, side), *junc_id);
                            }
                            None => {
                                let junc_id =
                                    net.topology.connect_to_new_junction((*pipe_id, side));
                                junc_entity_id_map.insert(*ejunc, junc_id);
                            }
                        }
                    }
                }
            })
        }
        build_flow_net(
            world,
            &mut net,
            &mut pipe_entity_id_map,
            &mut junc_entity_id_map,
            PortAJunction,
            PortTag::A,
        );
        build_flow_net(
            world,
            &mut net,
            &mut pipe_entity_id_map,
            &mut junc_entity_id_map,
            PortBJunction,
            PortTag::B,
        );

        // println!("{:?}", net.topology);

        // get current sim timestep
        let dt = world.get::<&Time>(|t| t.sim_dt_f64());

        // solve flow net
        let (ode, solution, next) = FlowNetSolver::new().step(&mut net, net_pipe_state, dt);

        if world.get::<&FlowSimConfig>(|c| c.debug_print_ode_solution) {
            ode.print_overview(&next);
            // println!("solution: {:?}", solution);
        }

        // Phase 1: liquid flows from pipes vessels into the transionary junction vessels
        fn pipe_outflow_impl<R>(
            world: &World,
            pipe_entity_id_map: &HashMap<Entity, PipeId>,
            solution: &IntMap<PipeSolution>,
            rel: R,
            port: PortTag,
        ) where
            Access: FromAccessArg<R> + FromAccessArg<Junction>,
        {
            let topo_query = world
                .query::<(&mut FlowNetPipeVessel, &mut FlowNetReservoirVessel)>()
                .related(This, rel, "$junc")
                .tagged("$junc", Junction)
                .tagged("$junc", Arg(1))
                .build();

            topo_query.each_entity(|epipe, (pipe_vessel, junc_vessel)| {
                let pipe_id = pipe_entity_id_map.get(&epipe).unwrap();
                let delta_volume = solution[**pipe_id].delta_volume[port];
                if delta_volume < 0. {
                    for chunk in pipe_vessel.0.drain(port, -delta_volume) {
                        junc_vessel.0.fill(chunk);
                    }
                }
            });
        }
        pipe_outflow_impl(
            world,
            &pipe_entity_id_map,
            &solution,
            PortAJunction,
            PortTag::A,
        );
        pipe_outflow_impl(
            world,
            &pipe_entity_id_map,
            &solution,
            PortBJunction,
            PortTag::B,
        );

        // Phase 2: liquid flows from transionary junction vessels into pipe vessels
        fn pipe_inflow_impl<R>(
            world: &World,
            pipe_entity_id_map: &HashMap<Entity, PipeId>,
            solution: &IntMap<PipeSolution>,
            rel: R,
            port: PortTag,
        ) where
            Access: FromAccessArg<R> + FromAccessArg<Junction>,
        {
            let topo_query = world
                .query::<(&mut FlowNetPipeVessel, &mut FlowNetReservoirVessel)>()
                .related(This, rel, "$junc")
                .tagged("$junc", Junction)
                .tagged("$junc", Arg(1))
                .build();

            topo_query.each_entity(|epipe, (pipe_vessel, junc_vessel)| {
                let pipe_id = pipe_entity_id_map.get(&epipe).unwrap();
                let delta_volume = solution[**pipe_id].delta_volume[port];
                if delta_volume > 0. {
                    if let Some(chunk) = junc_vessel.0.drain(delta_volume) {
                        pipe_vessel.0.fill(port, chunk);
                    }
                }
            });
        }
        pipe_inflow_impl(
            world,
            &pipe_entity_id_map,
            &solution,
            PortAJunction,
            PortTag::A,
        );
        pipe_inflow_impl(
            world,
            &pipe_entity_id_map,
            &solution,
            PortBJunction,
            PortTag::B,
        );

        // write back state
        world
            .query::<(
                &FlowNetPipeDef,
                &mut FlowNetPipeState,
                &mut PipeFlowState,
                &mut PipeFlowStats,
            )>()
            .build()
            .each_entity(|epipe, (_def, fls, state, stats)| {
                let pipe_id = **pipe_entity_id_map.get(&epipe).unwrap();
                let scr = &ode.pipe_scratch()[pipe_id];

                fls.0 = next[pipe_id].clone();

                state.junction_pressure = PortMap::from_array(scr.junction_pressure);

                for i in [0, 1] {
                    // P = F/A = a / (A/m)
                    let pressure = scr.total_accel[i] / scr.area_per_mass;
                    state.pressure[i] = pressure;
                    stats.pressure_ema[i].step(dt, pressure);

                    let delta_volume = solution[pipe_id].delta_volume[i];
                    let flow = delta_volume / dt;
                    state.flow[i] = flow;
                    stats.flow_ema[i].step(dt, delta_volume);
                }
            });

        // Operate valves based on pressure differential
        world
            .query::<(
                &ValveDef,
                &mut ValveState,
                &mut FlowNetPipeDef,
                &mut PipeFlowState,
            )>()
            .build()
            .each(|(valve_def, valve_state, pipe_def, pipe_state)| {
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
                    pipe_def.0.port_area_factor[i] = if is_open { 1. } else { 0. };
                    // println!("VALVE: {}", is_open);
                }
            });

        // Write pipe data to CSV
        if let Some(path) = world.get::<&FlowSimConfig>(|c| c.pipe_stats_csv_path.clone()) {
            let step = world.get::<&Time>(|t| t.frame_count);
            let file_path = path.join(format!("flow_net_pipes_{step:05}.csv"));

            write_flow_net_pipes_csv(world, &file_path).ok();
        }

        // Write graph topology to CSV
        if let Some(path) = world.get::<&FlowSimConfig>(|c| c.graph_topology_path.clone()) {
            let step = world.get::<&Time>(|t| t.frame_count);
            let file_path = path.join(format!("topology_{step:05}.csv"));

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

fn write_flow_net_pipes_csv(world: &World, file_path: &Path) -> std::io::Result<()> {
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
        .query::<(&FlowNetPipeDef, &PipeFlowState, &FlowNetPipeVessel)>()
        .build()
        .each_entity(|entity, (def, state, vessel)| {
            let volume = vessel.0.volume();
            let length = def.0.shape.model.length;
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
            let open_a = def.0.port_area_factor[PortTag::A];
            let open_b = def.0.port_area_factor[PortTag::B];

            writeln!(
                writer,
                "{},{},{volume},{length},{pressure_a},{pressure_b},{junction_a},{junction_b},{flow_a},{flow_b},{open_a},{open_b}",
                *entity,entity.name()
            )
            .unwrap();
        });

    Ok(())
}

fn write_flow_net_topology_dot(world: &World, file_path: &Path) -> std::io::Result<()> {
    use std::{
        collections::HashSet,
        fs::File,
        io::{BufWriter, Write},
    };

    // Helper: stable Graphviz ID for an entity and a safe label.
    fn gv_id(e: EntityView) -> String {
        format!("e{}", e.id())
    }

    fn gv_label(e: EntityView) -> String {
        // Use the entity's name if present; otherwise fall back to its id.
        match Some(e.name()) {
            Some(n) if !n.is_empty() => n.replace('"', r#"\""#),
            _ => format!("id={}", e.id()),
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

    fn walk_rel<R>(
        world: &World,
        rel: R,
        w: &mut BufWriter<File>,
        seen_pipes: &mut HashSet<Entity>,
        seen_juncs: &mut HashSet<Entity>,
        seen_edges: &mut HashSet<(Entity, Entity)>,
    ) where
        Access: FromAccessArg<R>,
    {
        let topo = world
            .query::<(&mut FlowNetPipeVessel, &mut FlowNetReservoirVessel)>()
            .related(This, rel, "$junc")
            .tagged("$junc", Junction)
            .tagged("$junc", Arg(1))
            .build();

        let junc_var = topo.find_var("junc").expect("var $junc");

        topo.run(|mut it| {
            while it.next() {
                for i in it.iter() {
                    let pipe = it.entity(i).unwrap();
                    let junc = it.get_var(junc_var);

                    // Node declarations (once).
                    if seen_pipes.insert(pipe.id()) {
                        writeln!(
                            w,
                            "  {} [label=\"{}\", shape=box, style=rounded, penwidth=1.2];",
                            gv_id(pipe),
                            gv_label(pipe)
                        ).ok();
                    }
                    if seen_juncs.insert(junc.id()) {
                        writeln!(
                            w,
                            "  {} [label=\"\", shape=circle, width=0.15, fixedsize=true, style=filled, fillcolor=\"#666666\"];",
                            gv_id(junc)
                        ).ok();
                    }

                    // Edges (once across both PortA/PortB passes).
                    if seen_edges.insert((pipe.id(), junc.id())) {
                        writeln!(w, "  {} -- {};", gv_id(pipe), gv_id(junc)).ok();
                    }
                }
            }
        });
    }

    walk_rel(
        world,
        PortAJunction,
        &mut w,
        &mut seen_pipes,
        &mut seen_juncs,
        &mut seen_edges,
    );
    walk_rel(
        world,
        PortBJunction,
        &mut w,
        &mut seen_pipes,
        &mut seen_juncs,
        &mut seen_edges,
    );

    writeln!(w, "}}")?;
    Ok(())
}
