mod pipe_vessel;

pub use pipe_vessel::*;

// // pipe outflow: pipe vessel -> junction vessel (negative delta volume)
// for port in junc.iter() {
//     match *port {
//         PipeOutlet { pipe_id, side } => {
//             let pipe_idx = *pipe_id;
//             let pipe = &mut net.pipes[*pipe_id];

//             let dv = &mut pipe_scratch[pipe_idx].delta_volume[side.index()];

//             if *dv < 0. {
//                 *dv *= junc_scr.supply_fullfillment;
//                 if *dv < 0. {
//                     for chunk in pipe.vessel.drain(side, -*dv) {
//                         junc_scr.vessel.fill(chunk);
//                     }
//                 }
//             }
//         }
//     }
// }

// // pipe inflow: junction vessel -> pipe vessel (positive delta volume)
// for port in junc.iter() {
//     match *port {
//         PipeOutlet { pipe_id, side } => {
//             let pipe_idx = *pipe_id;
//             let pipe = &mut net.pipes[*pipe_id];

//             let dv = &mut pipe_scratch[pipe_idx].delta_volume[side.index()];

//             if *dv > 0. {
//                 *dv *= junc_scr.demand_fullfillment;
//                 if *dv > 0. {
//                     if let Some(chunk) = junc_scr.vessel.drain(*dv) {
//                         pipe.vessel.fill(side, chunk);
//                     }
//                 }
//             }
//         }
//     }
// }
