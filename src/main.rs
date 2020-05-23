mod git;
mod util;
mod ui;

use git2::Repository;
use git2::RepositoryState;
use std::env::current_dir;

use git::{BranchRecord, extract_local_branches};
use ui::{BranchTable, render_branch_selection};

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

    if repo.state() != RepositoryState::Clean {
        println!("Repository is not in a clean state (in the middle of a merge?), aborting");
        return
    };

    let mut records = extract_local_branches(&repo);

    records.sort_by(|a, b| b.time_seconds.cmp(&a.time_seconds));

    for rec in &records {
        println!("{}", rec);
    };

    let mut branch_table = BranchTable::new(&records);

    match render_branch_selection(&mut branch_table) {
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
