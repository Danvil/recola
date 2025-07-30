use crate::{
    elastic_tube_inv_approx, elastic_tube_pressure, tube_law_pressure, Arg, EntityBuilder,
    FlecsQueryRelationHelpers, This, Time, TimeModule,
};
use flecs_ecs::prelude::*;
use gems::{Ema, Lerp};
use std::{collections::VecDeque, marker::PhantomData};

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
    /// Radius of vessels in meters
    pub radius: f64,

    /// Total length of vessels [meter]
    pub length: f64,

    /// Pipe wall thickness [meter]
    pub wall_thickness: f64,

    /// Young's modulus describing elasticity of the pipe wall
    pub youngs_modulus: f64,

    /// Number of vessels. This is a multiplier which serves to increase the volume without
    /// increasing vessel length or radius.
    pub count: f64,

    /// Minimal pressure for tube law used when radius is smaller than nominal
    pub pressure_min: f64,
}

impl PipeGeometry {
    /// Nominal volume [L] of liquid stored in the vessels
    pub fn nominal_volume(&self) -> f64 {
        self.radius_to_volume(self.radius)
    }

    /// Computes radius [m] based on given volume [L]
    pub fn volume_to_radius(&self, volume: f64) -> f64 {
        (volume / 1000. / (core::f64::consts::PI * self.length * self.count)).sqrt()
    }

    /// Computes volume [L] based on given radius [m]
    pub fn radius_to_volume(&self, radius: f64) -> f64 {
        disk_area(radius) * self.length * self.count * 1000.0
    }

    /// Computes volume [L] needed to achieve target pressure [Pa]
    /// Warning: This is only an approximation. If volume grows too large pressure will actually
    /// fall again due to wall thinning.
    pub fn pressure_to_volume(&self, pressure: f64) -> f64 {
        let radius = elastic_tube_inv_approx(
            pressure,
            self.radius,
            self.wall_thickness,
            self.youngs_modulus,
        );

        self.radius_to_volume(radius)
    }

    /// Compute pressure for given volume [L]
    pub fn pressure(&self, volume: f64) -> f64 {
        let r = self.volume_to_radius(volume);

        if r < self.radius {
            tube_law_pressure(
                r,
                self.radius,
                self.wall_thickness,
                self.youngs_modulus,
                self.pressure_min,
            )
        } else {
            elastic_tube_pressure(r, self.radius, self.wall_thickness, self.youngs_modulus)
        }
    }

    /// Total surface area of vessels
    pub fn total_surface_area(&self) -> f64 {
        disk_circumfence(self.radius) * self.length * self.count
    }

    /// Compute vessel count for given total volume [L]
    pub fn volume_to_count(&self, volume: f64) -> f64 {
        volume / 1000. / (disk_area(self.radius) * self.length)
    }

    pub fn conductance(&self) -> f64 {
        // TODO use correct viscosity
        self.count * core::f64::consts::PI * self.radius.powi(4)
            / (8. * self.length * VISCOSITY_WATER)
    }
}

const VISCOSITY_WATER: f64 = 0.0010016;

fn disk_area(r: f64) -> f64 {
    r * r * core::f64::consts::PI
}

fn disk_circumfence(r: f64) -> f64 {
    2. * r * core::f64::consts::PI
}

/// A vessel stores a single chunk of fluid. Inflow mixes perfectly.
#[derive(Component, Clone)]
pub struct Vessel<T: 'static + Send + Sync + Clone> {
    chunk: Option<FluidChunk<T>>,
}

impl<T: 'static + Send + Sync + Clone> Default for Vessel<T> {
    fn default() -> Self {
        Self { chunk: None }
    }
}

impl<T: 'static + Send + Sync + Clone> Vessel<T>
where
    T: Lerp<f64>,
{
    /// Return true if the vessel does not contain any liquid
    pub fn is_empty(&self) -> bool {
        self.chunk.is_none()
    }

    /// Volume of liquid stored in the vessel
    pub fn volume(&self) -> f64 {
        self.chunk.as_ref().map_or(0., |c| c.volume)
    }

    /// Volume-weighted average chunk data
    pub fn average_data(&self) -> Option<&T> {
        self.chunk.as_ref().map(|c| &c.data)
    }

    /// Mix liquid into the vessel
    pub fn fill(&mut self, incoming: FluidChunk<T>) {
        assert!(incoming.volume >= 0.);

        self.chunk = Some(match self.chunk.as_ref() {
            Some(current) => FluidChunk {
                volume: current.volume + incoming.volume,
                data: T::weighted_average([
                    (current.volume, &current.data),
                    (incoming.volume, &incoming.data),
                ]),
            },
            None => incoming,
        });
    }

    pub fn drain(&mut self, volume: f64) -> Option<FluidChunk<T>> {
        assert!(volume >= 0.);
        if volume == 0. {
            return None;
        }

        let Some(current) = self.chunk.as_mut() else {
            return None;
        };

        if volume >= current.volume {
            return self.chunk.take();
        }

        let mut out = current.clone();
        out.volume = volume;
        current.volume -= volume;

        Some(out)
    }

    pub fn drain_all(&mut self) -> Option<FluidChunk<T>> {
        self.chunk.take()
    }
}

/// A pipe stores fluid "chunks" as a FIFO list. Pipes can be connected to exchange liquid.
#[derive(Component, Clone)]
pub struct Pipe<T: 'static + Send + Sync + Clone> {
    /// Fluid chunks currently contained by the vessels
    chunks: VecDeque<FluidChunk<T>>,

    /// Total volume of all chunks
    volume: f64,

    /// Chunks smaller than this will be merged.
    min_chunk_volume: f64,
}

impl<T: 'static + Send + Sync + Clone> Pipe<T>
where
    T: Lerp<f64>,
{
    pub fn new() -> Self {
        Self {
            chunks: VecDeque::new(),
            volume: 0.,
            min_chunk_volume: 0.,
        }
    }

    pub fn set_min_chunk_volume(&mut self, min_chunk_volume: f64) {
        self.min_chunk_volume = min_chunk_volume;
    }

    pub fn with_min_chunk_volume(mut self, min_chunk_volume: f64) -> Self {
        self.set_min_chunk_volume(min_chunk_volume);
        self
    }

    /// Volume of liquid stored in the pipe
    pub fn volume(&self) -> f64 {
        self.volume
    }

    /// Volume-weighted average chunk data
    pub fn average_data(&self) -> Option<T>
    where
        T: Lerp<f64>,
    {
        if self.volume == 0. {
            None
        } else {
            Some(T::weighted_average(
                self.chunks.iter().map(|c| (c.volume, &c.data)),
            ))
        }
    }

    pub fn chunks(&self) -> impl ExactSizeIterator<Item = &FluidChunk<T>> {
        self.chunks.iter()
    }

    pub fn chunk_volume_data_mut(&mut self) -> impl Iterator<Item = (f64, &mut T)> {
        self.chunks.iter_mut().map(|c| (c.volume, &mut c.data))
    }

    /// Push liquid into the pipe at given port
    pub fn fill(&mut self, port: PortTag, chunk: FluidChunk<T>) {
        assert!(chunk.volume >= 0.);
        if chunk.volume == 0. {
            return;
        }

        self.volume += chunk.volume;

        let mut port = PortOp(port, &mut self.chunks);

        let chunk = if let Some(last) = port.get() {
            if last.volume < self.min_chunk_volume {
                // last chunk too small - mix in the inflow
                let volume = last.volume + chunk.volume;
                let data =
                    T::weighted_average([(last.volume, &last.data), (chunk.volume, &chunk.data)]);
                port.pop();

                FluidChunk { volume, data }
            } else {
                // start new chunk
                chunk
            }
        } else {
            // first chunk
            chunk
        };

        port.push(chunk);
    }

    pub fn filled(mut self, port: PortTag, chunk: FluidChunk<T>) -> Self {
        self.fill(port, chunk);
        self
    }

    /// Drain fluid from the pipe at given port.
    pub fn drain(
        &mut self,
        port: PortTag,
        volume: f64,
    ) -> impl Iterator<Item = FluidChunk<T>> + '_ {
        assert!(volume >= 0.);

        struct DrainIter<'a, T: Clone> {
            port: PortOp<'a, FluidChunk<T>>,
            remaining: f64,
            volume_ref: &'a mut f64,
        }

        impl<'a, T: Clone> Iterator for DrainIter<'a, T> {
            type Item = FluidChunk<T>;

            fn next(&mut self) -> Option<Self::Item> {
                if self.remaining <= 0. {
                    return None;
                }

                let next = self.port.pop()?;
                if next.volume > self.remaining {
                    let mut remainder = next.clone();
                    remainder.volume -= self.remaining;
                    self.port.push(remainder);

                    let mut taken = next;
                    taken.volume = self.remaining;
                    *self.volume_ref -= self.remaining;
                    self.remaining = 0.;
                    Some(taken)
                } else {
                    self.remaining -= next.volume;
                    *self.volume_ref -= next.volume;
                    Some(next)
                }
            }
        }

        DrainIter {
            port: PortOp(port, &mut self.chunks),
            remaining: volume,
            volume_ref: &mut self.volume,
        }
    }
}

/// Helper type to work on the ports of a pipe
struct PortOp<'a, T>(PortTag, &'a mut VecDeque<T>);

impl<'a, T> PortOp<'a, T> {
    /// Current chunk at port
    fn get(&self) -> Option<&T> {
        match self.0 {
            PortTag::A => self.1.front(),
            PortTag::B => self.1.back(),
        }
    }

    /// Pop chunk from port
    fn pop(&mut self) -> Option<T> {
        match self.0 {
            PortTag::A => self.1.pop_front(),
            PortTag::B => self.1.pop_back(),
        }
    }

    /// Push chunk into port
    fn push(&mut self, chunk: T) {
        match self.0 {
            PortTag::A => self.1.push_front(chunk),
            PortTag::B => self.1.push_back(chunk),
        }
    }
}

#[derive(Clone)]
pub struct FluidChunk<T> {
    pub volume: f64,
    pub data: T,
}

/// Internal state used for computation of liquid flow
#[derive(Component, Clone, Default)]
pub struct PipeFlowState {
    /// Current conductance of the pipe for flow through ports
    conductance: [f64; 2],

    /// Pipe pressure for each port
    pipe_pressure: [f64; 2],

    /// Junction pressure for each port
    junction_pressure: [f64; 2],

    /// Flow into the pipe through port A and B
    flow: [f64; 2],
}

impl PipeFlowState {
    pub fn intrinsic_port_pressure(&self, port: PortTag) -> f64 {
        self.pipe_pressure[port.index()]
    }

    /// Pressure differential over the pipe
    pub fn intrinsic_pressure_differential(&self, direction: FlowDirection) -> f64 {
        let [i1, i2] = direction.indices();
        self.pipe_pressure[i1] - self.pipe_pressure[i2]
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

/// Statistics for pipe
#[derive(Component, Clone, Default)]
pub struct PipeFlowStats {
    /// EMA of pressure at ports
    pipe_pressure_ema: [Ema; 2],

    /// EMA of flow through ports
    flow_ema: [Ema; 2],
}

impl PipeFlowStats {
    /// Pressure acting on the pipe wall (not on the ports)
    pub fn pipe_pressure_ema(&self, port: PortTag) -> f64 {
        self.pipe_pressure_ema[port.index()].value()
    }

    /// Pressure differential over the pipe
    pub fn pressure_differential_ema(&self, direction: FlowDirection) -> f64 {
        let [i1, i2] = direction.indices();
        self.pipe_pressure_ema[i1].value() - self.pipe_pressure_ema[i2].value()
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
}

#[derive(Clone, Default)]
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
    total_conductance: f64,
    total_weighted_pressure: f64,
    equalized_pressure: f64,
}

impl JunctionState {
    pub fn reset(&mut self) {
        self.total_conductance = 0.;
        self.total_weighted_pressure = 0.;
        self.equalized_pressure = 0.;
    }

    pub fn push(&mut self, conductance: f64, pressure: f64) {
        self.total_conductance += conductance;
        self.total_weighted_pressure += conductance * pressure;
    }

    pub fn finalize(&mut self) {
        if self.total_conductance > 0. {
            self.equalized_pressure = self.total_weighted_pressure / self.total_conductance;
        }
    }

    /// Flow from junction into pipe based on pipe conductance
    pub fn outflow(&mut self, conductance: f64, pressure: f64) -> f64 {
        conductance * (self.equalized_pressure - pressure)
    }
}

/// A pipe has two ports
#[derive(Clone, Copy, Debug)]
pub enum PortTag {
    A,
    B,
}

impl PortTag {
    pub fn index(&self) -> usize {
        match self {
            PortTag::A => 0,
            PortTag::B => 1,
        }
    }

    pub fn opposite(&self) -> PortTag {
        match self {
            PortTag::A => PortTag::B,
            PortTag::B => PortTag::A,
        }
    }

    pub fn tag(&self) -> &'static str {
        match self {
            PortTag::A => "A",
            PortTag::B => "B",
        }
    }
}

/// Indicates the junction to which port A of a pipe is connected. There can only be one junction
/// per port.
#[derive(Component)]
pub struct PortAJunction;

/// Indicates the junction to which port B of a pipe is connected.
#[derive(Component)]
pub struct PortBJunction;

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
            let intrinsic_pressure = pipe_def.pressure(pipe.volume());
            state.pipe_pressure[0] = intrinsic_pressure;
            state.pipe_pressure[1] = intrinsic_pressure;

            let conductance = pipe_def.conductance();
            state.conductance[0] = conductance;
            state.conductance[1] = conductance;
        });

    // Apply pipe external pressure
    world
        .system_named::<(&ExternalPipePressure, &mut PipeFlowState)>("PipeExternalPressure")
        .each(|(ext, state)| {
            state.pipe_pressure[0] += ext.0;
            state.pipe_pressure[1] += ext.0;
        });

    // Pressure from pumps
    world
        .system_named::<(&PumpState, &mut PipeFlowState)>("PumpPressure")
        .each(|(pump_state, pipe_state)| {
            pipe_state.pipe_pressure[0] += pump_state.dp[0];
            pipe_state.pipe_pressure[1] += pump_state.dp[1];
        });

    // Compute junction pressure based on nominal pipe conductance

    fn junction_pressure(world: &World, tag: &str) {
        world
            .system_named::<(&mut JunctionState,)>(&format!("ResetJunctionState{tag}"))
            .each(|(junc_state,)| junc_state.reset());

        fn junction_pressure_impl<R>(world: &World, rel: R, port: PortTag, name: &str)
        where
            Access: FromAccessArg<R>,
        {
            world
                .system_named::<(&PipeFlowState, &mut JunctionState)>(name)
                .related("$pipe", rel, This)
                .tagged("$pipe", Arg(0))
                .each(move |(pipe_state, junc_state)| {
                    let pix = port.index();
                    junc_state.push(pipe_state.conductance[pix], pipe_state.pipe_pressure[pix]);
                });
        }

        junction_pressure_impl(
            world,
            PortAJunction,
            PortTag::A,
            &format!("JunctionStatePortA{tag}"),
        );

        junction_pressure_impl(
            world,
            PortBJunction,
            PortTag::B,
            &format!("JunctionStatePortB{tag}"),
        );

        world
            .system_named::<(&mut JunctionState,)>(&format!("FinalizeJunctionState{tag}"))
            .each(|(junc_state,)| junc_state.finalize());
    }

    junction_pressure(world, "-1");

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
                    pipe_state.junction_pressure[pix] = junc_state.equalized_pressure;
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
    store_junction_pressure(world, "-1");

    // Operate valves
    world
        .system_named::<(&ValveDef, &mut PipeFlowState, &mut ValveState)>("OperateValves")
        .each(|(valve_def, pipe_state, valve_state)| {
            let factor = valve_def.conductance_factor_closed.max(0.);

            valve_state.is_open = match valve_def.kind {
                ValveKind::Closed => [true, true],
                ValveKind::Open => [false, false],
                ValveKind::Throughflow(FlowDirection::AtoB) => [
                    pipe_state.pipe_pressure[0] < pipe_state.junction_pressure[0],
                    pipe_state.pipe_pressure[1] > pipe_state.junction_pressure[1],
                ],
                ValveKind::Throughflow(FlowDirection::BtoA) => [
                    pipe_state.pipe_pressure[0] > pipe_state.junction_pressure[0],
                    pipe_state.pipe_pressure[1] < pipe_state.junction_pressure[1],
                ],
                ValveKind::Inflow => [
                    pipe_state.pipe_pressure[0] < pipe_state.junction_pressure[0],
                    pipe_state.pipe_pressure[1] < pipe_state.junction_pressure[1],
                ],
                ValveKind::Outflow => [
                    pipe_state.pipe_pressure[0] > pipe_state.junction_pressure[0],
                    pipe_state.pipe_pressure[1] > pipe_state.junction_pressure[1],
                ],
            };

            if !valve_state.is_open[0] {
                pipe_state.conductance[0] *= factor;
            }

            if !valve_state.is_open[1] {
                pipe_state.conductance[1] *= factor;
            }
        });

    // Compute junction pressure based on modified pipe conductance
    junction_pressure(world, "-2");
    store_junction_pressure(world, "-2");

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
            .with(Junction)
            .related("$pipe", rel, This)
            .tagged("$pipe", Arg(3))
            .tagged("$pipe", Arg(4))
            .each(move |(time, junc, junc_state, pipe, pipe_state)| {
                let dt = time.sim_dt_f64();

                // Compute flow based on pressure differential.
                // Note that conductance is doubled as liquid exchange between pipe and junction
                // (not throughflow!) does on average has to travel only half the pipe length.
                let flow = junc_state.outflow(
                    2.0 * pipe_state.conductance[pix],
                    pipe_state.pipe_pressure[pix],
                );
                pipe_state.flow[pix] = flow;

                // Phase 1: move liquid from pipes into junction buffer
                if flow < 0. {
                    let volume = -dt * flow;
                    for chunk in pipe.drain(port, volume) {
                        junc.fill(chunk);
                    }
                }
            });
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
        .system_named::<(&mut Vessel<T>,)>("ClearJunctionBuffers")
        .with(Junction)
        .each(|(junc,)| {
            let chunk = junc.drain_all();
            let leftover = chunk.map_or(0., |c| c.volume);
            assert!(leftover <= 1e-6);
        });

    // Flow estimation for pipes
    world
        .system_named::<(&Time, &PipeFlowState, &mut PipeFlowStats)>("PipeStatistics")
        .singleton_at(0)
        .each(|(t, state, stats)| {
            let dt = t.sim_dt.as_secs_f64();
            stats.pipe_pressure_ema[0].step(dt, state.pipe_pressure[0]);
            stats.pipe_pressure_ema[1].step(dt, state.pipe_pressure[1]);
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
        let volume = self.geometry.pressure_to_volume(self.target_pressure);

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
                    .with_min_chunk_volume(0.05),
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
