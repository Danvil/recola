use flecs_ecs::prelude::*;
use gosim::{FlowSimConfig, LogMocca};
use gossim_apps::apps::FlowSimValveMocca;
use mocca::{MoccaRunSettings, MoccaRunner};

fn main() {
    MoccaRunner::run::<(LogMocca, FlowSimValveMocca)>(
        MoccaRunSettings::app()
            .with_step_limit(100)
            .with_preamble(|w| {
                // enable flow sim logging
                w.get::<&mut FlowSimConfig>(|cfg| {
                    cfg.pipe_stats_csv_path = Some("I:/Ikabur/gos/tmp/flow_sim/".into());
                })
            }),
    );
}
