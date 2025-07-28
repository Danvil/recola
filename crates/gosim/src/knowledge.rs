// //! The server is the authority for the game world and while clients only have partial access.
// //! The server computes which information a client can access. This can be based on general rules,
// //! proximity, (in-game) subscriptions, or (in-game) purchases.

// // pub trait WorldApi {
// // 	fn apply_diff(&mut self, diff: WorldDiff);
// // }

// // pub enum WorldDiff {
// // 	Player(PlayerDiff),
// // }

// // pub enum PlayerDiff {

// // }

// // pub struct World {
// // 	players: ,

// // 	version: u64,
// // }

// // impl World {
// // 	pub fn new() -> Self {Self{
// // 		version: 0
// // 	}
// // 	}
// // }

// // impl WorldApi for World {
// // 	fn apply(&mut self, diff: WorldDiff) {
// // 		todo!()
// // 	}
// // }

// /// ID of an entity
// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// pub struct Entity(pub u64);

// pub mod knowledge {

//     use crate::Entity;
//     use std::collections::HashMap;
//     use std::collections::HashSet;

//     /// A key gives access to a group of entities
//     #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
//     pub struct Key(pub u64);

//     /// Stores what knowledge a key provides
//     pub struct Knowledge {
//         /// Stores for each key which entities it locks
//         locked_by: HashMap<Key, HashSet<Entity>>,

//         /// Stores for each entity which key are unlocking it
//         unlocked_by: HashMap<Entity, HashSet<Key>>,

//         /// Stores for each entity which keys it can access
//         access_to: HashMap<Entity, HashSet<Key>>,
//     }

//     pub enum Command {
//         /// Locks access to an entity with a key. If multiple keys lock an entity one key is enought
//         /// to access it.
//         Lock(Key, Entity),

//         /// Grants an entity a key and thus access to all knowledge locked behind it
//         Grant(Entity, Key),
//     }

//     impl Knowledge {
//         pub fn new() -> Self {
//             Self {
//                 locked_by: HashMap::new(),
//                 unlocked_by: HashMap::new(),
//                 access_to: HashMap::new(),
//             }
//         }

//         pub fn exec(&mut self, command: Command) {
//             match command {
//                 Command::Lock(key, secret) => {
//                     self.locked_by.entry(key).or_default().insert(secret);
//                     self.unlocked_by.entry(secret).or_default().insert(key);
//                 }
//                 Command::Grant(viewer, key) => {
//                     self.access_to.entry(viewer).or_default().insert(key);
//                 }
//             }
//         }

//         /// Iterates over all entities which can access a secret entity
//         pub fn iter_entity_viewers(&self, secret: Entity) -> impl Iterator<Item = Entity> {
//             let unlocking_keys = match self.unlocked_by.get(&secret) {
//                 Some(keys) => keys,
//                 None => return std::iter::empty(),
//             };

//             self.access_to.iter().filter_map(move |(viewer, keys)| {
//                 if unlocking_keys.iter().any(|k| keys.contains(k)) {
//                     Some(*viewer)
//                 } else {
//                     None
//                 }
//             })
//         }
//     }
// }
