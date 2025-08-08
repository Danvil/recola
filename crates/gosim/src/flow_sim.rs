use crate::{utils::FlecsQueryRelationHelpers, Arg, EntityBuilder, This, Time, TimeMocca};
use flecs_ecs::prelude::{World, *};
use flowsim::{
    models::{Bundle, ElasticTube, HoopTubePressureModel, PressureModel},
    FlowNet, FlowNetSolver, FluidChunk, FluidComposition, FluidDensityViscosity, JuncId, PipeDef,
    PipeId, PipeSolution, PipeState, PipeVessel, PortMap, PortTag, ReservoirVessel,
};
use gems::{volume_from_milli_liters, Ema, IntMap, Lerp, VolumeModel};
use mocca::{Mocca, MoccaDeps};
use std::{collections::HashMap, marker::PhantomData};

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

// /// Internal state used for computation of liquid flow
// #[derive(Component, Clone, Default, Debug)]
// pub struct PipeFlowState {
//     /// Current pressure model for the pipe
//     pressure_model: HoopTubePressureModel,

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

        let volume = match elasticity_pressure_model.volume(self.target_pressure, shape.volume()) {
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
            port_area_factor: [1., 1.],
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
            // .set(PipeFlowState::default())
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

impl Mocca for FlowSimMocca {
    fn load(mut dep: MoccaDeps) {
        dep.dep::<TimeMocca>();
    }

    fn register_components(world: &World) {
        world.component::<FlowNetPipeDef>();
        world.component::<FlowNetPipeState>();
        world.component::<FlowNetPipeVessel>();
        world.component::<FlowNetReservoirVessel>();
        // world.component::<PipeFlowState>();
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

    fn start(_: &World) -> Self {
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
        let (_ode, solution, next) = FlowNetSolver::new().step(&mut net, net_pipe_state, dt);
        // println!("ode: {:?}", ode);
        // ode.print_overview(&next);
        // println!("solution: {:?}", solution);

        // write back state
        world
            .query::<(&mut FlowNetPipeState,)>()
            .build()
            .each_entity(|epipe, (state,)| {
                let id = **pipe_entity_id_map.get(&epipe).unwrap();
                state.0 = next[id].clone();
            });

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
    }
}
