//! Claude Code's permission modes: the ones the picker offers, the label of a mode, and whether it asks.

pub(super) const DEFAULT: &str = "default";
pub(super) const ACCEPT_EDITS: &str = "acceptEdits";
const PLAN: &str = "plan";
const AUTO: &str = "auto";
const BYPASS: &str = "bypassPermissions";
const DONT_ASK: &str = "dontAsk";

/// The modes the picker offers, in its order; `auto` only for a model that supports it. `bypassPermissions` and `dontAsk` come from the user's config alone.
pub(super) fn picker(auto: bool) -> Vec<&'static str> {
    let mut modes = vec![DEFAULT, ACCEPT_EDITS, PLAN];
    if auto {
        modes.push(AUTO);
    }
    modes
}

/// The mode's label; a mode Chord does not know shows by its own name.
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

/// Whether the mode still asks before a tool runs. `dontAsk` denies what it would ask, so it counts as asking.
pub(super) fn asks(mode: &str) -> bool {
    mode != BYPASS
}
