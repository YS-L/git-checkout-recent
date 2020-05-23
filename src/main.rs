use git2::Repository;
use git2::BranchType;
use git2::Branch;
use git2::RepositoryState;
use std::fmt;

mod util;
use crate::util::event::{Event, Events};
use chrono::NaiveDateTime;
use chrono::offset::FixedOffset;
use chrono::offset::TimeZone;
use chrono_humanize::HumanTime;
use std::{error::Error, io};
use std::env::current_dir;
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
    items: &'a [Vec<String>],
}

impl<'a> StatefulTable<'a> {
    fn new(data: &'a [Vec<String>]) -> StatefulTable<'a> {
        StatefulTable {
            state: TableState::default(),
            items: data,
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
}

struct BranchRecord {
    name: String,
    commit_sha: String,
    time_seconds: i64,
    offset_minutes: i32,
    summary: String,
    ref_name: String,
    author_name: String,
    is_current_branch: bool,
}

impl BranchRecord {
    fn pretty_format_date(&self) -> String {
        let naive_dt = NaiveDateTime::from_timestamp(self.time_seconds, 0);
        let offset = FixedOffset::east(self.offset_minutes * 60);
        let dt = offset.from_utc_datetime(&naive_dt);
        let humanized_dt = HumanTime::from(dt);
        humanized_dt.to_string()
    }
}

impl fmt::Display for BranchRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Branch({}, {}, {}, {})",
            self.name,
            self.commit_sha,
            self.summary,
            self.pretty_format_date(),
        )
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
            "{} ({}) {}", &r.commit_sha[..8], r.pretty_format_date(), r.author_name
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

fn parse_local_branch(branch: &Branch, head_branch_refname: &Option<String>) -> Option<BranchRecord> {

    let mut is_valid = true;

    let mut branch_name = String::from("unknown");
    let mut commit_sha = String::from("unknown");
    let mut time_seconds = 0;
    let mut offset_minutes = 0;
    let mut summary = String::from("unknown");
    let mut author_name = String::from("unknown");
    let mut is_current_branch = false;

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

    let ref_name = reference.name()?.to_string();
    if let Some(current) = head_branch_refname {
        is_current_branch = ref_name == *current;
    }

    match reference.peel_to_commit() {
        Ok(commit) => {
            commit_sha = commit.id().to_string();
            time_seconds = commit.time().seconds();
            offset_minutes = commit.time().offset_minutes();
            if let Some(s) = commit.summary() {
                summary = s.to_string();
            }
            author_name = commit.author().name()?.to_string();
        },
        Err(e) => {
            println!("error getting commit: {}", e);
            is_valid = false;
        },
    }

    if is_valid {
        let record = BranchRecord {
            name: branch_name,
            commit_sha,
            time_seconds,
            offset_minutes,
            summary,
            ref_name,
            author_name,
            is_current_branch,
        };
        Some(record)
    } else {
        None
    }

}

fn get_current_branch_refname(repo: &Repository) -> Option<String> {
    if let Ok(is_detached) = repo.head_detached() {
        if is_detached {
            return None
        }
    };
    if let Ok(head) = repo.head() {
        if let Some(name) = head.name() {
            return Some(name.to_string())
        }
    };
    None
}

fn extract_local_branches(repo: &Repository) -> Vec<BranchRecord> {

    let mut records: Vec<BranchRecord> = Vec::new();

    let current_branch_refname = get_current_branch_refname(repo);

    match repo.branches(Some(BranchType::Local)) {
        Ok(branches) => for branch in branches {
            match branch {
                Ok((branch, _)) => {
                    if let Some(record) = parse_local_branch(&branch, &current_branch_refname) {
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

fn render_branch_selection(records: &[BranchRecord]) -> Result<Option<&BranchRecord>, Box<dyn Error>> {
    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let events = Events::new();

    // TODO: probably belong within StatefulTable
    let (table_data, header) = get_table_data_from_branch_records(&records);
    let mut table = StatefulTable::new(&table_data);

    let mut selected = None;
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
            let t = Table::new(header.iter(), rows)
                .block(Block::default().borders(Borders::ALL).title("Recent branches"))
                .highlight_style(selected_style)
                .highlight_symbol(">> ")
                .widths(&[
                    Constraint::Percentage(20),
                    Constraint::Percentage(80),
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
        // TODO: row / 3 should be refactored out of here
        Some(row) => Ok(Some(records.get(row / 3).unwrap())),
        _ => Ok(None),
    }
}

fn checkout_branch(repo: &Repository, record: &BranchRecord) -> Result<(), git2::Error> {
    let treeish = repo.revparse_single(record.commit_sha.as_str())?;
    repo.checkout_tree(&treeish, None)?;
    repo.set_head(record.ref_name.as_str())?;
    Ok(())
}

fn main() {

    let repo_dir = current_dir().expect("failed to get repo directory");

    let repo = match Repository::open(repo_dir) {
        Ok(repo) => repo,
        Err(e) => panic!("failed to open repo: {}", e),
    };

    let mut records = extract_local_branches(&repo);

    records.sort_by(|a, b| b.time_seconds.cmp(&a.time_seconds));

    for rec in &records {
        println!("{}", rec);
    };

    if repo.state() != RepositoryState::Clean {
        println!("Repository is not in a clean state (in the middle of a merge?), aborting");
        return
    };

    match render_branch_selection(&records) {
        Ok(res) => match res {
            Some(branch_record) => {

                if branch_record.is_current_branch {
                    println!("Already at branch: {}, nothing to do", branch_record.name);
                    return
                }

                println!("Checking out local branch: {}", branch_record.name);
                match checkout_branch(&repo, &branch_record) {
                    Ok(()) => println!("Done"),
                    Err(e) => {
                        println!("Failed to checkout branch: {}", e);
                        println!("Please commit your changes or stash them before you switch branches.");
                    },
                };

            },
            _ => println!("Nothing to do"),
        },
        Err(e) => println!("error rendering branch selection: {}", e),
    };
}
