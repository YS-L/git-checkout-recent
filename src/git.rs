use std::fmt;

use git2::Branch;
use git2::BranchType;
use git2::Repository;

use chrono::offset::FixedOffset;
use chrono::offset::TimeZone;
use chrono::NaiveDateTime;
use chrono_humanize::HumanTime;

pub struct BranchRecord {
    pub name: String,
    pub commit_sha: String,
    pub time_seconds: i64,
    pub offset_minutes: i32,
    pub summary: String,
    pub ref_name: String,
    pub author_name: String,
    pub is_current_branch: bool,
}

impl BranchRecord {
    pub fn pretty_format_date(&self) -> String {
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

fn parse_local_branch(
    branch: &Branch,
    head_branch_refname: &Option<String>,
) -> Option<BranchRecord> {
    let branch_name = branch.name().ok()??.to_string();

    let reference = branch.get();
    let ref_name = reference.name()?.to_string();

    let mut is_current_branch = false;
    if let Some(current) = head_branch_refname {
        is_current_branch = ref_name == current.as_str();
    }

    let commit = reference.peel_to_commit().ok()?;
    let commit_sha = commit.id().to_string();
    let time_seconds = commit.time().seconds();
    let offset_minutes = commit.time().offset_minutes();
    let summary = commit.summary()?.to_string();
    let author_name = commit.author().name()?.to_string();

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
}

fn get_current_branch_refname(repo: &Repository) -> Option<String> {
    if let Ok(is_detached) = repo.head_detached() {
        if is_detached {
            return None;
        }
    };
    if let Ok(head) = repo.head() {
        if let Some(name) = head.name() {
            return Some(name.to_string());
        }
    };
    None
}

pub fn extract_local_branches(repo: &Repository) -> Vec<BranchRecord> {
    let mut records: Vec<BranchRecord> = Vec::new();

    let current_branch_refname = get_current_branch_refname(repo);

    match repo.branches(Some(BranchType::Local)) {
        Ok(branches) => {
            for branch in branches {
                match branch {
                    Ok((branch, _)) => {
                        if let Some(record) = parse_local_branch(&branch, &current_branch_refname) {
                            records.push(record)
                        }
                    }
                    Err(e) => println!("error in branch: {e}"),
                }
            }
        }
        Err(e) => panic!("failed to get branches: {}", e),
    };

    records
}

pub fn checkout_branch(repo: &Repository, record: &BranchRecord) -> Result<(), git2::Error> {
    let treeish = repo.revparse_single(record.commit_sha.as_str())?;
    repo.checkout_tree(&treeish, None)?;
    repo.set_head(record.ref_name.as_str())?;
    Ok(())
}
