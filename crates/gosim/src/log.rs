use flecs_ecs::prelude::*;
use log::{Level, LevelFilter, Metadata, Record};
use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
};

/// Collects errors
#[derive(Component)]
pub struct LogModule;

struct WorldLogger {
    backend: Arc<RwLock<Backend>>,
}

impl log::Log for WorldLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            self.backend.write().unwrap().log(record);
        }
    }

    fn flush(&self) {}
}

struct Backend {
    max_entries: usize,
    entries: VecDeque<(Level, String)>,
}

impl Default for Backend {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            entries: VecDeque::new(),
        }
    }
}

impl Backend {
    pub fn log(&mut self, record: &Record) {
        let level = record.level();
        let message = format!("{}", record.args());

        self.entries.push_back((level, message));

        while self.entries.len() >= self.max_entries {
            self.entries.pop_front();
        }
    }

    pub fn print_to_console(&mut self) {
        for (level, message) in self.entries.drain(..) {
            println!("[{level}] {message}");
        }
    }
}

#[derive(Component)]
pub struct Log {
    backend: Arc<RwLock<Backend>>,
    enable_print_to_console: bool,
}

impl Log {
    pub fn print_to_console(&self) {
        self.backend.write().unwrap().print_to_console()
    }
}

impl Module for LogModule {
    fn module(world: &World) {
        world.module::<LogModule>("LogModule");

        world.component::<Log>();

        let backend = Arc::new(RwLock::new(Backend::default()));

        world.set(Log {
            backend: backend.clone(),
            enable_print_to_console: true,
        });

        if let Err(err) = log::set_boxed_logger(Box::new(WorldLogger { backend })) {
            eprintln!("{err}");
            panic!();
        };
        log::set_max_level(LevelFilter::Info);

        // Flush messages to console
        world.system::<()>().run(|it| {
            it.world().get::<&mut Log>(|log| {
                if log.enable_print_to_console {
                    log.print_to_console();
                }
            });
        });
    }
}
