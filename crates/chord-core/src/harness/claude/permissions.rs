pub(super) const DEFAULT: &str = "default";
pub(super) const ACCEPT_EDITS: &str = "acceptEdits";
const PLAN: &str = "plan";
const AUTO: &str = "auto";
const BYPASS: &str = "bypassPermissions";
const DONT_ASK: &str = "dontAsk";

pub(super) fn picker(auto: bool) -> Vec<&'static str> {
    let mut modes = vec![DEFAULT, ACCEPT_EDITS, PLAN];
    if auto {
        modes.push(AUTO);
    }
    modes
}

pub(super) fn label(mode: &str) -> &str {
    match mode {
        DEFAULT => "Manual",
        ACCEPT_EDITS => "Accept Edits",
        PLAN => "Plan",
        AUTO => "Auto",
        BYPASS => "Bypass Permissions",
        DONT_ASK => "Don't Ask",
        other => other,
    }
}

pub(super) fn asks(mode: &str) -> bool {
    mode != BYPASS
}
