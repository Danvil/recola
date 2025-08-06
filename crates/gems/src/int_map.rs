use std::ops::{Index, IndexMut};

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
enum Entry<T> {
    Empty,
    Occupied(T),
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

    pub fn set(&mut self, index: usize, value: T) {
        self.items.resize_with(index + 1, || Entry::Empty);
        self.items[index] = Entry::Occupied(value);
    }

    pub fn clear(&mut self) {
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
}

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
