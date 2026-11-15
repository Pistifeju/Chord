//! Projects: the git repositories Chord shows, keyed by their main checkout.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::{Error, Store, worktrees};

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
pub struct Project {
    pub path: PathBuf,
    pub name: String,
}

pub fn add(store: &Store, folder: &Path) -> Result<Project, Error> {
    let project = open(folder)?;
    store.update(|state| {
        if !state.projects.iter().any(|p| p.path == project.path) {
            state.projects.push(project.clone());
        }
    })?;
    Ok(project)
}

pub fn remove(store: &Store, path: &Path) -> Result<(), Error> {
    store.update(|state| {
        state.projects.retain(|p| p.path != path);
        state.run_destinations.remove(path);
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
    let path = main.worktree.path;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    Ok(Project { path, name })
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
}
