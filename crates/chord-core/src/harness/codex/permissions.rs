//! Codex's permission modes: the policy pair each one sends, and the label and asking of a policy pair.

use serde_json::{Value, json};

/// The modes the picker offers, in its order.
pub(super) const MODES: [&str; 3] = [READ_ONLY, WORKSPACE, FULL_ACCESS];

const READ_ONLY: &str = "Read Only";
const WORKSPACE: &str = "Workspace";
const FULL_ACCESS: &str = "Full Access";

/// The mode a policy pair reads as, and whether it still asks. A pair that matches no mode names its sandbox and its approval policy.
pub(super) fn mode(approval: &Value, sandbox: &Value) -> (String, bool) {
    let approval_name = approval.as_str().unwrap_or("granular");
    let sandbox_name = match sandbox["type"].as_str() {
        Some("readOnly") => READ_ONLY,
        Some("workspaceWrite") => WORKSPACE,
        Some("dangerFullAccess") => FULL_ACCESS,
        Some("externalSandbox") => "External Sandbox",
        _ => "Sandbox",
    };
    let label = match (approval_name, sandbox_name) {
        ("on-request", READ_ONLY | WORKSPACE) | ("never", FULL_ACCESS) => sandbox_name.into(),
        _ => format!("{sandbox_name}, {approval_name}"),
    };
    (label, approval_name != "never")
}

/// The approval policy and the sandbox policy of a picked mode. Workspace keeps `workspace`, the policy the thread opened with, so the user's writable roots and network setting stay.
pub(super) fn policies(mode: &str, workspace: Option<&Value>) -> Option<(Value, Value)> {
    match mode {
        READ_ONLY => Some((
            json!("on-request"),
            json!({ "type": "readOnly", "networkAccess": false }),
        )),
        WORKSPACE => Some((
            json!("on-request"),
            workspace.cloned().unwrap_or_else(|| {
                json!({
                    "type": "workspaceWrite",
                    "writableRoots": [],
                    "networkAccess": false,
                    "excludeTmpdirEnvVar": false,
                    "excludeSlashTmp": false,
                })
            }),
        )),
        FULL_ACCESS => Some((json!("never"), json!({ "type": "dangerFullAccess" }))),
        _ => None,
    }
}
