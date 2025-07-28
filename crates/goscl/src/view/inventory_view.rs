use crate::{GosClientModel, View};
use flecs_ecs::{
    core::{Builder, QueryAPI, QueryBuilderImpl},
    prelude::{Entity, World},
};
use gems::{Cycle, CycleDirection};
use gosim::{
    ContainedBy, FlecsQueryRelationHelpers, HasInventory, ItemTag, PlayerTag, This, Weight,
};
use ratatui::{prelude::*, widgets::*};
use std::collections::HashMap;

pub struct InventoryView {
    view: InventoryViewTable,
    sorting_criteria_cycle: Cycle<InventorySortingCriteria>,
    popup: bool,
}

impl InventoryView {
    pub fn new() -> Self {
        Self {
            view: InventoryViewTable::new(),
            sorting_criteria_cycle: Cycle::from_iter([
                InventorySortingCriteria::Name,
                InventorySortingCriteria::Weight,
                InventorySortingCriteria::Value,
            ]),
            popup: false,
        }
    }

    pub fn select_previous(&mut self) {
        self.view.select_previous();
    }

    pub fn select_next(&mut self) {
        self.view.select_next();
    }

    pub fn cycle_sorting_criteria(&mut self, direction: CycleDirection) {
        self.sorting_criteria_cycle.cycle(direction);
        self.view
            .set_sorting_criteria(*self.sorting_criteria_cycle.selection());
    }

    pub fn selection(&self) -> Option<Entity> {
        self.view.selection()
    }
}

impl View for InventoryView {
    fn view(&mut self, model: &GosClientModel, frame: &mut Frame) {
        // Split the screen and show both widgets
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(10), Constraint::Fill(1)].as_ref())
            .split(frame.area());

        // Render the left ASCII art
        let header_ascii_paragraph = Paragraph::new(stonkpack_ascii())
            .alignment(Alignment::Right)
            .style(Style::default().fg(Color::Cyan));
        frame.render_widget(header_ascii_paragraph, chunks[0]);

        let player = player_entity(model.world());
        self.view.update(model.world(), player);
        let rows = self.view.to_rows();

        // Columns widths are constrained in the same way as Layout...
        let widths = [
            Constraint::Fill(1),
            Constraint::Length(8),
            Constraint::Length(8),
        ];

        let header_titles = ["Name", "Weight", "Value"];
        let header_cells: Vec<Cell> = header_titles
            .iter()
            .enumerate()
            .map(|(i, title)| {
                if i == self.sorting_criteria_cycle.selection_index() {
                    Cell::from(*title).style(Style::new().white().bold().reversed())
                } else {
                    Cell::from(*title).style(Style::new().white().bold())
                }
            })
            .collect();

        let header = Row::new(header_cells);

        let table = Table::new(rows, widths)
            .column_spacing(1)
            .style(Style::new().cyan())
            .header(header.bottom_margin(1))
            .footer(Row::new(vec!["Sponsored by: Oooh²™ - fresh like a breeze"]))
            .block(
                Block::new()
                    .borders(Borders::ALL)
                    .title("Charn [Inventory]"),
            )
            .row_highlight_style(Style::new().reversed());

        let mut table_state = TableState::new();
        table_state.select(self.view.selection_index());
        frame.render_stateful_widget(table, chunks[1], &mut table_state);
    }
}

fn player_entity(world: &World) -> Entity {
    let mut entity = None;

    world
        .query::<()>()
        .with(PlayerTag)
        .build()
        .each_entity(|e, ()| {
            assert!(entity.is_none());
            entity = Some(*e);
        });

    entity.unwrap()
}

fn stonkpack_ascii() -> &'static str {
    // Source: https://patorjk.com/software/taag/#p=display&f=Ivrit&t=STONKPACK
    let ascii_art = r#"
  ____ _____ ___  _   _ _  ______   _    ____ _  __
 / ___|_   _/ _ \| \ | | |/ /  _ \ / \  / ___| |/ /
 \___ \ | || | | |  \| | ' /| |_) / _ \| |   | ' / 
  ___) || || |_| | |\  | . \|  __/ ___ \ |___| . \ 
 |____/ |_| \___/|_| \_|_|\_\_| /_/   \_\____|_|\_\
                                                                                                     "#;
    ascii_art
}

struct InventoryLine {
    name: String,
    weight: Weight,
    value: f64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InventorySortingCriteria {
    Name,
    Weight,
    Value,
}

struct InventoryViewTable {
    items: HashMap<Entity, InventoryLine>,
    sorting: Vec<Entity>,
    selection: Option<Entity>,
    selection_index: Option<usize>,
    criteria: InventorySortingCriteria,
}

impl InventoryViewTable {
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
            sorting: Vec::new(),
            selection: None,
            selection_index: None,
            criteria: InventorySortingCriteria::Name,
        }
    }

    pub fn update(&mut self, world: &World, owner: Entity) {
        self.items.clear();
        world
            .query::<(&Weight,)>()
            .with(ItemTag)
            .related(This, ContainedBy, "$container")
            .related(owner, HasInventory, "$container")
            .build()
            .each_entity(|entity, (&weight,)| {
                self.items.insert(
                    *entity,
                    InventoryLine {
                        name: entity.name(),
                        weight,
                        value: 0.,
                    },
                );
            });

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

    pub fn set_sorting_criteria(&mut self, criteria: InventorySortingCriteria) {
        if criteria != self.criteria {
            self.criteria = criteria;
            self.sort_by_impl()
        }
    }

    fn sort_by_impl(&mut self) {
        match self.criteria {
            InventorySortingCriteria::Name => self
                .sorting
                .sort_by(|a, b| self.items[a].name.cmp(&self.items[b].name)),
            InventorySortingCriteria::Weight => self.sorting.sort_by(|a, b| {
                self.items[a]
                    .weight
                    .partial_cmp(&self.items[b].weight)
                    .unwrap()
            }),
            InventorySortingCriteria::Value => self.sorting.sort_by(|a, b| {
                self.items[a]
                    .value
                    .partial_cmp(&self.items[b].value)
                    .unwrap()
            }),
        }

        self.update_selection_index();
    }

    fn update_selection_index(&mut self) {
        self.selection_index = self
            .selection
            .and_then(|selection| self.sorting.iter().position(|&x| x == selection));
    }

    pub fn selection(&self) -> Option<Entity> {
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

    pub fn to_rows(&self) -> Vec<Row> {
        self.sorting
            .iter()
            .map(|e| {
                let item = &self.items[e];
                Row::new(vec![
                    item.name.clone(),
                    format!("{:0.02}", *item.weight),
                    format!("${:0.}", item.value),
                ])
            })
            .collect()
    }
}
