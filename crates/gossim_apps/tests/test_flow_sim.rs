use gossim_apps::apps::{FlowSimPipeChainMocca, FlowSimValveMocca};
use mocca::{MoccaRunSettings, MoccaRunner};

#[test]
fn test_flow_sim_line() {
    MoccaRunner::run::<FlowSimPipeChainMocca>(MoccaRunSettings::test(2000));
}

#[test]
fn test_flow_sim_valve() {
    MoccaRunner::run::<FlowSimValveMocca>(MoccaRunSettings::test(2000));
}

// #[test]
// fn test_flow_net_pump_a() {
//     FlowNetPump::run_test(PortTag::A, 2000);
// }

// #[test]
// fn test_flow_net_pump_b() {
//     FlowNetPump::run_test(PortTag::B, 2000);
// }

// #[test]
// fn test_flow_net_valve() {
//     FlowNetValve::run_test((), 2000);
// }
