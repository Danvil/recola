use crate::{EntityListView, GosClientModel, View};
use flecs_ecs::{
    core::{Builder, QueryAPI},
    prelude::flecs,
};
use gosim::{BodyPart, FlecsQueryRelationHelpers, HeartStats, PlayerTag, PumpStats, TissueStats};
use num_traits::cast::ToPrimitive;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::{Alignment, Stylize},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Row, Table},
    Frame,
};

pub struct BodyView {
    items: EntityListView<OrganItem>,
}

impl BodyView {
    pub fn new() -> Self {
        Self {
            items: EntityListView::new(),
        }
    }

    pub fn select_previous(&mut self) {
        self.items.select_previous()
    }

    pub fn select_next(&mut self) {
        self.items.select_next()
    }
}

pub struct OrganItem {
    stats: Vec<(String, Line<'static>)>,
}

impl View for BodyView {
    fn view(&mut self, model: &GosClientModel, frame: &mut Frame) {
        // Split the screen and show both widgets
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(&[Constraint::Length(10), Constraint::Fill(1)])
            .split(frame.area());

        // Render the left ASCII art
        let header_ascii_paragraph = Paragraph::new(header_ascii())
            .alignment(Alignment::Right)
            .style(Style::default().fg(Color::Cyan));
        frame.render_widget(header_ascii_paragraph, chunks[0]);

        // Split into overview and detail view
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(&[Constraint::Fill(1), Constraint::Fill(2)])
            .split(chunks[1]);

        // Overview view

        // Query all organs of the player
        let mut items = Vec::new();
        model
            .world()
            .query::<(&BodyPart,)>()
            .related("$this", flecs::ChildOf, "$player")
            .tagged("$player", PlayerTag)
            .build()
            .each_entity(|e, (part,)| {
                let stats = match part {
                    BodyPart::Heart => {
                        let hstats = e.cloned::<&HeartStats>();
                        let pstats = e.cloned::<&PumpStats>();
                        let tissue = e.cloned::<&TissueStats>();
                        vec![
                            (
                                "Heart Rate".into(),
                                Line::from(format!("{:.0} bpm", 60. * hstats.heart_rate.value())),
                            ),
                            (
                                "Heart Beat".into(),
                                heart_beat_list_ascii(
                                    hstats.monitor.as_slice(),
                                    hstats.monitor.latest_index(),
                                ),
                            ),
                            (
                                "Blood Flow".into(),
                                Line::from(format!(
                                    "{:4.0} ml/s",
                                    1000. /* * pstats.flow.value() */
                                )),
                            ),
                            (
                                "Tissue O2 saturation".into(),
                                Line::from(format!(
                                    "{:2.0} %",
                                    100. * tissue.o2_saturation.to_f32().unwrap()
                                )),
                            ),
                        ]
                    }
                    _ => vec![],
                };
                items.push((*e, format!("{part:?}"), OrganItem { stats }));
            });
        self.items.update(items.into_iter());

        let mut overview_list_items = Vec::new();
        for (_entity, label, _value) in self.items.iter_sorted() {
            overview_list_items.push(ListItem::new(label));
        }

        let overview_list = List::new(overview_list_items)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(Style::default().reversed());

        let mut state = ListState::default();
        state.select(self.items.selection_index());
        frame.render_stateful_widget(overview_list, chunks[0], &mut state);

        // Detail view
        let details_rows: Vec<Row> = match self.items.selection() {
            Some((_entity, _label, value)) => value
                .stats
                .iter()
                .map(|(k, v)| Row::new(vec![Line::from(k.as_str()), v.clone()]))
                .collect(),
            None => Vec::new(),
        };

        let details_table = Table::new(details_rows, [Constraint::Fill(1), Constraint::Fill(3)])
            .column_spacing(1)
            .style(Style::new().cyan())
            // .header(header.bottom_margin(1))
            .block(Block::new().borders(Borders::ALL))
            .row_highlight_style(Style::new().reversed());
        frame.render_widget(details_table, chunks[1]);
    }
}

fn header_ascii() -> &'static str {
    // Source: https://patorjk.com/software/taag/#p=display&f=Ivrit&t=i%20LIVE
    let ascii_art = r#"
  _   _     _____     _______ 
 (_) | |   |_ _\ \   / / ____|
 | | | |    | | \ \ / /|  _|  
 | | | |___ | |  \ V / | |___ 
 |_| |_____|___|  \_/  |_____|"#;
    ascii_art
}

fn heart_beat_list_ascii(beat: &[bool], latest_index: usize) -> Line<'static> {
    let len = beat.len();
    assert!(latest_index < len, "latest_index out of bounds");

    let min_brightness = 20;
    let max_brightness = 255;
    let max_age = len - 1;

    let spans: Vec<_> = (0..len)
        .map(|i| {
            // Age: 0 for latest, increasing going backward, wrapping forward as oldest
            let age = (latest_index + len - i) % len;

            // Linear fade from max to min brightness
            let t = age as f32 / max_age as f32;
            let brightness = ((1.0 - t).powi(2) * (max_brightness - min_brightness) as f32
                + min_brightness as f32)
                .round() as u8;

            let color = Color::Rgb(brightness, brightness, brightness);
            let ch = if beat[i] { '^' } else { '.' };

            Span::styled(ch.to_string(), Style::default().fg(color))
        })
        .collect();

    Line::from(spans)
}

// fn heart_beat_list_ascii(beat: &[bool], latest_index: usize) -> Text {
//     beat.iter().map(|&b| if b { '^' } else { '.' }).collect()
// }
