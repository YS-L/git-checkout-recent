use git2::Repository;
use git2::BranchType;
use git2::Branch;
use std::fmt;

mod util;
use crate::util::event::{Event, Events};
use std::{error::Error, io};
use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Row, Table, TableState},
    Terminal,
};

pub struct StatefulTable<'a> {
    state: TableState,
    items: &'a Vec<Vec<&'a str>>,
}

impl<'a> StatefulTable<'a> {
    fn new(data: &'a Vec<Vec<&'a str>>) -> StatefulTable<'a> {
        StatefulTable {
            state: TableState::default(),
            items: data,
        }
    }
    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

struct BranchRecord {
    name: String,
    commit_sha: String,
    time_seconds: i64,
    summary: String,
}

impl fmt::Display for BranchRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Branch({}, {}, {})", self.name, self.commit_sha, self.summary)
    }
}

fn get_table_data_from_branch_records(records: &Vec<BranchRecord>) -> (Vec<Vec<&str>>, Vec<&str>) {
    let mut data = vec![];
    let header = vec!["Branch", "Last Commit", "Summary"];
    for r in records {
        let row = vec![
            r.name.as_str(),
            r.commit_sha.as_str(),
            r.summary.as_str(),
        ];
        data.push(row);
    }
    return (data, header);
}

fn parse_local_branch(branch: &Branch) -> Option<BranchRecord> {

    let mut is_valid = true;

    let mut branch_name = String::from("unknown");
    let mut commit_sha = String::from("unknown");
    let mut time_seconds = 0;
    let mut summary = String::from("unknown");

    match branch.name() {
        Ok(name) => if let Some(name) = name {
            branch_name = name.to_string();
        },
        Err(e) => {
            println!("branch name error: {}", e);
            is_valid = false;
        },
    };

    let reference = branch.get();
    match reference.peel_to_commit() {
        Ok(commit) => {
            commit_sha = commit.id().to_string();
            time_seconds = commit.time().seconds();
            if let Some(s) = commit.summary() {
                summary = s.to_string();
            }
        },
        Err(e) => {
            println!("error getting commit: {}", e);
            is_valid = false;
        },
    }

    if is_valid {
        let record = BranchRecord {
            name: branch_name,
            commit_sha: commit_sha,
            time_seconds: time_seconds,
            summary: summary,
        };
        return Some(record);
    } else {
        return None;
    }

}

fn extract_local_branches(repo: &Repository) -> Vec<BranchRecord> {

    let mut records: Vec<BranchRecord> = Vec::new();

    match repo.branches(Some(BranchType::Local)) {
        Ok(branches) => for branch in branches {
            match branch {
                Ok((branch, _)) => {
                    if let Some(record) = parse_local_branch(&branch) {
                        records.push(record)
                    }
                }
                Err(e) => println!("error in branch: {}", e),
            }
        }
        Err(e) => panic!("failed to get branches: {}", e),
    };

    records
}

fn render_branch_selection(records: &Vec<BranchRecord>) -> Result<Option<&BranchRecord>, Box<dyn Error>> {
    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let events = Events::new();

    let (table_data, header) = get_table_data_from_branch_records(&records);
    let mut table = StatefulTable::new(&table_data);

    let mut selected = None;

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
            let t = Table::new(header.iter(), rows)
                .block(Block::default().borders(Borders::ALL).title("Table"))
                .highlight_style(selected_style)
                .highlight_symbol(">> ")
                .widths(&[
                    Constraint::Percentage(50),
                    Constraint::Length(30),
                    Constraint::Max(10),
                ]);
            f.render_stateful_widget(t, rects[0], &mut table.state);
        })?;

        if let Event::Input(key) = events.next()? {
            match key {
                Key::Char('q') => {
                    break;
                }
                Key::Down => {
                    table.next();
                }
                Key::Up => {
                    table.previous();
                }
                Key::Char('\n') => {
                    selected = table.state.selected();
                    break;
                }
                _ => {}
            }
        };
    }

    match selected {
        Some(row) => return Ok(Some(records.get(row).unwrap())),
        _ => return Ok(None),
    };
}

fn main() {

    let repo = match Repository::open("/home/liauys/Code/test-repo") {
        Ok(repo) => repo,
        Err(e) => panic!("failed to init: {}", e),
    };

    let mut records = extract_local_branches(&repo);

    records.sort_by(|a, b| b.time_seconds.cmp(&a.time_seconds));

    for rec in &records {
        println!("{}", rec);
    };

    match render_branch_selection(&records) {
        Ok(res) => match(res) {
            Some(branch_record) => println!("Selected branch: {}", branch_record.name),
            _ => println!("Selected nothing"),
        },
        Err(e) => println!("error getting input: {}", e),
    };
}
