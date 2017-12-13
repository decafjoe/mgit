use git2;
use git2::{Error, Repository, Status, StatusOptions, StatusShow};

pub struct Worktree<'a> {
    repo: &'a Repository,
}

impl<'a> Worktree<'a> {
    pub fn new(repo: &'a Repository) -> Worktree<'a> {
        Worktree{ repo: repo }
    }

    fn status_options(&self) -> StatusOptions {
        let mut s = StatusOptions::new();
        s.exclude_submodules(true);
        s.renames_head_to_index(true);
        s.renames_index_to_workdir(true);
        s.renames_from_rewrites(true);
        s
    }

    fn filter(&self, s: &mut StatusOptions, f: Status)
              -> Result<usize, Error> {
        let statuses = self.repo.statuses(Some(s))?;
        Ok(statuses.iter().filter(|e| e.status().intersects(f)).count())
    }

    pub fn uncommitted(&self) -> Result<usize, Error> {
        let mut s = self.status_options();
        s.show(StatusShow::Index);
        Ok(self.repo.statuses(Some(&mut s))?.len())
    }

    pub fn modified(&self) -> Result<usize, Error> {
        let mut s = self.status_options();
        s.show(StatusShow::Workdir);
        let flags = git2::STATUS_WT_DELETED
            | git2::STATUS_WT_MODIFIED
            | git2::STATUS_WT_RENAMED
            | git2::STATUS_WT_TYPECHANGE;
        self.filter(&mut s, flags)
    }

    pub fn untracked(&self) -> Result<usize, Error> {
        let mut s = self.status_options();
        s.show(StatusShow::Workdir);
        s.include_untracked(true);
        s.recurse_untracked_dirs(true);
        self.filter(&mut s, git2::STATUS_WT_NEW)
    }

    pub fn is_dirty(&self) -> Result<bool, Error> {
        let mut s = self.status_options();
        s.show(StatusShow::IndexAndWorkdir);
        s.include_untracked(true);
        Ok(self.repo.statuses(Some(&mut s))?.len() > 0)
    }
}
