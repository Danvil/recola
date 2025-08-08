use gossim_apps::apps::FlowNetPipeChainMocca;
use mocca::{MoccaRunSettings, MoccaRunner};

#[test]
fn test_flow_net_line() {
    MoccaRunner::run::<FlowNetPipeChainMocca>(MoccaRunSettings::test(2000));
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
