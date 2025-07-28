use crate::{stat_component, Arg, FlecsQueryRelationHelpers, This, Time, TimeModule};
use flecs_ecs::prelude::*;
use gems::{Lerp, RateEma};
use num_traits::Pow;
use std::collections::VecDeque;

/// Flow nets pump fluid through compliant pipes using a basic pressure model.
#[derive(Component)]
pub struct FlowNetModule;

#[derive(Component)]
pub struct FlowNetConfig {
    /// Fluid flow per second and unit of pressure difference
    pub flow_factor: f64,
}

/// Stats for an elastic fluid pipe
#[derive(Component, Clone, Debug)]
pub struct PipeStats {
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

impl PipeStats {
    /// Nominal volume [L] of liquid stored in the vessels
    pub fn nominal_volume(&self) -> f64 {
        disk_area(self.radius) * self.length * self.count * 1000.0
    }

    /// Computes radius based on given volume [L]
    pub fn volume_to_radius(&self, volume: f64) -> f64 {
        (volume / 1000. / (core::f64::consts::PI * self.length * self.count)).sqrt()
    }

    /// Compute pressure for given volume [L]
    pub fn pressure(&self, volume: f64) -> f64 {
        let r0 = self.radius;
        let r = self.volume_to_radius(volume);

        if r < r0 {
            // Tube Law; exponent computed such that tangent matches my law at r=r0
            let n = -2. * self.pressure_min * r0 / (self.youngs_modulus * self.wall_thickness);
            self.pressure_min * (1. - (r / r0).pow(2. / n))
        } else {
            // my law derived from stress equations for an elastic ring
            self.youngs_modulus * self.wall_thickness * (r0 / r) * ((r - r0) / (r * r))
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
}

fn disk_area(r: f64) -> f64 {
    r * r * core::f64::consts::PI
}

fn disk_circumfence(r: f64) -> f64 {
    2. * r * core::f64::consts::PI
}

#[derive(Clone)]
pub struct FluidChunk<T> {
    pub volume: f64,
    pub data: T,
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

    /// Volume of liquid stored in chunks
    pub fn volume(&self) -> f64 {
        self.volume
    }

    /// Volume-weighted average chunk data
    pub fn average_data(&self) -> Option<T>
    where
        T: Lerp<f64>,
    {
        T::weighted_average(self.chunks.iter().map(|c| (c.volume, &c.data)))
    }

    pub fn chunks(&self) -> impl ExactSizeIterator<Item = &FluidChunk<T>> {
        self.chunks.iter()
    }

    pub fn chunk_volume_data_mut(&mut self) -> impl Iterator<Item = (f64, &mut T)> {
        self.chunks.iter_mut().map(|c| (c.volume, &mut c.data))
    }

    /// Push blood into the vessel (one chunk)
    pub fn fill(&mut self, chunk: FluidChunk<T>) {
        assert!(chunk.volume >= 0.);
        if chunk.volume > 0. {
            self.volume += chunk.volume;

            if let Some(last) = self.chunks.back() {
                if last.volume < self.min_chunk_volume {
                    // last chunk too small - mix in the inflow
                    let volume = last.volume + chunk.volume;
                    let data = T::weighted_average([
                        (last.volume, &last.data),
                        (chunk.volume, &chunk.data),
                    ])
                    .expect("two chunks where given as input");
                    self.chunks.pop_back();
                    self.chunks.push_back(FluidChunk { volume, data });
                } else {
                    // start new chunk
                    self.chunks.push_back(chunk);
                }
            } else {
                // first chunk
                self.chunks.push_back(chunk);
            }
        }
    }

    pub fn filled(mut self, chunk: FluidChunk<T>) -> Self {
        self.fill(chunk);
        self
    }

    /// Pop blood from the vessel (by volume)
    pub fn drain(&mut self, volume: f64) -> impl Iterator<Item = FluidChunk<T>> + '_ {
        assert!(volume >= 0.);

        struct DrainIter<'a, T: Clone> {
            chunks: &'a mut VecDeque<FluidChunk<T>>,
            remaining: f64,
            volume_ref: &'a mut f64,
        }

        impl<'a, T: Clone> Iterator for DrainIter<'a, T> {
            type Item = FluidChunk<T>;

            fn next(&mut self) -> Option<Self::Item> {
                if self.remaining <= 0. {
                    return None;
                }

                let next = self.chunks.pop_front()?;
                if next.volume > self.remaining {
                    let mut remainder = next.clone();
                    remainder.volume -= self.remaining;
                    self.chunks.push_front(remainder);

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
            chunks: &mut self.chunks,
            remaining: volume,
            volume_ref: &mut self.volume,
        }
    }

    pub fn drain_into(&mut self, dst: &mut Self, volume: f64) -> f64 {
        let mut total = 0.;

        // let vsrc1 = self.chunks.front().unwrap().volume;
        // let nsrc1 = self.chunks().len();
        // let ndst1 = dst.chunks().len();

        for c in self.drain(volume) {
            total += c.volume;
            dst.fill(c);
        }

        // let vsrc2 = self.chunks.front().unwrap().volume;
        // let nsrc2 = self.chunks().len();
        // let ndst2 = dst.chunks().len();

        // println!(
        //     "{}/{} => {}/{}, {} => {}",
        //     nsrc1, ndst1, nsrc2, ndst2, vsrc1, vsrc2
        // );

        total
    }

    /// Liquid flows from A to B if volume is positive and from B to A if negative.
    pub fn flow(a: &mut Self, b: &mut Self, volume: f64) -> f64 {
        if volume >= 0. {
            a.drain_into(b, volume)
        } else {
            b.drain_into(a, -volume)
        }
    }
}

/// Internal state used for computation of liquid flow
#[derive(Component, Clone, Default)]
pub struct PipeFlowState {
    pressure: f64,
    flow: RateEma,
    step_in: f64,
    step_out: f64,
}

impl PipeFlowState {
    pub fn pressure(&self) -> f64 {
        self.pressure
    }

    pub fn flow(&self) -> f64 {
        self.flow.value()
    }
}

/// Indicates that two pipes are connected to each other and can exchange fluid.
#[derive(Component)]
pub struct FluidFlowLink;

/// Indicates that a flow link is currently closed
#[derive(Component)]
pub struct IsLinkClosed;

/// Pipe from which a pump takes fluid.
#[derive(Component)]
pub struct PumpIntakePipe;

/// Pipe into which a pump deposits fluid.
#[derive(Component)]
pub struct PumpOutputPipe;

stat_component!(
    /// Amount of liquid pumped per tick
    PumpVolume
);

/// Tag used to indicate that a pump is active. Useful for periodic pumps.
#[derive(Component)]
pub struct IsPumpActive;

/// Pump statistics
#[derive(Component, Default, Clone)]
pub struct PumpStats {
    /// Measured liquid flow moved by the pumpt
    pub flow: RateEma,

    pub delta: f64,
}

pub fn setup_flow_net<T: 'static + Send + Sync + Clone + Lerp<f64>>(world: &World) {
    world.component::<Pipe<T>>();

    // Active pumps force liquid from intake to output
    world
        .system::<(&PumpVolume, &mut Pipe<T>, &mut Pipe<T>, &mut PumpStats)>()
        .with(IsPumpActive)
        .related("$chamber", flecs::ChildOf, "$this")
        .related("$chamber", PumpIntakePipe, "$in")
        .related("$chamber", PumpOutputPipe, "$out")
        .tagged("$in", Arg(1))
        .tagged("$out", Arg(2))
        .each(|(volume, intake, output, stats)| {
            stats.delta = intake.drain_into(output, **volume);
        });

    // Update flow estimation for all pumps
    world
        .system::<(&Time, &mut PumpStats)>()
        .singleton_at(0)
        .each(|(t, stats)| {
            stats.flow.step(t.sim_dt_f64(), stats.delta);
            stats.delta = 0.;
        });

    // Prepare flow state
    world
        .system::<(&PipeStats, &Pipe<T>, &mut PipeFlowState)>()
        .each(|(stats, vessel, state)| {
            state.pressure = stats.pressure(vessel.volume());
            state.step_in = 0.;
            state.step_out = 0.;
        });

    // Compute flow volume through pipes based on pressure gradient
    world
        .system::<(
            &Time,
            &FlowNetConfig,
            &mut Pipe<T>,
            &mut Pipe<T>,
            &mut PipeFlowState,
            &mut PipeFlowState,
        )>()
        .singleton_at(0)
        .singleton_at(1)
        .related(This, FluidFlowLink, "$dst")
        .unrelated(This, IsLinkClosed, "$dst")
        .tagged("$dst", Arg(3))
        .tagged("$dst", Arg(5))
        .each(|(t, cfg, src_vessel, dst_vessel, src_state, dst_state)| {
            let req = t.sim_dt_f64() * cfg.flow_factor * (src_state.pressure - dst_state.pressure);

            // Limit request to 50% of available volume (remember this is per tick!)
            let delta = req.min(0.50 * src_vessel.volume());

            // This gets called twice for each edge, a-b and b-a, thus only act on positive
            // flow.
            if delta > 0. {
                let actual = src_vessel.drain_into(dst_vessel, delta);

                src_state.step_out += actual;
                dst_state.step_in += actual;
            }
        });

    // Update pressure based on new volume
    world
        .system::<(&PipeStats, &Pipe<T>, &mut PipeFlowState)>()
        .each(|(stats, vessel, state)| {
            state.pressure = stats.pressure(vessel.volume());
        });

    // Flow estimation for pipes
    world
        .system::<(&Time, &mut PipeFlowState)>()
        .singleton_at(0)
        .each(|(t, state)| {
            state
                .flow
                .step(t.sim_dt.as_secs_f64(), state.step_in.min(state.step_out));
        });
}

impl Module for FlowNetModule {
    fn module(world: &World) {
        world.module::<FlowNetModule>("FlowNetModule");

        world.import::<TimeModule>();

        world.component::<FlowNetConfig>();
        world.component::<PipeFlowState>();
        world.component::<PipeStats>();

        world
            .component::<FluidFlowLink>()
            .add_trait::<flecs::Symmetric>();

        world.component::<IsLinkClosed>();

        world
            .component::<PumpIntakePipe>()
            .add_trait::<flecs::Exclusive>();
        world
            .component::<PumpOutputPipe>()
            .add_trait::<flecs::Exclusive>();

        PumpVolume::setup(world);

        world.component::<IsPumpActive>();
        world.component::<PumpStats>();

        world.set(FlowNetConfig {
            flow_factor: 0.00002,
        });
    }
}
