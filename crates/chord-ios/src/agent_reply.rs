use std::collections::VecDeque;

use serde::Serialize;

use crate::build::{BuildEvent, BuildPhase};
use crate::issues::{BuildIssue, parse_line};

const TAIL: usize = 20;

#[derive(Serialize, Debug, PartialEq)]
pub(crate) struct AgentReply {
    pub text: String,
    pub failed: bool,
}

impl AgentReply {
    fn ok(text: &str) -> Self {
        Self {
            text: text.into(),
            failed: false,
        }
    }

    pub fn failed(text: String) -> Self {
        Self { text, failed: true }
    }
}

pub(crate) fn agent_reply(events: impl IntoIterator<Item = BuildEvent>, run: bool) -> AgentReply {
    let mut tail = VecDeque::with_capacity(TAIL);
    let mut error_lines = false;
    for event in events {
        match event {
            BuildEvent::Log { text } => {
                for line in text.lines() {
                    error_lines |= parse_line(line).is_some();
                    if tail.len() == TAIL {
                        tail.pop_front();
                    }
                    tail.push_back(line.to_string());
                }
            }
            BuildEvent::Phase {
                phase: BuildPhase::Running,
            } if run => return AgentReply::ok("Launched"),
            BuildEvent::Succeeded => return AgentReply::ok("Build succeeded"),
            BuildEvent::Exited => return AgentReply::ok("The app exited"),
            BuildEvent::Stopped => return AgentReply::ok("Stopped"),
            BuildEvent::Failed { issues, more } => {
                let lines: Vec<String> = if error_lines || tail.is_empty() {
                    issues.iter().map(issue_line).collect()
                } else {
                    tail.into()
                };
                let more = (more > 0).then(|| format!("\n{more} more errors"));
                return AgentReply::failed(format!(
                    "Build failed\n{}{}",
                    lines.join("\n"),
                    more.unwrap_or_default()
                ));
            }
            BuildEvent::Phase { .. } | BuildEvent::Console { .. } => {}
        }
    }
    AgentReply::ok("Stopped")
}

pub(crate) fn issue_line(issue: &BuildIssue) -> String {
    match (&issue.file, issue.line) {
        (Some(file), Some(line)) => format!("{file}:{line}: {}", issue.message),
        _ => issue.message.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log(text: &str) -> BuildEvent {
        BuildEvent::Log { text: text.into() }
    }

    #[test]
    fn a_failed_build_answers_with_its_error_lines() {
        let events = vec![
            log("/w/A.swift:4:2: error: bad\nerror: Signing needs a team\n"),
            BuildEvent::Failed {
                issues: vec![
                    BuildIssue {
                        file: Some("/w/A.swift".into()),
                        line: Some(4),
                        message: "bad".into(),
                    },
                    BuildIssue {
                        file: None,
                        line: None,
                        message: "Signing needs a team".into(),
                    },
                ],
                more: 3,
            },
        ];
        assert_eq!(
            agent_reply(events, false),
            AgentReply::failed(
                "Build failed\n/w/A.swift:4: bad\nSigning needs a team\n3 more errors".into()
            )
        );
    }

    #[test]
    fn a_failed_build_without_an_error_line_answers_with_the_last_twenty_log_lines() {
        let lines: String = (1..=25).map(|n| format!("line {n}\n")).collect();
        let events = vec![
            log(&lines),
            BuildEvent::Failed {
                issues: vec![BuildIssue {
                    file: None,
                    line: None,
                    message: "xcodebuild exited with code 65".into(),
                }],
                more: 0,
            },
        ];
        let reply = agent_reply(events, false);
        assert!(reply.failed);
        assert!(reply.text.starts_with("Build failed\nline 6\n"));
        assert!(reply.text.ends_with("\nline 25"));
    }

    #[test]
    fn a_run_answers_at_launch() {
        let launched = vec![BuildEvent::Phase {
            phase: BuildPhase::Running,
        }];
        assert_eq!(agent_reply(launched, true).text, "Launched");
    }

    #[test]
    fn a_build_that_ends_without_an_end_answers_stopped() {
        assert_eq!(agent_reply(vec![log("Compiling\n")], true).text, "Stopped");
    }
}
