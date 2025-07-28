use crate::{EntityListView, GosClientModel, View};
use flecs_ecs::{
    core::{Builder, QueryAPI},
    prelude::World,
};
use gosim::{BloodStats, BloodVessel, BodyPart, PipeFlowState, PipeStats};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::{Alignment, Stylize},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListState, Paragraph, Row, Table},
    Frame,
};

pub struct DevView {
    main: ListState,

    sub_blood_vessels: BloodVesselDevView,
}

impl DevView {
    pub fn new() -> Self {
        Self {
            main: ListState::default(),
            sub_blood_vessels: BloodVesselDevView::default(),
        }
    }

    pub fn select_previous(&mut self) {
        self.main.select_previous()
    }

    pub fn select_next(&mut self) {
        self.main.select_next()
    }
}

impl View for DevView {
    fn view(&mut self, model: &GosClientModel, frame: &mut Frame) {
        // Split the screen and show both widgets
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(&[Constraint::Length(1), Constraint::Fill(1)])
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

        // Main view to select sub view
        let main_list_items = vec!["Blood Vessels"];
        let main_list = List::new(main_list_items)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(Style::default().reversed());
        frame.render_stateful_widget(main_list, chunks[0], &mut self.main);

        // Sub view
        let ((header_layout, header_row), details_rows): (_, Vec<Row>) = match self.main.selected()
        {
            Some(0) => {
                self.sub_blood_vessels.update(model.world());
                (
                    self.sub_blood_vessels.header_row(),
                    self.sub_blood_vessels.to_rows(),
                )
            }
            _ => ((vec![], Row::new::<Vec<&str>>(vec![])), Vec::new()),
        };

        let details_table = Table::new(details_rows, header_layout)
            .column_spacing(1)
            .style(Style::new().cyan())
            .header(header_row)
            .block(Block::new().borders(Borders::ALL))
            .row_highlight_style(Style::new().reversed());
        frame.render_widget(details_table, chunks[1]);
    }
}

fn header_ascii() -> &'static str {
    // Source: https://patorjk.com/software/taag/#p=display&f=Ivrit&t=i%20LIVE
    let ascii_art = r#"DEV MODE"#;
    ascii_art
}

#[derive(Default)]
struct BloodVesselDevView {
    list: EntityListView<(
        Option<BodyPart>,
        BloodStats,
        BloodVessel,
        PipeStats,
        PipeFlowState,
    )>,
}

impl BloodVesselDevView {
    pub fn update(&mut self, world: &World) {
        let mut items = Vec::new();
        world
            .query::<(
                Option<&BodyPart>,
                &BloodStats,
                &BloodVessel,
                &PipeStats,
                &PipeFlowState,
            )>()
            .build()
            .each_entity(|e, (a1, a2, a3, a4, a5)| {
                items.push((
                    *e,
                    e.name(),
                    (a1.cloned(), a2.clone(), a3.clone(), a4.clone(), a5.clone()),
                ));
            });
        self.list.update(items.into_iter());
    }

    pub fn to_rows(&self) -> Vec<Row> {
        self.list
            .to_rows(|_e, l, (part, blood, pipe, stats, state)| {
                Row::new(vec![
                    l.to_string(),
                    part.as_ref()
                        .map_or_else(|| String::new(), |x| format!("{x:?}")),
                    format!("{:4.0} mmHg", state.pressure() / 133.322),
                    format!("{:3.0} mL", 1000.0 * pipe.volume()),
                    format!("{:3.0} mL", 1000.0 * stats.nominal_volume()),
                    format!("{:3.0} %", 100. * blood.so2),
                    format!("{:5.0} mmHg", blood.po2),
                ])
            })
    }

    pub fn header_row(&self) -> (Vec<Constraint>, Row) {
        (
            vec![
                Constraint::Fill(2),
                Constraint::Fill(1),
                Constraint::Fill(1),
                Constraint::Fill(1),
                Constraint::Fill(1),
                Constraint::Fill(1),
                Constraint::Fill(1),
            ],
            Row::new(vec!["Name", "Part", "P", "V", "V0", "SO2", "PO2"]),
        )
    }
}
