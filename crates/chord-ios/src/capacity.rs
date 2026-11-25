use std::time::Instant;

pub(crate) struct Booted {
    pub udid: String,
    pub opened: Option<Instant>,
    pub busy: bool,
}

const GIGABYTE: u64 = 1 << 30;

pub(crate) fn limit(ram_bytes: u64) -> usize {
    usize::try_from(ram_bytes / GIGABYTE / 6)
        .unwrap_or(usize::MAX)
        .max(1)
}

pub(crate) fn installed_ram() -> u64 {
    let mut bytes: u64 = 0;
    let mut size = size_of::<u64>();
    let status = unsafe {
        libc::sysctlbyname(
            c"hw.memsize".as_ptr(),
            (&raw mut bytes).cast(),
            &raw mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if status == 0 { bytes } else { 0 }
}

pub(crate) fn to_shut_down(mut booted: Vec<Booted>, keep: usize) -> Vec<String> {
    let excess = booted.len().saturating_sub(keep);
    booted.retain(|device| !device.busy);
    booted.sort_by_key(|device| device.opened);
    booted
        .into_iter()
        .take(excess)
        .map(|device| device.udid)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn booted(udid: &str, opened: Option<Instant>, busy: bool) -> Booted {
        Booted {
            udid: udid.into(),
            opened,
            busy,
        }
    }

    #[test]
    fn allows_one_simulator_per_6_gb_and_at_least_one() {
        assert_eq!(limit(0), 1);
        assert_eq!(limit(8 * GIGABYTE), 1);
        assert_eq!(limit(18 * GIGABYTE), 3);
        assert_eq!(limit(36 * GIGABYTE), 6);
    }

    #[test]
    fn shuts_down_the_longest_opened_one_without_a_run_when_the_limit_is_reached() {
        let start = Instant::now();
        let at = |seconds| Some(start + Duration::from_secs(seconds));
        let devices = vec![
            booted("old-busy", at(1), true),
            booted("newer", at(3), false),
            booted("older", at(2), false),
        ];
        assert_eq!(to_shut_down(devices, 2), vec!["older"]);
    }

    #[test]
    fn shuts_down_a_simulator_the_session_never_saw_first() {
        let start = Instant::now();
        let devices = vec![
            booted("seen", Some(start), false),
            booted("unseen", None, false),
        ];
        assert_eq!(to_shut_down(devices, 1), vec!["unseen"]);
    }

    #[test]
    fn shuts_down_nothing_below_the_limit() {
        let start = Instant::now();
        assert!(to_shut_down(vec![booted("seen", Some(start), false)], 1).is_empty());
    }

    #[test]
    fn leaves_every_simulator_booted_when_all_have_a_run() {
        assert!(to_shut_down(vec![booted("a", None, true)], 0).is_empty());
    }
}
