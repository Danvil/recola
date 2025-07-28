use flecs_ecs::prelude::Entity;
use ratatui::widgets::*;
use std::{cmp::Ordering, collections::HashMap};

pub struct EntityListView<T> {
    items: HashMap<Entity, EntityListItem<T>>,
    sorting: Vec<Entity>,
    selection: Option<Entity>,
    selection_index: Option<usize>,
    criteria: Option<Box<dyn SortCriteria<Item = T>>>,
}

impl<T> Default for EntityListView<T> {
    fn default() -> Self {
        Self {
            items: HashMap::default(),
            sorting: Vec::default(),
            selection: None,
            selection_index: None,
            criteria: None,
        }
    }
}

pub trait SortCriteria {
    type Item;

    fn cmp(&self, a: &Self::Item, b: &Self::Item) -> Ordering;
}

pub struct EntityListItem<T> {
    pub label: String,
    pub value: T,
}

impl<T> EntityListView<T> {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
            sorting: Vec::new(),
            selection: None,
            selection_index: None,
            criteria: None,
        }
    }

    pub fn update(&mut self, items: impl IntoIterator<Item = (Entity, String, T)>) {
        self.items.clear();

        for (entity, label, value) in items {
            self.items.insert(entity, EntityListItem { label, value });
        }

        // Retain only keys still in the map
        self.sorting.retain(|k| self.items.contains_key(k));

        // Add new keys from the map
        for k in self.items.keys() {
            if !self.sorting.contains(k) {
                self.sorting.push(k.clone());
            }
        }

        // Unselect if selected item was removed
        if let Some(selection) = self.selection {
            if !self.items.contains_key(&selection) {
                self.selection = None;
            }
        }

        // If nothing is selected select first
        if self.selection.is_none() {
            if let Some(e) = self.sorting.first() {
                self.selection = Some(*e);
                self.selection_index = Some(0);
            }
        }

        self.sort_by_impl();
    }

    pub fn set_sorting_criteria(&mut self, criteria: Option<Box<dyn SortCriteria<Item = T>>>) {
        self.criteria = criteria;
        self.sort_by_impl()
    }

    fn sort_by_impl(&mut self) {
        match &self.criteria {
            Some(criteria) => self
                .sorting
                .sort_by(|a, b| criteria.cmp(&self.items[a].value, &self.items[b].value)),
            None => {
                // sort by label
                self.sorting
                    .sort_by(|a, b| self.items[a].label.cmp(&self.items[b].label));
            }
        }

        self.update_selection_index();
    }

    fn update_selection_index(&mut self) {
        self.selection_index = self
            .selection
            .and_then(|selection| self.sorting.iter().position(|&x| x == selection));
    }

    pub fn selection(&self) -> Option<(Entity, &str, &T)> {
        self.selection
            .map(|e| (e, self.items[&e].label.as_str(), &self.items[&e].value))
    }

    pub fn selected_entity(&self) -> Option<Entity> {
        self.selection
    }

    pub fn selection_index(&self) -> Option<usize> {
        self.selection_index
    }

    pub fn select_next(&mut self) {
        match self.selection_index {
            Some(current) => {
                // If there is currently one selected the list is not empty.
                let next = (current + 1).min(self.sorting.len() - 1);
                self.selection = Some(self.sorting[next]);
                self.selection_index = Some(next);
            }
            None => {
                // select first
                // list might be empty
                if let Some(entity) = self.sorting.get(0) {
                    self.selection = Some(*entity);
                    self.selection_index = Some(0);
                }
            }
        }
    }

    pub fn select_previous(&mut self) {
        match self.selection_index {
            Some(current) => {
                // If there is currently one selected the list is not empty.
                let previous = current.saturating_sub(1);
                self.selection = Some(self.sorting[previous]);
                self.selection_index = Some(previous);
            }
            None => {
                // select first
                // list might be empty
                if let Some(entity) = self.sorting.get(0) {
                    self.selection = Some(*entity);
                    self.selection_index = Some(0);
                }
            }
        }
    }

    pub fn iter_sorted(&self) -> impl Iterator<Item = (Entity, &str, &T)> {
        self.sorting.iter().map(|e| {
            let item = &self.items[e];
            (*e, item.label.as_str(), &item.value)
        })
    }

    pub fn to_rows<F>(&self, mut f: F) -> Vec<Row>
    where
        F: for<'a> FnMut(Entity, &'a str, &'a T) -> Row<'a>,
    {
        self.iter_sorted().map(|(e, l, v)| f(e, l, v)).collect()
    }
}
