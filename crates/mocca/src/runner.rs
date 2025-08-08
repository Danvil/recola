use crate::{DepTree, Mocca};
use flecs_ecs::prelude::*;
use gems::Ema;
use std::{
    any::TypeId,
    sync::mpsc,
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct MoccaRunner {
    world: World,
    deps: DepTree,
    moccas: Vec<Entry>,
    order: Vec<usize>,
}

struct Entry {
    id: MoccaId,
    type_name: String,
    moc: Box<dyn MoccaDyn>,
    step_duration: Ema,
}

impl Entry {
    pub fn step(&mut self, world: &World) {
        let time = Instant::now();
        self.moc.step(world);
        self.step_duration
            .step(0.050, (Instant::now() - time).as_secs_f64());
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct MoccaId(TypeId);

impl MoccaId {
    pub fn new<M: Mocca>() -> Self {
        Self(TypeId::of::<M>())
    }
}

impl MoccaRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enable_flecs_stats_server(&self) {
        // Optional, gather statistics for explorer
        self.world.import::<stats::Stats>();

        // Creates REST server on default port (27750)
        self.world.set(flecs::rest::Rest::default());
    }

    pub fn load<MT: MoccaTuple>(&mut self) {
        MT::load(self)
    }

    fn load_impl<M: Mocca>(&mut self) -> MoccaId {
        let id = MoccaId::new::<M>();

        if !self.deps.contains_node(id) {
            let type_name = std::any::type_name::<M>().to_string();
            self.deps.add_node(id, type_name.clone());

            M::load(MoccaDeps(id, self));
            M::register_components(&self.world);
            let m = M::start(&self.world);

            self.moccas.push(Entry {
                id,
                type_name,
                moc: Box::new(m),
                step_duration: Ema::default(),
            });
        }

        id
    }

    pub fn start(&mut self) {
        println!("\nMOCCA DEP TREE");
        self.deps.print_tree();

        self.order.clear();
        for id in self.deps.order_after_deps().unwrap() {
            let idx = self.moccas.iter().position(|e| e.id == id).unwrap();
            self.order.push(idx);
        }
    }

    pub fn step(&mut self) {
        for &idx in self.order.iter() {
            self.moccas[idx].step(&self.world);
        }
    }

    pub fn fini(&mut self) {
        for &idx in self.order.iter().rev() {
            self.moccas[idx].moc.fini(&self.world);
        }
    }

    pub fn fini_test(&mut self) {
        for &idx in self.order.iter().rev() {
            self.moccas[idx].moc.fini_test(&self.world);
        }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn print_report(&self) {
        let mut rows: Vec<(f64, &str)> = self
            .moccas
            .iter()
            .map(|entry| {
                // Convert seconds to milliseconds
                let duration_ms = entry.step_duration.value() * 1000.0;
                (duration_ms, entry.type_name.as_str())
            })
            .collect();

        // Sort by duration descending
        rows.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Determine column widths
        let type_col_width = rows.iter().map(|(_, t)| t.len()).max().unwrap_or(9);

        // Header
        println!("\nMOCCA RUNNER REPORT");
        println!(
            "{:>15} | {:<width$}",
            "Duration [ms]",
            "Type Name",
            width = type_col_width
        );
        println!("{}", "-".repeat(18 + 3 + type_col_width));

        // Rows
        for (duration_ms, type_name) in rows {
            println!(
                "{:>15.3} | {:<width$}",
                duration_ms,
                type_name,
                width = type_col_width
            );
        }
    }

    pub fn run<MT: MoccaTuple>(settings: MoccaRunSettings) {
        let (ctrl_tx, ctrl_rx) = mpsc::channel();

        let mut runner = Self::new();

        if settings.stop_on_ctrl_c {
            ctrlc::set_handler(move || {
                println!("received ctrl+c - terminating");
                ctrl_tx.send(()).ok();
            })
            .expect("Error setting Ctrl-C handler");
        }

        if settings.enable_flecs_stats_server {
            runner.enable_flecs_stats_server()
        }

        runner.load::<MT>();

        runner.start();

        let mut iteration = 0;

        loop {
            iteration += 1;

            runner.step();

            if ctrl_rx.try_recv().is_ok() {
                break;
            }

            if let Some(max_count) = settings.step_limit {
                if iteration >= max_count {
                    break;
                }
            }

            if let Some(throttle) = settings.throttle {
                std::thread::sleep(throttle);
            }
        }

        runner.fini();

        runner.print_report();

        if settings.enable_test {
            runner.fini_test();
        }
    }
}

pub struct MoccaRunSettings {
    pub step_limit: Option<usize>,
    pub stop_on_ctrl_c: bool,
    pub enable_test: bool,
    pub enable_flecs_stats_server: bool,
    pub throttle: Option<Duration>,
}

impl MoccaRunSettings {
    pub fn app() -> Self {
        Self {
            step_limit: None,
            stop_on_ctrl_c: true,
            enable_test: false,
            enable_flecs_stats_server: true,
            throttle: Some(Duration::from_millis(50)),
        }
    }

    pub fn test(step_limit: usize) -> Self {
        Self {
            step_limit: Some(step_limit),
            stop_on_ctrl_c: false,
            enable_test: true,
            enable_flecs_stats_server: false,
            throttle: None,
        }
    }
}

pub struct MoccaDeps<'a>(MoccaId, &'a mut MoccaRunner);

impl MoccaDeps<'_> {
    pub fn dep<M: Mocca + 'static>(&mut self) {
        self.1.load::<M>();
        self.1.deps.add_dep(self.0, MoccaId::new::<M>());
    }
}

trait MoccaDyn {
    fn step(&mut self, world: &World);
    fn fini(&mut self, world: &World);
    fn fini_test(&mut self, world: &World);
}

impl<T: Mocca> MoccaDyn for T {
    fn step(&mut self, world: &World) {
        Mocca::step(self, world);
    }

    fn fini(&mut self, world: &World) {
        Mocca::fini(self, world);
    }

    fn fini_test(&mut self, world: &World) {
        Mocca::fini_test(self, world);
    }
}

pub trait MoccaTuple {
    fn load(runner: &mut MoccaRunner);
}

impl<M: Mocca> MoccaTuple for M {
    fn load(runner: &mut MoccaRunner) {
        runner.load_impl::<M>();
    }
}

impl<M1: Mocca, M2: Mocca> MoccaTuple for (M1, M2) {
    fn load(runner: &mut MoccaRunner) {
        runner.load_impl::<M1>();
        runner.load_impl::<M2>();
    }
}

impl<M1: Mocca, M2: Mocca, M3: Mocca> MoccaTuple for (M1, M2, M3) {
    fn load(runner: &mut MoccaRunner) {
        runner.load_impl::<M1>();
        runner.load_impl::<M2>();
        runner.load_impl::<M3>();
    }
}
