use git2::Repository;
use git2::BranchType;
use git2::Branch;

struct BranchRecord {
    name: String,
    commit_sha: String,
    time_seconds: i64,
    summary: String,
}

fn parse_local_branch(branch: &Branch) -> Option<BranchRecord> {

    let mut is_valid = true;

    let mut branch_name = String::from("unknown");
    let mut commit_sha = String::from("unknown");
    let mut time_seconds = 0;
    let mut summary = String::from("unknown");

    match branch.name() {
        Ok(name) => if let Some(name) = name {
            println!("branch name: {}", name);
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
            println!("commit: {} {}", commit.id(), commit.time().seconds());
            if let Some(s) = commit.summary() {
                println!("{}", s);
                summary = s.to_string();
            }
        },
        Err(e) => {
            println!("error getting commit: {}", e);
            is_valid = false;
        },
    }
    println!("--------------------");

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

fn main() {

    let repo = match Repository::open("/home/liauys/Code/test-repo") {
        Ok(repo) => repo,
        Err(e) => panic!("failed to init: {}", e),
    };

    let records = extract_local_branches(&repo);

}
