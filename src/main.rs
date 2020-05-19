use git2::Repository;
use git2::BranchType;

fn main() {

    let repo = match Repository::open("/home/liauys/Code/test-repo") {
        Ok(repo) => repo,
        Err(e) => panic!("failed to init: {}", e),
    };

    let branches = match repo.branches(Some(BranchType::Local)) {
        Ok(branches) => for branch in branches {
            match branch {
                Ok((branch, _)) => {
                    match branch.name() {
                        Ok(name) => if let Some(name) = name {
                            println!("branch name: {}", name);
                        },
                        Err(e) => println!("branch name error: {}", e),
                    }
                    let reference = branch.get();
                    match reference.peel_to_commit() {
                        Ok(commit) => {
                            println!("commit: {} {}", commit.id(), commit.time().seconds());
                            if let Some(summary) = commit.summary() {
                                println!("{}", summary);
                            }
                        },
                        Err(e) => println!("error getting commit: {}", e),
                    }
                    println!("--------------------");
                }
                Err(e) => println!("error in branch: {}", e),
            }
        }
        Err(e) => panic!("failed to get branches: {}", e),
    };
}
