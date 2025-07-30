use flecs_ecs::prelude::*;

pub trait TestRunner: Sized {
    type Config;

    fn init(cfg: Self::Config, world: &World) -> Self;

    fn step(&mut self, _world: &World) {}

    fn fini_test(&mut self, _world: &World) {}

    fn fini(self, _world: &World) {}

    fn run_interactive(cfg: Self::Config) {
        let world = World::new();

        world.import::<stats::Stats>();
        world.set(flecs::rest::Rest::default());

        let mut x = Self::init(cfg, &world);

        loop {
            x.step(&world);
            if !world.progress() {
                break;
            }
        }

        x.fini(&world);
    }

    fn run_test(cfg: Self::Config, iterations: usize) {
        let world = World::new();

        let mut x = Self::init(cfg, &world);

        for _i in 0..iterations {
            x.step(&world);
            world.progress();
        }

        x.fini_test(&world);

        x.fini(&world);
    }
}
