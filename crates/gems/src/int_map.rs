use std::{
    mem::swap,
    ops::{Index, IndexMut},
};

#[derive(Clone, Debug)]
pub struct IntMap<T> {
    items: Vec<Entry<T>>,
}

impl<T> Default for IntMap<T> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

#[derive(Clone, Debug)]
pub enum Entry<T> {
    Empty,
    Occupied(T),
}

impl<T> Entry<T> {
    fn as_ref_opt(&self) -> Option<&T> {
        match self {
            Entry::Empty => None,
            Entry::Occupied(x) => Some(x),
        }
    }

    fn as_mut_opt(&mut self) -> Option<&mut T> {
        match self {
            Entry::Empty => None,
            Entry::Occupied(x) => Some(x),
        }
    }
}

impl<T> Default for Entry<T> {
    fn default() -> Self {
        Entry::Empty
    }
}

impl<T> IntMap<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (usize, T)>,
    {
        let mut out = Self::default();
        for (k, v) in iter {
            out.set(k, v);
        }
        out
    }

    pub fn from_count<F>(count: usize, f: F) -> Self
    where
        F: Fn(usize) -> T,
    {
        let mut out = IntMap::default();
        for i in 0..count {
            out.set(i, f(i));
        }
        out
    }

    /// Total number of slots empty or occupied
    pub fn slot_count(&self) -> usize {
        self.items.len()
    }

    pub fn set_slot_count(&mut self, count: usize) {
        // TODO implement more efficiently
        while self.items.len() < count {
            self.items.push(Entry::Empty);
        }
    }

    pub fn set(&mut self, index: usize, value: T) {
        self.items.resize_with(index + 1, || Entry::Empty);
        self.items[index] = Entry::Occupied(value);
    }

    pub fn insert(&mut self, value: T) -> usize {
        match self.items.iter().position(|e| matches!(e, Entry::Empty)) {
            Some(index) => {
                self.items[index] = Entry::Occupied(value);
                index
            }
            None => {
                let index = self.items.len();
                self.items.push(Entry::Occupied(value));
                index
            }
        }
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        let mut tmp = Entry::Empty;
        swap(self.items.get_mut(index)?, &mut tmp);
        match tmp {
            Entry::Empty => None,
            Entry::Occupied(x) => Some(x),
        }
    }

    pub fn clear(&mut self) {
        // for x in self.items.iter_mut() {
        //     *x = Entry::Empty;
        // }
        self.items.clear();
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        match self.items.get(index) {
            None | Some(Entry::Empty) => None,
            Some(Entry::Occupied(value)) => Some(value),
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        match self.items.get_mut(index) {
            None | Some(Entry::Empty) => None,
            Some(Entry::Occupied(value)) => Some(value),
        }
    }

    pub fn map<S, F>(&self, f: F) -> IntMap<S>
    where
        F: Fn(&T) -> S,
    {
        IntMap {
            items: self
                .items
                .iter()
                .map(|e| match e {
                    Entry::Empty => Entry::Empty,
                    Entry::Occupied(x) => Entry::Occupied(f(x)),
                })
                .collect(),
        }
    }

    pub fn zip_iter<'a, 'b>(
        a: &'a Self,
        b: &'b Self,
    ) -> impl Iterator<Item = (usize, Option<&'a T>, Option<&'b T>)> {
        assert_eq!(a.items.len(), b.items.len());
        a.items
            .iter()
            .enumerate()
            .zip(b.items.iter())
            .map(|((i, a), b)| (i, a.as_ref_opt(), b.as_ref_opt()))
    }
}

// #[derive(Clone, Copy, PartialEq, Eq, Hash)]
// pub struct IntMapId<T>(usize, PhantomData<T>);

// impl<T> IntMapId<T> {
//     pub fn from_usize(index: usize) -> Self {
//         Self(index, PhantomData)
//     }

//     pub fn as_usize(&self) -> usize {
//         self.0
//     }
// }

// impl<T> Deref for IntMapId<T> {
//     type Target = usize;

//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }

// impl<T> Index<IntMapId<T>> for IntMap<T> {
//     type Output = T;

//     fn index(&self, index: IntMapId<T>) -> &T {
//         self.get(*index).unwrap()
//     }
// }

// impl<T> IndexMut<IntMapId<T>> for IntMap<T> {
//     fn index_mut(&mut self, index: IntMapId<T>) -> &mut T {
//         self.get_mut(*index).unwrap()
//     }
// }

impl<T> Index<usize> for IntMap<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        self.get(index).unwrap()
    }
}

impl<T> IndexMut<usize> for IntMap<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        self.get_mut(index).unwrap()
    }
}

impl<T> IntMap<T> {
    pub fn iter(&self) -> IntMapIter<'_, T> {
        IntMapIter {
            inner: self.items.iter().enumerate(),
        }
    }
}

pub struct IntMapIter<'a, T> {
    inner: std::iter::Enumerate<std::slice::Iter<'a, Entry<T>>>,
}

impl<'a, T> Iterator for IntMapIter<'a, T> {
    type Item = (usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((i, entry)) = self.inner.next() {
            if let Entry::Occupied(v) = entry {
                return Some((i, v));
            }
        }
        None
    }
}

impl<T> IntMap<T> {
    pub fn iter_mut(&mut self) -> IntMapIterMut<'_, T> {
        IntMapIterMut {
            inner: self.items.iter_mut().enumerate(),
        }
    }
}

pub struct IntMapIterMut<'a, T> {
    inner: std::iter::Enumerate<std::slice::IterMut<'a, Entry<T>>>,
}

impl<'a, T> Iterator for IntMapIterMut<'a, T> {
    type Item = (usize, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((i, entry)) = self.inner.next() {
            if let Entry::Occupied(v) = entry {
                return Some((i, v));
            }
        }
        None
    }
}

// impl<T> Add for IntMap<T>
// where
//     T: Clone + Add<T, Output = T>,
// {
//     type Output = Self;

//     fn add(self, rhs: IntMap<T>) -> Self {
//         IntMap::from_iter(
//             IntMap::zip_iter(&self, &rhs).filter_map(|(i, a, b)| match (a, b) {
//                 (Some(a), Some(b)) => Some((i, a.clone() + b.clone())),
//                 (Some(x), None) | (None, Some(x)) => Some((i, x.clone())),
//                 (None, None) => None,
//             }),
//         )
//     }
// }

// impl<T> Sub for IntMap<T>
// where
//     T: Clone + Sub<T, Output = T>,
// {
//     type Output = Self;

//     fn sub(self, rhs: IntMap<T>) -> Self {
//         IntMap::from_iter(
//             IntMap::zip_iter(&self, &rhs).filter_map(|(i, a, b)| match (a, b) {
//                 (Some(a), Some(b)) => Some((i, a.clone() - b.clone())),
//                 (Some(x), None) | (None, Some(x)) => Some((i, x.clone())),
//                 (None, None) => None,
//             }),
//         )
//     }
// }

// impl<T> Mul<f64> for IntMap<T>
// where
//     T: Clone + Mul<f64, Output = T>,
// {
//     type Output = Self;

//     fn mul(self, rhs: f64) -> Self {
//         self.map(|x| x.clone() * rhs)
//     }
// }

pub trait IntMapTuple {
    type RowRef;

    fn iter(self) -> impl Iterator<Item = Self::RowRef>;

    fn map<F, S>(self, f: F) -> IntMap<S>
    where
        F: FnMut(Self::RowRef) -> S;
}

impl<M1, M2> IntMapTuple for (M1, M2)
where
    M1: IntMapRef,
    M2: IntMapRef,
{
    type RowRef = (M1::Item, M2::Item);

    fn iter(self) -> impl Iterator<Item = Self::RowRef> {
        self.0
            .iter_entries()
            .zip(self.1.iter_entries())
            .filter_map(
                |(a, b)| match (M1::entry_to_item(a), M2::entry_to_item(b)) {
                    (Some(a), Some(b)) => Some((a, b)),
                    _ => None,
                },
            )
    }

    fn map<F, S>(self, mut f: F) -> IntMap<S>
    where
        F: FnMut(Self::RowRef) -> S,
    {
        IntMap {
            items: self
                .0
                .iter_entries()
                .zip(self.1.iter_entries())
                .map(
                    |(a, b)| match (M1::entry_to_item(a), M2::entry_to_item(b)) {
                        (Some(a), Some(b)) => Entry::Occupied(f((a, b))),
                        _ => Entry::Empty,
                    },
                )
                .collect(),
        }
    }
}

impl<M1, M2, M3> IntMapTuple for (M1, M2, M3)
where
    M1: IntMapRef,
    M2: IntMapRef,
    M3: IntMapRef,
{
    type RowRef = (M1::Item, M2::Item, M3::Item);

    fn iter(self) -> impl Iterator<Item = Self::RowRef> {
        self.0
            .iter_entries()
            .zip(self.1.iter_entries())
            .zip(self.2.iter_entries())
            .filter_map(|((a1, a2), a3)| {
                match (
                    M1::entry_to_item(a1),
                    M2::entry_to_item(a2),
                    M3::entry_to_item(a3),
                ) {
                    (Some(a1), Some(a2), Some(a3)) => Some((a1, a2, a3)),
                    _ => None,
                }
            })
    }

    fn map<F, S>(self, mut f: F) -> IntMap<S>
    where
        F: FnMut(Self::RowRef) -> S,
    {
        IntMap {
            items: self
                .0
                .iter_entries()
                .zip(self.1.iter_entries())
                .zip(self.2.iter_entries())
                .map(|((a1, a2), a3)| {
                    match (
                        M1::entry_to_item(a1),
                        M2::entry_to_item(a2),
                        M3::entry_to_item(a3),
                    ) {
                        (Some(a1), Some(a2), Some(a3)) => Entry::Occupied(f((a1, a2, a3))),
                        _ => Entry::Empty,
                    }
                })
                .collect(),
        }
    }
}

pub trait IntMapRef {
    type Item;
    type Entry;

    fn iter_entries(self) -> impl Iterator<Item = Self::Entry>;
    fn entry_to_item(entry: Self::Entry) -> Option<Self::Item>;
}

impl<'a, T> IntMapRef for &'a IntMap<T> {
    type Item = &'a T;
    type Entry = &'a Entry<T>;

    fn iter_entries(self) -> impl Iterator<Item = Self::Entry> {
        self.items.iter()
    }

    fn entry_to_item(entry: Self::Entry) -> Option<Self::Item> {
        entry.as_ref_opt()
    }
}

impl<'a, T> IntMapRef for &'a mut IntMap<T> {
    type Item = &'a mut T;
    type Entry = &'a mut Entry<T>;

    fn iter_entries(self) -> impl Iterator<Item = Self::Entry> {
        self.items.iter_mut()
    }

    fn entry_to_item(entry: Self::Entry) -> Option<Self::Item> {
        entry.as_mut_opt()
    }
}
