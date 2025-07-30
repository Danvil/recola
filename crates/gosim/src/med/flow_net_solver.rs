use faer::prelude::Solve;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct FlowNetSolver {
    pipes: Vec<Pipe>,
    junctions: HashMap<usize, HashSet<(usize, Port)>>,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
enum Port {
    A,
    B,
}

pub struct Pipe {
    pub conductivity: f64,
    pub external_pressure: f64,
    pub junctions: [usize; 2],
}

#[derive(Default)]
pub struct Flow {
    pub pressure: [f64; 2],
    pub flow: [f64; 2],
}

impl FlowNetSolver {
    pub fn add_pipe(&mut self, pipe: Pipe) -> usize {
        let i = self.pipes.len();
        self.junctions
            .entry(pipe.junctions[0])
            .or_default()
            .insert((i, Port::A));
        self.junctions
            .entry(pipe.junctions[1])
            .or_default()
            .insert((i, Port::B));
        self.pipes.push(pipe);
        i
    }

    pub fn solve(&self) -> Vec<Flow> {
        // We are going to solve a linear system A x = b.
        //
        // x stores four variables per pipe: pa, pb, qa, qb
        //
        // The number of equations is:
        //   per pipe:               through flow (1)
        //                           inflow (1)
        //   per junction (N ports): mass conversation (1)
        //                           pressure equality (N-1)
        let n = 4 * self.pipes.len();
        let m = 2 * self.pipes.len()
            + self
                .junctions
                .iter()
                .map(|(_, ports)| ports.len())
                .sum::<usize>();

        let ipaf = |i: usize| 4 * i;
        let ipbf = |i: usize| 4 * i + 1;
        let iqaf = |i: usize| 4 * i + 2;
        let iqbf = |i: usize| 4 * i + 3;

        let iqf = |(i, port): (usize, Port)| match port {
            Port::A => iqaf(i),
            Port::B => iqbf(i),
        };
        let ipf = |(i, port): (usize, Port)| match port {
            Port::A => ipaf(i),
            Port::B => ipbf(i),
        };

        let mut a = faer::Mat::<f64>::zeros(m, n);
        let mut b = faer::Col::zeros(m);

        let mut j = 0;

        for (i, pipe) in self.pipes.iter().enumerate() {
            let c2 = 2.0 * pipe.conductivity;

            // A inflow: qa = 2*g*(pa-P)
            a[(j, iqaf(i))] = 1.;
            a[(j, ipaf(i))] = -c2;
            b[j] = -c2 * pipe.external_pressure;
            j += 1;

            // B inflow: qb = 2*g*(pb-P)
            a[(j, iqbf(i))] = 1.;
            a[(j, ipbf(i))] = -c2;
            b[j] = -c2 * pipe.external_pressure;
            j += 1;
        }

        for (_, junc) in self.junctions.iter() {
            // mass conversation: sum_i q_i = 0
            for &ip in junc.iter() {
                a[(j, iqf(ip))] += 1.;
            }
            j += 1;

            // pressure equality: p_i = p_1, i >= 2
            if junc.len() >= 2 {
                let mut it = junc.iter();
                let v0 = ipf(*it.next().unwrap());
                for &ip in it {
                    a[(j, v0)] = 1.;
                    a[(j, ipf(ip))] = -1.;
                    j += 1;
                }
            }
        }

        assert_eq!(j, a.nrows());

        println!("A: {a:?}",);
        println!("b: {b:?}",);
        let lu = a.full_piv_lu();
        let x = lu.solve(&b);
        println!("x: {x:?}",);

        let mut out = Vec::new();
        for i in 0..self.pipes.len() {
            out.push(Flow {
                pressure: [x[ipaf(i)], x[ipbf(i)]],
                flow: [x[iqaf(i)], x[iqbf(i)]],
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flow_net_solver_1p() {
        // Single pipe, pressurized

        let mut solver = FlowNetSolver::default();
        solver.add_pipe(Pipe {
            conductivity: 1.,
            external_pressure: 1.,
            junctions: [0, 1],
        });

        let sol = solver.solve();
        assert_eq!(sol.len(), 1);
        approx::assert_relative_eq!(sol[0].pressure[0], 1.);
        approx::assert_relative_eq!(sol[0].pressure[1], 1.);
        approx::assert_relative_eq!(sol[0].flow[0], 0.);
        approx::assert_relative_eq!(sol[0].flow[1], 0.);
    }

    #[test]
    fn test_flow_net_solver_2p() {
        test_flow_net_solver_np_impl(2);
    }

    #[test]
    fn test_flow_net_solver_3p() {
        test_flow_net_solver_np_impl(3);
    }

    #[test]
    fn test_flow_net_solver_5p() {
        test_flow_net_solver_np_impl(5);
    }

    #[test]
    fn test_flow_net_solver_6p() {
        test_flow_net_solver_np_impl(6);
    }

    #[test]
    fn test_flow_net_solver_7p() {
        test_flow_net_solver_np_impl(7);
    }

    #[test]
    fn test_flow_net_solver_8p() {
        test_flow_net_solver_np_impl(8);
    }

    #[test]
    fn test_flow_net_solver_10p() {
        test_flow_net_solver_np_impl(10);
    }

    fn test_flow_net_solver_np_impl(n: usize) {
        // n pipes in a line, j-th pipe pressurized

        for j in 1..n - 1 {
            let mut solver = FlowNetSolver::default();
            for i in 0..n {
                solver.add_pipe(Pipe {
                    conductivity: 1.,
                    external_pressure: if i == j { 1. } else { 0. },
                    junctions: [i, i + 1],
                });
            }

            let sol = solver.solve();
            assert_eq!(sol.len(), n);

            for i in 0..n {
                approx::assert_relative_eq!(
                    sol[i].pressure[0],
                    if i == j {
                        0.5
                    } else if i == j + 1 {
                        0.5
                    } else {
                        0.
                    }
                );
                approx::assert_relative_eq!(
                    sol[i].pressure[1],
                    if i + 1 == j {
                        0.5
                    } else if i == j {
                        0.5
                    } else {
                        0.
                    }
                );
                approx::assert_relative_eq!(
                    sol[i].flow[0],
                    if i == j {
                        -1.
                    } else if i == j + 1 {
                        1.
                    } else {
                        0.
                    }
                );
                approx::assert_relative_eq!(
                    sol[i].flow[1],
                    if i + 1 == j {
                        1.
                    } else if i == j {
                        -1.
                    } else {
                        0.
                    }
                );
            }
        }
    }

    #[test]
    fn test_flow_net_solver_2p2j() {
        // Two pipes forming a ring, first pipe pressurized
        let mut solver = FlowNetSolver::default();
        solver.add_pipe(Pipe {
            conductivity: 1.,
            external_pressure: 1.,
            junctions: [0, 1],
        });
        solver.add_pipe(Pipe {
            conductivity: 1.,
            external_pressure: 0.,
            junctions: [0, 1],
        });

        let sol = solver.solve();
        assert_eq!(sol.len(), 2);

        approx::assert_relative_eq!(sol[0].pressure[0], 0.5);
        approx::assert_relative_eq!(sol[0].pressure[1], 0.5);
        approx::assert_relative_eq!(sol[0].flow[0], -0.5);
        approx::assert_relative_eq!(sol[0].flow[1], -0.5);
        approx::assert_relative_eq!(sol[1].pressure[0], 0.5);
        approx::assert_relative_eq!(sol[1].pressure[1], 0.5);
        approx::assert_relative_eq!(sol[1].flow[0], 0.5);
        approx::assert_relative_eq!(sol[1].flow[1], 0.5);
    }
}
