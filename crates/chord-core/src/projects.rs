//! Projects: the git repositories Chord shows, keyed by their main checkout, and plain folders outside any repository.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::{Error, Store, git, worktrees};

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
pub struct Project {
    pub path: PathBuf,
    pub name: String,
}

pub fn add(store: &Store, folder: &Path) -> Result<Project, Error> {
    let project = open(folder)?;
    store.update(|state| {
        state.welcomed = true;
        if !state.projects.iter().any(|p| p.path == project.path) {
            state.projects.push(project.clone());
        }
    })?;
    Ok(project)
}

pub fn remove(store: &Store, path: &Path) -> Result<(), Error> {
    store.update(|state| {
        state.welcomed = true;
        state.projects.retain(|p| p.path != path);
        state.run_destinations.remove(path);
        state.preview_models.remove(path);
        state.scheme_picks.remove(path);
        state.scheme_defaults.remove(path);
        state.worktree_order.remove(path);
    })
}

pub fn find(store: &Store, path: &Path) -> Result<Project, Error> {
    store
        .get()
        .projects
        .into_iter()
        .find(|p| p.path == path)
        .ok_or_else(|| Error::Message(format!("{} is not a Chord project", path.display())))
}

fn open(folder: &Path) -> Result<Project, Error> {
    if !git::in_repository(folder) {
        worktrees::folder_at(folder)?;
        // Canonical, since git lists real paths and the card must keep its path after `git init`.
        return Ok(named(folder.canonicalize()?));
    }
    let entries = worktrees::entries(folder).map_err(|error| match error {
        Error::Message(_) => {
            Error::Message(format!("{} is not a git repository", folder.display()))
        }
        missing => missing,
    })?;
    let main = entries
        .into_iter()
        .next()
        .ok_or_else(|| Error::Message(format!("{} has no worktree", folder.display())))?;
    if main.bare {
        return Err(Error::Message(format!(
            "{} is a bare repository. Add a folder with a checkout",
            folder.display()
        )));
    }
    Ok(named(main.worktree.path))
}

fn named(path: PathBuf) -> Project {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    Project { path, name }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    #[test]
    fn adding_a_linked_worktree_adds_its_main_checkout_once() -> Result<(), Error> {
        let root = std::env::temp_dir().join(format!("chord-projects-{}", std::process::id()));
        let repo = root.join("app");
        fs::create_dir_all(&repo)?;
        let git = |args: &[&str]| Command::new("git").arg("-C").arg(&repo).args(args).output();
        git(&["init", "-q", "-b", "main"])?;
        git(&[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "first",
        ])?;
        git(&["worktree", "add", "-q", "-b", "fix", "../app-fix"])?;
        let store = Store::open(root.join("state.json"))?;

        add(&store, &root.join("app-fix"))?;
        add(&store, &repo)?;

        let projects = store.get().projects;
        let _ = fs::remove_dir_all(&root);
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "app");
        Ok(())
    }

    #[test]
    fn a_plain_folder_is_a_project_with_one_card_until_git_init() -> Result<(), Error> {
        let root = std::env::temp_dir().join(format!("chord-plain-{}", std::process::id()));
        let folder = root.join("notes");
        fs::create_dir_all(&folder)?;
        let store = Store::open(root.join("state.json"))?;

        let project = add(&store, &folder)?;
        let before = worktrees::list(&project.path)?;
        Command::new("git")
            .arg("-C")
            .arg(&folder)
            .args(["init", "-q"])
            .output()?;
        let after = worktrees::list(&project.path)?;
        let _ = fs::remove_dir_all(&root);

        assert_eq!(project.name, "notes");
        assert_eq!(before.len(), 1);
        assert!(!before[0].git);
        assert_eq!(before[0].path, project.path);
        assert!(after[0].git);
        assert_eq!(after[0].path, project.path);
        Ok(())
    }
}
