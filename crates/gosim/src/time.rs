use flecs_ecs::prelude::*;
use mocca::Mocca;
use std::time::{Duration, Instant};

/// Measures time
#[derive(Default)]
pub struct TimeMocca;

#[derive(Component, Clone)]
pub struct Time {
    /// Current system time
    pub walltime: Instant,

    /// Total number of simulated frames since program start
    pub frame_count: u64,

    /// Simulation time (accumulated time of all simulated frames)
    pub sim_time: Duration,

    /// Simulation step time
    pub sim_dt: Duration,
}

impl Time {
    pub fn sim_dt_f64(&self) -> f64 {
        self.sim_dt.as_secs_f64()
    }

    pub fn sim_frame_to_sim_time_f64(&self, frame: u64) -> f64 {
        frame as f64 * self.sim_dt.as_secs_f64()
    }
}

impl Mocca for TimeMocca {
    fn register_components(world: &World) {
        world.component::<Time>();
    }

    fn start(world: &World) -> Self {
        world.set(Time {
            walltime: Instant::now(),
            frame_count: 0,
            sim_time: Duration::default(),
            sim_dt: Duration::from_millis(20),
        });

        Self
    }

    fn step(&mut self, world: &World) {
        // Progress time
        world.get::<&mut Time>(|time| {
            time.walltime = Instant::now();
            time.frame_count += 1;
            time.sim_time += time.sim_dt;
        });
    }
}
