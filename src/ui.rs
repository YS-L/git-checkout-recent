use std::{error::Error, io};
use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Row, Table, TableState},
    Terminal,
};

use super::git::BranchRecord;
use super::util::event::{Event, Events};

pub struct BranchTable<'a> {
    state: TableState,
    items: Vec<Vec<String>>,
    header: Vec<String>,
    records: &'a [BranchRecord],
}

impl<'a> BranchTable<'a> {
    pub fn new(records: &'a [BranchRecord]) -> BranchTable<'a> {
        let (data, header) = get_table_data_from_branch_records(&records);
        BranchTable {
            state: TableState::default(),
            items: data,
            header,
            records,
        }
    }

    pub fn init(&mut self) {
        self.state.select(Some(0));
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i + 3 >= self.items.len() {
                    i
                } else {
                    i + 3
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i < 3 {
                    i
                } else {
                    i - 3
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn deselect(&mut self) {
        self.state.select(None);
    }

    pub fn selected_record(&mut self) -> Option<&BranchRecord> {
        match self.state.selected() {
            Some(row) => self.records.get(row / 3),
            _ => None,
        }
    }
}

fn get_table_data_from_branch_records(records: &[BranchRecord]) -> (Vec<Vec<String>>, Vec<String>) {
    let mut data = vec![];
    let header = vec![String::from("Name"), String::from("Last Commit")];
    for r in records {
        let mut name = r.name.clone();
        if r.is_current_branch {
            name = String::from("* ") + &name;
        }
        let commit_info = format!(
            "{} ({}) {}",
            &r.commit_sha[..8],
            r.pretty_format_date(),
            r.author_name
        );
        let row = vec![name, commit_info.clone()];
        data.push(row);
        let row = vec![String::from(""), r.summary.clone()];
        data.push(row);
        let row = vec![String::from(""), String::from("")];
        data.push(row);
    }
    (data, header)
}

pub fn render_branch_selection<'a>(
    table: &'a mut BranchTable,
) -> Result<Option<&'a BranchRecord>, Box<dyn Error>> {
    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let events = Events::new();

    table.init();

    // Input
    loop {
        terminal.draw(|mut f| {
            let rects = Layout::default()
                .constraints([Constraint::Percentage(100)].as_ref())
                .margin(5)
                .split(f.size());

            let selected_style = Style::default().fg(Color::Yellow).modifier(Modifier::BOLD);
            let normal_style = Style::default().fg(Color::White);
            let rows = table
                .items
                .iter()
                .map(|i| Row::StyledData(i.iter(), normal_style));
            let t = Table::new(table.header.iter(), rows)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Recent branches"),
                )
                .highlight_style(selected_style)
                .highlight_symbol(">> ")
                .widths(&[Constraint::Percentage(20), Constraint::Percentage(80)]);
            f.render_stateful_widget(t, rects[0], &mut table.state);
        })?;

        if let Event::Input(key) = events.next()? {
            match key {
                Key::Char('q') => {
                    table.deselect();
                }
                Key::Down => {
                    table.next();
                }
                Key::Up => {
                    table.previous();
                }
                Key::Char('\n') => {
                    break;
                }
                _ => {}
            }
        };
    }

    Ok(table.selected_record())
}
