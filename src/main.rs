mod git;
mod ui;
mod util;

use git2::Repository;
use git2::RepositoryState;
use std::process::exit;

use git::{checkout_branch, extract_local_branches, BranchRecord};
use ui::{render_branch_selection, BranchTable};

fn handle_selected_branch(repo: &Repository, branch_record: Option<&BranchRecord>) {
    match branch_record {
        Some(branch_record) => {
            if branch_record.is_current_branch {
                println!("Already on '{}'", branch_record.name);
                return;
            }

            println!("Switching to branch '{}'", branch_record.name);
            if let Err(e) = checkout_branch(repo, branch_record) {
                println!("Failed to checkout branch: {e}");
                println!("Please commit your changes or stash them before you switch branches.");
                exit(1);
            };
        }
        _ => println!("Nothing to do"),
    }
}

fn main() {
    let repo = match Repository::discover(".") {
        Ok(repo) => repo,
        Err(e) => panic!("failed to open repo: {}", e),
    };

    if repo.state() != RepositoryState::Clean {
        println!("Repository is not in a clean state (in the middle of a merge?), aborting");
        exit(1);
    };

    let mut records = extract_local_branches(&repo);
    records.sort_by(|a, b| b.time_seconds.cmp(&a.time_seconds));
    records.truncate(50);

    let mut branch_table = BranchTable::new(&records);

    match render_branch_selection(&mut branch_table) {
        Ok(res) => handle_selected_branch(&repo, res),
        Err(e) => {
            println!("error rendering branch selection: {e}");
            exit(1);
        }
    };
}
