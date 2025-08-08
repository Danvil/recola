use gosim::LogMocca;
use gossim_apps::apps::FlowNetPipeChainMocca;
use mocca::{MoccaRunSettings, MoccaRunner};

fn main() {
    MoccaRunner::run::<(LogMocca, FlowNetPipeChainMocca)>(MoccaRunSettings::app());
}
