pub struct Cycle<T>(Vec<T>, usize);

impl<T> Cycle<T> {
    pub fn from_iter(entries: impl IntoIterator<Item = T>) -> Self {
        let items = entries.into_iter().collect::<Vec<_>>();
        assert!(items.len() > 0);
        Self(items, 0)
    }

    pub fn cycle(&mut self, direction: CycleDirection) {
        match direction {
            CycleDirection::Forward => {
                self.1 = (self.1 + 1) % self.0.len();
            }
            CycleDirection::Backward => {
                self.1 = match self.1 {
                    0 => self.0.len() - 1,
                    i => i - 1,
                }
            }
        }
    }

    pub fn selection(&self) -> &T {
        &self.0[self.1]
    }

    pub fn selection_index(&self) -> usize {
        self.1
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CycleDirection {
    Forward,
    Backward,
}
