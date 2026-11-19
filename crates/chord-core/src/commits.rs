//! A worktree's commit of its staged files, or its amend of the last commit, and the push of its branch: who a commit is by, where a push goes and whether it has anything to push, and the push that sets the upstream the first time.

use std::path::Path;

use serde::Serialize;
use specta::Type;

use crate::{Error, git, git_counts};

#[derive(Serialize, Type, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommitTarget {
    /// "Name <email>", as a commit records it.
    pub author: Option<String>,
    /// The remote branch a push goes to; None on a detached HEAD, a branch with no commit yet or without a remote.
    pub push_to: Option<String>,
    /// Whether the branch has commits `push_to` lacks, or no upstream.
    pub pushable: bool,
    /// Whether HEAD has a commit to amend.
    pub amendable: bool,
}

/// Commits the staged files with the message, or with `amend` replaces the last commit; a hook that rejects it fails with its output.
pub fn commit(dir: &Path, message: &str, amend: bool) -> Result<(), Error> {
    let mut args = vec!["commit", "-q", "-F", "-"];
    if amend {
        args.push("--amend");
    }
    git::run_with_input(dir, &args, message.as_bytes()).map(drop)
}

/// The last commit's message, which an amend starts from.
pub fn last_message(dir: &Path) -> Result<String, Error> {
    Ok(git::run(dir, &["log", "-1", "--format=%B"])?
        .trim_end()
        .to_string())
}

/// The remote a push goes to: `origin`, else the repository's only remote.
fn remote(dir: &Path) -> Result<Option<String>, Error> {
    let output = git::run(dir, &["remote"])?;
    let remotes: Vec<&str> = output.lines().collect();
    Ok(match remotes.as_slice() {
        [only] => Some(only.to_string()),
        _ => remotes.contains(&"origin").then(|| "origin".to_string()),
    })
}

struct Branch {
    /// None on a detached HEAD.
    name: Option<String>,
    /// False on a branch with no commit yet.
    born: bool,
    /// The remote of an upstream with the branch's own name; another upstream counts as none, since `git push` refuses it.
    upstream_remote: Option<String>,
    /// None without such an upstream, or with one that is gone from the remote.
    ahead: Option<u32>,
}

fn branch(dir: &Path) -> Result<Branch, Error> {
    let status = git::run(
        dir,
        &[
            "--no-optional-locks",
            "status",
            "--porcelain=v2",
            "--branch",
            "--untracked-files=no",
        ],
    )?;
    let header = |key: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix("# ")?.strip_prefix(key))
            .map(str::to_string)
    };
    let name = header("branch.head ").filter(|name| name != "(detached)");
    let upstream_remote = name.as_deref().and_then(|name| {
        header("branch.upstream ")?
            .strip_suffix(&format!("/{name}"))
            .map(str::to_string)
    });
    Ok(Branch {
        born: header("branch.oid ").is_some_and(|oid| oid != "(initial)"),
        ahead: upstream_remote
            .is_some()
            .then(|| git_counts::parse_status(&status).0.map(|(_, ahead)| ahead))
            .flatten(),
        upstream_remote,
        name,
    })
}

pub fn target(dir: &Path) -> Result<CommitTarget, Error> {
    let branch = branch(dir)?;
    let push_to = match (&branch.name, branch.born) {
        (Some(name), true) => match branch.upstream_remote.clone() {
            Some(remote) => Some(remote),
            None => remote(dir)?,
        }
        .map(|remote| format!("{remote}/{name}")),
        _ => None,
    };
    // `git var` gives "Name <email> time zone", or fails while git has no identity.
    let author = git::run(dir, &["var", "GIT_AUTHOR_IDENT"])
        .ok()
        .and_then(|ident| ident.trim_end().rsplitn(3, ' ').nth(2).map(str::to_string));
    Ok(CommitTarget {
        author,
        pushable: push_to.is_some() && branch.ahead.is_none_or(|ahead| ahead > 0),
        push_to,
        amendable: branch.born,
    })
}

/// Pushes the branch to its own name on the remote, which sets the upstream the first time.
pub fn push(dir: &Path) -> Result<(), Error> {
    let remote = match branch(dir)?.upstream_remote {
        Some(remote) => remote,
        None => remote(dir)?
            .ok_or_else(|| Error::Message("This repository has no remote to push to.".into()))?,
    };
    git::run(dir, &["push", "-q", "-u", &remote, "HEAD"]).map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_shows_until_the_upstream_has_every_commit() -> Result<(), Error> {
        let root = std::env::temp_dir().join(format!("chord-push-{}", std::process::id()));
        let result = push_in(&root);
        let _ = std::fs::remove_dir_all(&root);
        result
    }

    fn push_in(root: &Path) -> Result<(), Error> {
        let (remote, repo) = (root.join("remote.git"), root.join("app"));
        std::fs::create_dir_all(&remote)?;
        std::fs::create_dir_all(&repo)?;
        git::run(&remote, &["init", "-q", "--bare"])?;
        git::run(&repo, &["init", "-q", "-b", "main"])?;
        git::run(&repo, &["config", "user.name", "T Tester"])?;
        git::run(&repo, &["config", "user.email", "t@t"])?;
        git::run(&repo, &["config", "commit.gpgsign", "false"])?;
        assert!(!target(&repo)?.amendable);
        std::fs::write(repo.join("f"), "1\n")?;
        git::run(&repo, &["add", "f"])?;
        commit(&repo, "first", false)?;
        let alone = target(&repo)?;
        assert_eq!(alone.author.as_deref(), Some("T Tester <t@t>"));
        assert_eq!((alone.push_to, alone.pushable), (None, false));

        git::run(
            &repo,
            &["remote", "add", "up", remote.to_str().unwrap_or_default()],
        )?;
        let first = target(&repo)?;
        assert_eq!(
            (first.push_to.as_deref(), first.pushable),
            (Some("up/main"), true)
        );
        push(&repo)?;
        assert!(!target(&repo)?.pushable);

        std::fs::write(repo.join("f"), "2\n")?;
        git::run(&repo, &["add", "f"])?;
        commit(&repo, "second", false)?;
        assert!(target(&repo)?.pushable);
        commit(&repo, "second, amended", true)?;
        assert_eq!(last_message(&repo)?, "second, amended");
        push(&repo)?;
        assert!(!target(&repo)?.pushable);

        // A branch tracking another branch's name pushes to its own.
        git::run(
            &repo,
            &["checkout", "-q", "-b", "feat", "--track", "up/main"],
        )?;
        let feat = target(&repo)?;
        assert_eq!(
            (feat.push_to.as_deref(), feat.pushable),
            (Some("up/feat"), true)
        );
        push(&repo)?;
        // A second remote, neither one origin, leaves the upstream's remote in place.
        git::run(&repo, &["remote", "add", "other", "/nowhere"])?;
        let tracked = target(&repo)?;
        assert_eq!(
            (tracked.push_to.as_deref(), tracked.pushable),
            (Some("up/feat"), false)
        );
        Ok(())
    }
}
