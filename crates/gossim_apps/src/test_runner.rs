use flecs_ecs::prelude::*;
use gosim::LogModule;

const ENABLE_DEBUG_INFO: bool = false;

pub trait TestRunner: Sized {
    type Config;

    fn init(cfg: Self::Config, world: &World) -> Self;

    fn step(&mut self, _world: &World) {}

    fn debug_info(&mut self, _world: &World) {}

    fn fini_test(&mut self, _world: &World) {}

    fn fini(self, _world: &World) {}

    fn run_interactive(cfg: Self::Config) {
        let world = World::new();

        world.import::<stats::Stats>();
        world.set(flecs::rest::Rest::default());

        let mut x = Self::init(cfg, &world);

        loop {
            x.step(&world);

            let running = world.progress();

            if !running {
                break;
            }
        }

        x.fini(&world);
    }

    fn run_test(cfg: Self::Config, iterations: usize) {
        let world = World::new();
        world.import::<LogModule>();

        let mut x = Self::init(cfg, &world);

        println!("Initial:");
        x.debug_info(&world);

        for i in 0..iterations {
            if ENABLE_DEBUG_INFO {
                println!("Iteration {i}:");
            }

            x.step(&world);
            world.progress();

            if ENABLE_DEBUG_INFO {
                x.debug_info(&world);
            }
        }

        println!("Final:");
        x.debug_info(&world);

        x.fini_test(&world);

        x.fini(&world);
    }

    fn run_example(cfg: Self::Config, iterations: usize) {
        let world = World::new();
        world.import::<LogModule>();

        let mut x = Self::init(cfg, &world);

        println!("Initial:");
        x.debug_info(&world);

        for i in 0..iterations {
            println!("Iteration {i}:");

            x.step(&world);
            world.progress();

            x.debug_info(&world);
        }

        println!("Final:");
        x.debug_info(&world);

        x.fini(&world);
    }
}
