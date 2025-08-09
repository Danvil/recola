use gosim::LogMocca;
use gossim_apps::apps::FlowSimPipeChainMocca;
use mocca::{MoccaRunSettings, MoccaRunner};

fn main() {
    MoccaRunner::run::<(LogMocca, FlowSimPipeChainMocca)>(MoccaRunSettings::app());
}
