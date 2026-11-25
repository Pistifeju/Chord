use std::path::Path;

use serde::Serialize;
use specta::Type;

use crate::{Error, git, git_counts};

#[derive(Serialize, Type, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommitTarget {
    pub author: Option<String>,
    pub push_to: Option<String>,
    pub pushable: bool,
    pub ahead: Option<u32>,
    pub behind: Option<u32>,
    pub fetched: Option<f64>,
    pub amendable: bool,
}

pub fn commit(dir: &Path, message: &str, amend: bool) -> Result<(), Error> {
    let mut args = vec!["commit", "-q", "-F", "-"];
    if amend {
        args.push("--amend");
    }
    git::run_with_input(dir, &args, message.as_bytes()).map(drop)
}

pub fn last_message(dir: &Path) -> Result<String, Error> {
    Ok(git::run(dir, &["log", "-1", "--format=%B"])?
        .trim_end()
        .to_string())
}

fn remote(dir: &Path) -> Result<Option<String>, Error> {
    let output = git::run(dir, &["remote"])?;
    let remotes: Vec<&str> = output.lines().collect();
    Ok(match remotes.as_slice() {
        [only] => Some(only.to_string()),
        _ => remotes.contains(&"origin").then(|| "origin".to_string()),
    })
}

struct Branch {
    name: Option<String>,
    born: bool,
    upstream_remote: Option<String>,
    sides: Option<(u32, u32)>,
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
        sides: upstream_remote
            .is_some()
            .then(|| git_counts::parse_status(&status).0)
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
    let author = git::run(dir, &["var", "GIT_AUTHOR_IDENT"])
        .ok()
        .and_then(|ident| ident.trim_end().rsplitn(3, ' ').nth(2).map(str::to_string));
    Ok(CommitTarget {
        author,
        pushable: push_to.is_some() && branch.sides.is_none_or(|(_, ahead)| ahead > 0),
        push_to,
        ahead: branch.sides.map(|(_, ahead)| ahead),
        behind: branch.sides.map(|(behind, _)| behind),
        fetched: fetched(dir),
        amendable: branch.born,
    })
}

fn fetched(dir: &Path) -> Option<f64> {
    let path = git::read(dir, &["rev-parse", "--git-path", "FETCH_HEAD"]).ok()?;
    let modified = std::fs::metadata(dir.join(path.trim())).ok()?.modified();
    let since = modified.ok()?.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(since.as_millis() as f64)
}

fn sync_remote(dir: &Path, verb: &str) -> Result<String, Error> {
    match branch(dir)?.upstream_remote {
        Some(remote) => Ok(remote),
        None => remote(dir)?
            .ok_or_else(|| Error::Message(format!("This repository has no remote to {verb}."))),
    }
}

pub fn push(dir: &Path) -> Result<(), Error> {
    let remote = sync_remote(dir, "push to")?;
    git::run(dir, &["push", "-q", "-u", &remote, "HEAD"]).map(drop)
}

pub fn fetch(dir: &Path) -> Result<String, Error> {
    let remote = sync_remote(dir, "fetch from")?;
    git::run(dir, &["fetch", "-q", &remote])?;
    Ok(remote)
}

pub fn pull(dir: &Path) -> Result<bool, Error> {
    let unset = git::optional(git::read(dir, &["config", "--get", "pull.ff"]))?.is_none();
    let ff: &[&str] = if unset { &["--ff"] } else { &[] };
    moves_head(dir, &[&["pull", "-q", "--no-edit"], ff].concat())
}

pub fn merge(dir: &Path, branch: &str) -> Result<bool, Error> {
    moves_head(dir, &["merge", "-q", "--no-edit", branch])
}

fn moves_head(dir: &Path, args: &[&str]) -> Result<bool, Error> {
    let head = || git::optional(git::read(dir, &["rev-parse", "HEAD"]));
    let before = head()?;
    git::run(dir, args)?;
    Ok(head()? != before)
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
        let second = target(&repo)?;
        assert_eq!((second.pushable, second.ahead), (true, Some(1)));
        commit(&repo, "second, amended", true)?;
        assert_eq!(last_message(&repo)?, "second, amended");
        push(&repo)?;
        assert!(!target(&repo)?.pushable);

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
        git::run(&repo, &["remote", "add", "other", "/nowhere"])?;
        let tracked = target(&repo)?;
        assert_eq!(
            (tracked.push_to.as_deref(), tracked.pushable),
            (Some("up/feat"), false)
        );
        Ok(())
    }

    #[test]
    fn fetch_counts_the_commits_to_pull_and_pull_and_merge_tell_whether_head_moved()
    -> Result<(), Error> {
        let root = std::env::temp_dir().join(format!("chord-pull-{}", std::process::id()));
        let result = pull_in(&root);
        let _ = std::fs::remove_dir_all(&root);
        result
    }

    fn pull_in(root: &Path) -> Result<(), Error> {
        let (remote, repo, other) = (
            root.join("remote.git"),
            root.join("app"),
            root.join("other"),
        );
        std::fs::create_dir_all(&remote)?;
        git::run(&remote, &["init", "-q", "--bare", "-b", "main"])?;
        let clone = |to: &Path| -> Result<(), Error> {
            let from = remote.to_string_lossy();
            git::run(root, &["clone", "-q", &from, &to.to_string_lossy()])?;
            git::run(to, &["config", "user.name", "T Tester"])?;
            git::run(to, &["config", "user.email", "t@t"])?;
            git::run(to, &["config", "commit.gpgsign", "false"]).map(drop)
        };
        let write = |dir: &Path, file: &str, text: &str, message: &str| -> Result<(), Error> {
            std::fs::write(dir.join(file), text)?;
            git::run(dir, &["add", file])?;
            commit(dir, message, false)
        };
        clone(&other)?;
        git::run(&other, &["checkout", "-q", "-b", "main"])?;
        write(&other, "f", "1\n", "first")?;
        push(&other)?;
        clone(&repo)?;
        assert_eq!(target(&repo)?.fetched, None);
        write(&other, "f", "2\n", "second")?;
        push(&other)?;

        assert_eq!(target(&repo)?.behind, Some(0));
        assert_eq!(fetch(&repo)?, "origin");
        let fetched = target(&repo)?;
        assert_eq!(fetched.behind, Some(1));
        assert!(fetched.fetched.is_some());
        assert!(pull(&repo)?);
        assert!(!pull(&repo)?);
        assert_eq!(target(&repo)?.behind, Some(0));

        write(&other, "h", "theirs\n", "theirs")?;
        push(&other)?;
        write(&repo, "i", "ours\n", "ours")?;
        assert!(pull(&repo)?);
        assert_eq!(target(&repo)?.behind, Some(0));

        git::run(&repo, &["checkout", "-q", "-b", "side"])?;
        write(&repo, "g", "side\n", "side")?;
        git::run(&repo, &["checkout", "-q", "main"])?;
        assert!(merge(&repo, "side")?);
        assert!(!merge(&repo, "side")?);

        git::run(&repo, &["checkout", "-q", "side"])?;
        write(&repo, "f", "side\n", "side again")?;
        git::run(&repo, &["checkout", "-q", "main"])?;
        write(&repo, "f", "main\n", "main again")?;
        let stopped = merge(&repo, "side");
        assert!(matches!(stopped, Err(Error::Message(reason)) if reason.contains("CONFLICT")));
        Ok(())
    }
}
