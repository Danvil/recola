//! Puzzel generator
//!
//! A puzzle is set of rules.
//! A state defines the room in which the player is located and the state of other objects.
//! Each state offers a set of actions based on the puzzle rules.
//! Graph search algorithms can be used to expand states and find puzzle solutions.
//! Proc-gen can be used to generate puzzles.

use bitmask_enum::bitmask;
use petgraph::{Graph, graph::UnGraph, visit::EdgeRef};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt,
    ops::{Add, Deref, Mul},
};

#[derive(Debug, Clone)]
pub struct Puzzle {
    name: String,
    rooms_by_name: HashMap<String, RoomId>,
    entities: Vec<Entity>,
    room_graph: RoomGraph,
    win_room: RoomId,
    initial_state: PuzzleState,
}

/// Elements of a puzzle. We use an uber-entity architecture for simplicity and because
/// components are quite bounded.
#[derive(Debug, Default, Clone)]
pub struct Entity {
    /// If this condition is met
    condition: PowerCondition,

    /// Valid targets of this entity
    target: TargetKind,

    /// This effect is applied
    effect: Option<Effect>,
}

#[derive(Debug, Default, Clone)]
pub enum PowerCondition {
    #[default]
    Never,
    Always,
    Power {
        /// If enabled the entity stays active after being powered the first time
        latch: bool,
        /// Amount of power necessary to activate (all must be fulfilled)
        power: Power,
    },
}

#[derive(Debug, Default, Clone)]
pub enum TargetKind {
    #[default]
    None,

    /// The target cannot be changed
    Fixed(EntityId),

    /// The target can be changed to one of the list (or None)
    Changable(Vec<EntityId>),
}

#[derive(Debug, Clone)]
pub enum Effect {
    ProvidePower(PowerProvider),
}

#[derive(Debug, Clone)]
pub struct PowerProvider {
    kind: PowerKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(usize);

impl Deref for EntityId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[bitmask]
pub enum PowerKind {
    Player,
    Laser,
    Switch,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Power {
    laser: usize,
    player: usize,
    switch: usize,
}

impl Power {
    pub const ZERO: Self = Self {
        laser: 0,
        player: 0,
        switch: 0,
    };

    pub fn one(kind: PowerKind) -> Self {
        let mut out = Power::default();
        out.inc(kind);
        out
    }

    pub fn inc(&mut self, kind: PowerKind) {
        if kind.contains(PowerKind::Laser) {
            self.laser += 1;
        }
        if kind.contains(PowerKind::Player) {
            self.player += 1;
        }
        if kind.contains(PowerKind::Switch) {
            self.switch += 1;
        }
    }

    pub fn dec(&mut self, kind: PowerKind) {
        if kind.contains(PowerKind::Laser) {
            assert!(self.laser >= 1);
            self.laser -= 1;
        }
        if kind.contains(PowerKind::Player) {
            assert!(self.player >= 1);
            self.player -= 1;
        }
        if kind.contains(PowerKind::Switch) {
            assert!(self.switch >= 1);
            self.switch -= 1;
        }
    }

    pub fn ge(&self, other: &Power) -> bool {
        self.laser >= other.laser && self.player >= other.player && self.switch >= other.switch
    }

    pub fn lt(&self, other: &Power) -> bool {
        !self.ge(other)
    }

    pub fn min(&self, other: &Power) -> Power {
        Power {
            laser: self.laser.min(other.laser),
            player: self.player.min(other.player),
            switch: self.switch.min(other.switch),
        }
    }
}

impl Mul<usize> for Power {
    type Output = Power;

    fn mul(self, other: usize) -> Self::Output {
        Power {
            laser: self.laser * other,
            player: self.player * other,
            switch: self.switch * other,
        }
    }
}

impl Add<Power> for Power {
    type Output = Power;

    fn add(self, other: Power) -> Self::Output {
        Power {
            laser: self.laser + other.laser,
            player: self.player + other.player,
            switch: self.switch + other.switch,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Room {
    entities: Vec<EntityId>,
}

impl Room {
    pub fn from_entities(entities: impl IntoIterator<Item = EntityId>) -> Self {
        Room {
            entities: entities.into_iter().collect(),
        }
    }
}

type RoomGraph = UnGraph<Room, EntityId>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoomId(petgraph::prelude::NodeIndex);

impl Deref for RoomId {
    type Target = petgraph::prelude::NodeIndex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GateId(petgraph::prelude::EdgeIndex);

impl Deref for GateId {
    type Target = petgraph::prelude::EdgeIndex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PuzzleState {
    player_room: RoomId,
    player_power_target: Option<EntityId>,
    entities: Vec<EntityState>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct EntityState {
    /// Amount of power currently provided
    power: Power,

    /// If the entity is activated
    is_active: bool,

    /// Current target for Effect::ProvidePower
    target: Option<EntityId>,
}

/// Actions change the state of a puzzle
#[derive(Debug, Clone)]
pub enum Action {
    /// Player moves to another room. Only
    MovePlayer { room: RoomId },

    /// Player provides power to an entity.
    /// Can only target entities in the same room.
    /// This will remove power from the current player target.
    ProvidePlayerPower { target: Option<EntityId> },

    /// Change power target of an entity.
    /// Can only target entities from the target list.
    /// This will remove power from the current target.
    SetTarget {
        entity: EntityId,
        target: Option<EntityId>,
    },
}

impl PuzzleState {
    pub fn new(player_room: RoomId, entity_count: usize) -> Self {
        PuzzleState {
            player_room,
            player_power_target: None,
            entities: (0..entity_count).map(|_| EntityState::default()).collect(),
        }
    }

    pub fn branch(&self, spec: &Puzzle, action: &Action) -> Self {
        let mut out = self.clone();
        out.apply(spec, action);
        out
    }

    pub fn setup(&mut self, spec: &Puzzle) {
        for (i, entity_spec) in spec.entities.iter().enumerate() {
            let entity = EntityId(i);

            // Set target entity for entities with fixed target
            match entity_spec.target {
                TargetKind::Fixed(target) => {
                    self.entities[*entity].target = Some(target);
                    if let Some(Effect::ProvidePower(pp)) = &spec.entities[*entity].effect {
                        self.provide_power(spec, Some(entity), target, pp.kind);
                    }
                }
                _ => {}
            }

            // Power on entities which are always powered
            match entity_spec.condition {
                PowerCondition::Always => self.activate(spec, entity),
                _ => {}
            }
        }
    }

    pub fn apply(&mut self, spec: &Puzzle, action: &Action) {
        match *action {
            Action::MovePlayer { room } => {
                self.player_room = room;
            }
            Action::SetTarget { entity, target } => {
                if let Some(Effect::ProvidePower(pp)) = &spec.entities[*entity].effect {
                    if let Some(old_target) = self.entities[*entity].target {
                        self.remove_power(spec, Some(entity), old_target, pp.kind);
                    }

                    self.entities[*entity].target = target;

                    if let Some(new_target) = target {
                        self.provide_power(spec, Some(entity), new_target, pp.kind);
                    }
                } else {
                    panic!("invalid action");
                }
            }
            Action::ProvidePlayerPower { target } => {
                if let Some(old_target) = self.player_power_target {
                    self.remove_power(spec, None, old_target, PowerKind::Player);
                }

                self.player_power_target = target;

                if let Some(new_target) = target {
                    self.provide_power(spec, None, new_target, PowerKind::Player);
                }
            }
        }
    }

    /// Provide power to an entity
    fn provide_power(
        &mut self,
        spec: &Puzzle,
        src: Option<EntityId>,
        target: EntityId,
        power: PowerKind,
    ) {
        if let Some(src) = src {
            if !self.entities[*src].is_active {
                return;
            }
        }

        let entity_spec = &spec.entities[*target];
        let entity_state = &mut self.entities[*target];

        // provide power to the entity
        entity_state.power.inc(power);

        if !entity_state.is_active {
            // check if enough power for activation is provided
            match entity_spec.condition {
                PowerCondition::Power { power, .. } => {
                    if entity_state.power.ge(&power) {
                        self.activate(spec, target);
                    }
                }
                _ => unreachable!(),
            }
        }
    }

    fn activate(&mut self, spec: &Puzzle, entity: EntityId) {
        let entity_spec = &spec.entities[*entity];
        let entity_state = &mut self.entities[*entity];

        entity_state.is_active = true;

        // apply the power effect
        match &entity_spec.effect {
            Some(Effect::ProvidePower(pp)) => {
                if let Some(target) = entity_state.target {
                    self.provide_power(spec, Some(entity), target, pp.kind);
                }
            }
            None => {}
        }
    }

    /// Remove power from an entity
    fn remove_power(
        &mut self,
        spec: &Puzzle,
        src: Option<EntityId>,
        target: EntityId,
        power: PowerKind,
    ) {
        if let Some(src) = src {
            if !self.entities[*src].is_active {
                return;
            }
        }

        let entity_spec = &spec.entities[*target];
        let entity_state = &mut self.entities[*target];

        // remove power from the entity
        entity_state.power.dec(power);

        if entity_state.is_active {
            // remove the power effect
            match entity_spec.condition {
                PowerCondition::Power { power, latch } => {
                    if entity_state.power.lt(&power) && !latch {
                        self.deactivate(spec, target);
                    }
                }
                _ => {}
            }
        }
    }

    fn deactivate(&mut self, spec: &Puzzle, entity: EntityId) {
        let entity_spec = &spec.entities[*entity];
        let entity_state = &mut self.entities[*entity];

        // remove the power effect
        match &entity_spec.effect {
            Some(Effect::ProvidePower(pp)) => {
                if let Some(next_target) = entity_state.target {
                    self.remove_power(spec, Some(entity), next_target, pp.kind);
                }
            }
            None => {}
        }

        self.entities[*entity].is_active = false;
    }
}

impl Puzzle {
    pub fn initalize(&self) -> PuzzleState {
        let mut state = self.initial_state.clone();
        state.setup(self);
        state
    }

    pub fn add_room(&mut self, name: String) -> RoomId {
        let id = RoomId(self.room_graph.add_node(Room::default()));
        self.rooms_by_name.insert(name, id);
        id
    }

    pub fn add_gate(&mut self, room_1: RoomId, room_2: RoomId, gate: EntityId) {
        self.room_graph.add_edge(*room_1, *room_2, gate);
    }

    pub fn extend_entities(
        &mut self,
        entities: impl IntoIterator<Item = Entity, IntoIter: ExactSizeIterator>,
    ) {
        let iter = entities.into_iter();
        let len = iter.len();
        self.entities.extend(iter);
        self.initial_state
            .entities
            .extend((0..len).map(|_| Default::default()));
    }

    pub fn room_id_by_name(&self, name: &str) -> Option<RoomId> {
        self.rooms_by_name.get(name).cloned()
    }

    pub fn room_by_name(&self, name: &str) -> Option<&Room> {
        let node_id = *self.rooms_by_name.get(name)?;
        Some(&self.room_graph[*node_id])
    }

    pub fn room_by_name_mut(&mut self, name: &str) -> Option<&mut Room> {
        let node_id = *self.rooms_by_name.get(name)?;
        Some(&mut self.room_graph[*node_id])
    }

    fn actions(&self, state: &PuzzleState) -> Vec<Action> {
        let mut out = vec![];

        // move player through open gates
        for edge in self.room_graph.edges(*state.player_room) {
            if state.entities[**edge.weight()].is_active {
                out.push(Action::MovePlayer {
                    room: RoomId(edge.target()),
                });
            }
        }

        // Interaction with entities in current room
        for entity in &self.room_graph[*state.player_room].entities {
            let entity_spec = &self.entities[**entity];
            let entity_state = &state.entities[**entity];

            // modify entity target
            match &entity_spec.target {
                TargetKind::None | TargetKind::Fixed(_) => {}
                TargetKind::Changable(targets) => {
                    // change target
                    for &target in targets {
                        if Some(target) != entity_state.target {
                            out.push(Action::SetTarget {
                                entity: *entity,
                                target: Some(target),
                            });
                        }
                    }

                    // clear target
                    if entity_state.target.is_some() {
                        out.push(Action::SetTarget {
                            entity: *entity,
                            target: None,
                        });
                    }
                }
            }

            if let PowerCondition::Power { power, .. } = &entity_spec.condition {
                // provide player power to entity if not at max
                let with_player_power =
                    (entity_state.power + Power::one(PowerKind::Player)).min(&power);
                if with_player_power != entity_state.power {
                    out.push(Action::ProvidePlayerPower {
                        target: Some(*entity),
                    });
                }
            }
        }

        // remove player power if currently providing power
        if state.player_power_target.is_some() {
            out.push(Action::ProvidePlayerPower { target: None });
        }

        out
    }
}

fn exit_gate() -> Entity {
    Entity {
        condition: PowerCondition::Power {
            latch: true,
            power: Power::one(PowerKind::Switch),
        },
        ..Default::default()
    }
}

fn rift(switch_count: usize) -> Entity {
    Entity {
        condition: PowerCondition::Power {
            latch: true,
            power: Power::one(PowerKind::Player) + Power::one(PowerKind::Switch) * switch_count,
        },
        target: TargetKind::Fixed(EntityId(0)),
        effect: Some(Effect::ProvidePower(PowerProvider {
            kind: PowerKind::Switch,
        })),
        ..Default::default()
    }
}

fn switch(target: EntityId) -> Entity {
    Entity {
        condition: PowerCondition::Power {
            latch: false,
            power: Power::one(PowerKind::Laser),
        },
        target: TargetKind::Fixed(target),
        effect: Some(Effect::ProvidePower(PowerProvider {
            kind: PowerKind::Switch,
        })),
        ..Default::default()
    }
}

fn rift_switch() -> Entity {
    switch(EntityId(1))
}

fn laser(targets: impl IntoIterator<Item = EntityId>) -> Entity {
    Entity {
        condition: PowerCondition::Always,
        target: TargetKind::Changable(targets.into_iter().collect()),
        effect: Some(Effect::ProvidePower(PowerProvider {
            kind: PowerKind::Laser,
        })),
        ..Default::default()
    }
}

fn overgrowth() -> Entity {
    Entity {
        condition: PowerCondition::Power {
            latch: true,
            power: Power::one(PowerKind::Laser),
        },
        ..Default::default()
    }
}

fn barrier_switch(target: EntityId) -> Entity {
    switch(target)
}

fn barrier() -> Entity {
    Entity {
        condition: PowerCondition::Power {
            latch: false,
            power: Power::one(PowerKind::Switch),
        },
        ..Default::default()
    }
}

/// Creates a puzzle with an exit room (0) linked to a start room(1) and a rift in the start room
fn puzzle_basis(name: &str, rift_switch_power: usize) -> Puzzle {
    let entities = vec![exit_gate(), rift(rift_switch_power)];

    let mut room_graph = RoomGraph::new_undirected();
    // Exit room
    let room_0 = room_graph.add_node(Room::default());
    // Main room
    let room_1 = room_graph.add_node(Room::from_entities([EntityId(1)]));
    let _gate_0 = GateId(room_graph.add_edge(room_0, room_1, EntityId(0)));

    let rooms_by_name = [
        ("exit".to_string(), RoomId(room_0)),
        ("main".to_string(), RoomId(room_1)),
    ];

    let initial_state = PuzzleState::new(RoomId(room_1), 2);

    Puzzle {
        name: name.into(),
        rooms_by_name: HashMap::from_iter(rooms_by_name),
        entities,
        room_graph,
        win_room: RoomId(room_0),
        initial_state,
    }
}

fn level_1() -> Puzzle {
    puzzle_basis("level_1", 0)
}

fn level_2() -> Puzzle {
    let mut basis = puzzle_basis("Level 1-2", 2);

    basis.extend_entities([
        // [2] Rift Switch 1
        rift_switch(),
        // [3] Rift Switch 2
        rift_switch(),
        // [4] Laser 1
        laser(vec![EntityId(2), EntityId(3)]),
        // [5] Laser 2
        laser(vec![EntityId(2)]),
    ]);

    basis.room_by_name_mut("main").unwrap().entities.extend([
        EntityId(2),
        EntityId(3),
        EntityId(4),
        EntityId(5),
    ]);

    basis
}

fn level_3() -> Puzzle {
    let mut basis = puzzle_basis("Level 1-3", 3);

    // overgrowth gate
    let green_room_id = basis.add_room("green_room".into());
    basis.add_gate(
        basis.room_id_by_name("main").unwrap(),
        green_room_id,
        EntityId(8),
    );

    basis.extend_entities([
        // [2] Rift Switch 1 "center"
        rift_switch(),
        // [3] Rift Switch 2 "left"
        rift_switch(),
        // [4] Rift Switch 3 "right"
        rift_switch(),
        // [5] Laser 1 "first"
        laser(vec![EntityId(2), EntityId(3), EntityId(8)]),
        // [6] Laser 2 "green room"
        laser(vec![EntityId(4)]),
        // [7] Laser 3 "alcove room"
        laser(vec![EntityId(2)]),
        // [8] Gate from main room to green room
        overgrowth(),
    ]);

    basis.room_by_name_mut("main").unwrap().entities.extend([
        EntityId(2),
        EntityId(3),
        EntityId(4),
        EntityId(5),
        EntityId(7),
    ]);

    basis
        .room_by_name_mut("green_room")
        .unwrap()
        .entities
        .extend([EntityId(6)]);

    basis
}

fn level_4() -> Puzzle {
    let mut basis = puzzle_basis("Level 1-4", 2);

    // barrier gate
    let room_2_id = basis.add_room("room_2".into());
    basis.add_gate(
        basis.room_id_by_name("main").unwrap(),
        room_2_id,
        EntityId(7),
    );

    basis.extend_entities([
        // [2] Rift Switch 1
        rift_switch(),
        // [3] Rift Switch 2
        rift_switch(),
        // [4] Laser 1
        laser(vec![EntityId(2), EntityId(6)]),
        // [5] Laser 2
        laser(vec![EntityId(3), EntityId(6)]),
        // [6] Barrier Switch
        barrier_switch(EntityId(7)),
        // [7] Barrier
        barrier(),
    ]);

    basis
        .room_by_name_mut("main")
        .unwrap()
        .entities
        .extend([EntityId(2), EntityId(4)]);

    basis.room_by_name_mut("room_2").unwrap().entities.extend([
        EntityId(3),
        EntityId(5),
        EntityId(6),
    ]);

    basis
}

fn level_5() -> Puzzle {
    let mut basis = puzzle_basis("Level 1-5", 3);

    // start room
    let start_id = basis.add_room("start".into());
    basis.add_gate(
        basis.room_id_by_name("main").unwrap(),
        start_id,
        EntityId(2),
    );
    basis.initial_state.player_room = start_id;

    // annex room
    let room_3_id = basis.add_room("annex".into());
    basis.add_gate(
        basis.room_id_by_name("main").unwrap(),
        room_3_id,
        EntityId(3),
    );

    basis.extend_entities([
        // [2] Barrier
        barrier(),
        // [3] Barrier
        barrier(),
        // [4] Rift Switch 1
        rift_switch(),
        // [5] Rift Switch 2
        rift_switch(),
        // [6] Rift Switch 3
        rift_switch(),
        // [7] Laser 1
        laser(vec![EntityId(5), EntityId(10), EntityId(11)]),
        // [8] Laser 2
        laser(vec![EntityId(4), EntityId(5), EntityId(10)]),
        // [9] Laser 3
        laser(vec![EntityId(6), EntityId(11)]),
        // [10] Barrier Switch
        barrier_switch(EntityId(2)),
        // [11] Barrier Switch
        barrier_switch(EntityId(3)),
    ]);

    basis.room_by_name_mut("main").unwrap().entities.extend([
        EntityId(4),
        EntityId(5),
        EntityId(8),
        EntityId(10),
    ]);

    basis
        .room_by_name_mut("start")
        .unwrap()
        .entities
        .extend([EntityId(7)]);

    basis.room_by_name_mut("annex").unwrap().entities.extend([
        EntityId(6),
        EntityId(9),
        EntityId(11),
    ]);

    basis
}

fn main() {
    println!("RECOLA puzzle generator");

    for puzzle in [level_1(), level_2(), level_3(), level_4(), level_5()] {
        expand_and_print(&puzzle, puzzle.initalize(), 10000);
    }
}

fn expand_and_print(puzzle: &Puzzle, start: PuzzleState, max_nodes: usize) {
    let mut graph = Graph::new();
    let mut index_of = HashMap::new();
    let mut visited = HashSet::new();

    println!();
    println!("LEVEL: {}", puzzle.name);
    println!();
    println!("{puzzle}");
    println!();

    let start_ix = graph.add_node(start.clone());
    index_of.insert(start.clone(), start_ix);
    visited.insert(start.clone());

    let mut q: VecDeque<(PuzzleState, usize)> = VecDeque::new();
    q.push_back((start, 0));

    let mut expanded = 1;
    let mut first_solution_depth = None;
    let mut max_solution_depth = 0;
    let mut total_solutions = 0;

    while let Some((current, current_depth)) = q.pop_front() {
        let actions = puzzle.actions(&current);
        for action in actions {
            let state = current.branch(puzzle, &action);

            let state_ix = if let Some(&ix) = index_of.get(&state) {
                ix
            } else {
                let ix = graph.add_node(state.clone());
                index_of.insert(state.clone(), ix);
                ix
            };

            let from_ix = *index_of.get(&current).expect("node must exist");
            graph.add_edge(from_ix, state_ix, ());

            if visited.insert(state.clone()) {
                // println!("{expanded:05} [{current_depth}] ACTION: {action}");
                let win = state.player_room == puzzle.win_room;
                if win {
                    total_solutions += 1;
                    if first_solution_depth.is_none() {
                        println!("{expanded:05} depth={current_depth}: {state}",);
                        first_solution_depth = Some(current_depth);
                    }
                    max_solution_depth = max_solution_depth.max(current_depth);
                } else {
                    q.push_back((state, current_depth + 1));
                }
            } else {
                // repeat
            }

            expanded += 1;
            if expanded >= max_nodes {
                println!("Aborted due to maximum number of nodes reached");
                return;
            }
        }
    }

    println!("Total node expansion: {}", expanded);
    println!("Total solutions: {}", total_solutions);
    if let Some(depth) = first_solution_depth {
        println!("First solution depth: {}", depth);
        println!("Max solution depth: {}", max_solution_depth);
    } else {
        println!("No solution found");
    }
}

// Display

// --- Atomics ---------------------------------------------------------------

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "E{}", self.0)
    }
}

impl fmt::Display for RoomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "R{}", self.0.index())
    }
}

impl fmt::Display for GateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "G{}", self.0.index())
    }
}

impl fmt::Display for Power {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Compact: omit zero fields; print Ø when all zero.
        let mut first = true;
        let mut write_field = |name: &str, val: usize| -> fmt::Result {
            if val > 0 {
                if !first {
                    write!(f, " ")?;
                }
                write!(f, "{}{}", name, val)?;
                first = false;
            }
            Ok(())
        };
        write_field("L", self.laser)?;
        write_field("P", self.player)?;
        write_field("S", self.switch)?;
        if first { write!(f, "Ø") } else { Ok(()) }
    }
}

impl fmt::Display for EntityState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{power:{}, {}, target:{}}}",
            self.power,
            if self.is_active { "on" } else { "off" },
            match self.target {
                Some(t) => format!("{t}"),
                None => "-".to_string(),
            }
        )
    }
}

// --- High-level states -----------------------------------------------------

impl fmt::Display for PuzzleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "room:{}, player_target:{}, entities:[",
            self.player_room,
            match self.player_power_target {
                Some(id) => format!("{id}"),
                None => "-".to_string(),
            }
        )?;
        for (i, es) in self.entities.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "#{i}:{es}")?;
        }
        write!(f, "]")
    }
}

impl fmt::Display for Puzzle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let g = &self.room_graph;
        let rooms = g.node_count();
        let gates = g.edge_count();
        let ents = self.entities.len();

        writeln!(f, "Puzzle[rooms:{rooms}, gates:{gates}, entities:{ents}]")?;

        // Rooms with their entity lists.
        for (_idx, room) in g.node_indices().enumerate() {
            let r = &g[room];
            write!(f, "  R{}: [", room.index())?;
            for (i, eid) in r.entities.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{eid}")?;
            }
            writeln!(f, "]")?;
        }

        // Gates as undirected edges labeled by the gate entity id (edge weight).
        for e in g.edge_references() {
            let a = e.source().index();
            let b = e.target().index();
            let label = e.weight();
            writeln!(f, "  R{a} --{label}-- R{b}")?;
        }

        for (i, e) in self.entities.iter().enumerate() {
            writeln!(
                f,
                "E{:02}: condition={:?}, effect={:?}, target={:?}",
                i, e.condition, e.effect, e.target
            )?;
        }

        // Initial state summary on a single line for quick scans.
        write!(f, "  initial: {}", self.initial_state)
    }
}

// --- Actions ---------------------------------------------------------------

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::MovePlayer { room } => {
                write!(f, "MovePlayer → {}", room)
            }
            Action::ProvidePlayerPower { target } => match target {
                Some(t) => write!(f, "ProvidePlayerPower → {}", t),
                None => write!(f, "ProvidePlayerPower → (none)"),
            },
            Action::SetTarget { entity, target } => match target {
                Some(t) => write!(f, "SetTarget {} → {}", entity, t),
                None => write!(f, "ClearTarget {}", entity),
            },
        }
    }
}
