use crate::{Pipe, PortTag};
use slab::Slab;
use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipeId(pub usize);

impl Deref for PipeId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct JunctionId(pub usize);

impl Deref for JunctionId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Port {
    PipeOutlet { pipe_id: PipeId, side: PortTag },
}

#[derive(Default)]
pub struct FlowNet {
    pub(crate) pipes: Slab<Pipe>,
    pub(crate) pipe_to_junc: HashMap<Port, JunctionId>,
    pub(crate) junctions: Slab<HashSet<Port>>,
}

impl FlowNet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_pipe(&mut self, pipe: Pipe) -> PipeId {
        let idx = self.pipes.insert(pipe);
        PipeId(idx)
    }

    pub fn get_pipe(&self, id: PipeId) -> Option<&Pipe> {
        self.pipes.get(*id)
    }

    pub fn connect(&mut self, p1: (PipeId, PortTag), p2: (PipeId, PortTag)) {
        let key1 = Port::PipeOutlet {
            pipe_id: p1.0,
            side: p1.1,
        };
        let key2 = Port::PipeOutlet {
            pipe_id: p2.0,
            side: p2.1,
        };

        match (
            self.pipe_to_junc.get(&key1).cloned(),
            self.pipe_to_junc.get(&key2).cloned(),
        ) {
            (None, None) => {
                // Neither pipe port is connected to a junction yet: create a new junction.

                let ports = HashSet::from_iter([key1, key2]);
                let j = JunctionId(self.junctions.insert(ports));

                self.pipe_to_junc.insert(key1, j);
                self.pipe_to_junc.insert(key2, j);
            }
            (Some(j), None) => {
                // First pipe is connected to a junction already: also connect the other one.
                self.junctions[*j].insert(key2);
                self.pipe_to_junc.insert(key2, j);
            }
            (None, Some(j)) => {
                // Second pipe is connected to a junction already: also connect the other one.
                self.junctions[*j].insert(key1);
                self.pipe_to_junc.insert(key1, j);
            }
            (Some(j1), Some(j2)) => {
                // Both pipes are connected to a junction already: merge the junctions into one.
                self.join_junctions(j1, j2);
            }
        }
    }

    /// Joins the second junction into the first one thus connecting all pipe ports they are
    /// connected to.
    pub fn join_junctions(&mut self, j1: JunctionId, j2: JunctionId) {
        // Find all pipe-ports connected to J2
        let j2_ports: Vec<Port> = self
            .pipe_to_junc
            .iter()
            .filter_map(|(k, v)| (*v == j2).then(|| k.clone()))
            .collect::<Vec<_>>();

        // Connect them to J1 instead
        for port in j2_ports {
            self.junctions[*j1].insert(port);
            self.pipe_to_junc.insert(port, j1);
        }

        // Delete J2
        self.junctions.remove(*j2);
    }
}
