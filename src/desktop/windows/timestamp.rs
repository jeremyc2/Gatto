use windows::Win32::{
    Foundation::{FILETIME, SYSTEMTIME},
    System::Time::{
        DYNAMIC_TIME_ZONE_INFORMATION, FileTimeToSystemTime, SystemTimeToTzSpecificLocalTimeEx,
    },
};

/// Format Unix milliseconds in the user's local time zone.
pub fn format_local_timestamp(milliseconds: u64) -> Option<String> {
    format_timestamp(milliseconds, None)
}

fn format_timestamp(
    milliseconds: u64,
    timezone: Option<&DYNAMIC_TIME_ZONE_INFORMATION>,
) -> Option<String> {
    // Zero denotes missing metadata; the display format uses a four-digit year.
    if milliseconds == 0 || milliseconds > 253_402_300_799_999 {
        return None;
    }
    // FILETIME counts 100 ns ticks from 1601-01-01, rather than Unix's 1970 epoch.
    // The range check above keeps both operations below u64::MAX and bit 63 clear.
    let ticks = (milliseconds + 11_644_473_600_000) * 10_000;
    let filetime = FILETIME {
        dwLowDateTime: ticks as u32,
        dwHighDateTime: (ticks >> 32) as u32,
    };
    let mut utc = SYSTEMTIME::default();
    let mut local = SYSTEMTIME::default();
    // SAFETY: All pointers reference live, distinct structures for these
    // synchronous calls. None selects the system time zone, including DST rules.
    unsafe {
        FileTimeToSystemTime(&filetime, &mut utc).ok()?;
        SystemTimeToTzSpecificLocalTimeEx(timezone.map(std::ptr::from_ref), &utc, &mut local)
            .ok()?;
    }
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let month = MONTHS.get(usize::from(local.wMonth).checked_sub(1)?)?;
    Some(format!(
        "{month} {}, {:04} {:02}:{:02}",
        local.wDay, local.wYear, local.wHour, local.wMinute
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::Time::EnumDynamicTimeZoneInformation;

    fn timezone(key: &str) -> DYNAMIC_TIME_ZONE_INFORMATION {
        for index in 0.. {
            let mut zone = DYNAMIC_TIME_ZONE_INFORMATION::default();
            // SAFETY: The output points to a live structure for this synchronous call.
            let result = unsafe { EnumDynamicTimeZoneInformation(index, &mut zone) };
            assert_eq!(
                result, 0,
                "Time zone {key} not found: Windows error {result}"
            );
            // Native buffers may contain unrelated data after the first NUL.
            let name = zone.TimeZoneKeyName;
            let end = name
                .iter()
                .position(|&unit| unit == 0)
                .unwrap_or(name.len());
            if name[..end].iter().copied().eq(key.encode_utf16()) {
                return zone;
            }
        }
        unreachable!()
    }

    #[test]
    fn system_time_zone_formats_a_present_timestamp() {
        assert!(format_local_timestamp(1_700_000_000_000).is_some());
    }

    #[test]
    fn formats_unix_milliseconds_and_rejects_missing_or_out_of_range_dates() {
        let utc = timezone("UTC");
        let format = |milliseconds| format_timestamp(milliseconds, Some(&utc));
        assert_eq!(format(1).as_deref(), Some("Jan 1, 1970 00:00"));
        assert_eq!(
            format(1_700_000_000_000).as_deref(),
            Some("Nov 14, 2023 22:13")
        );
        assert_eq!(
            format(951_782_400_000).as_deref(),
            Some("Feb 29, 2000 00:00")
        );
        assert_eq!(
            format(253_402_300_799_999).as_deref(),
            Some("Dec 31, 9999 23:59")
        );
        for missing in [0, 253_402_300_800_000, u64::MAX] {
            assert_eq!(format(missing), None);
        }
    }

    #[test]
    fn local_time_crosses_year_and_daylight_saving_boundaries() {
        // Registered Eastern rules include the 2024 DST transitions below.
        let eastern = timezone("Eastern Standard Time");
        let format = |milliseconds| format_timestamp(milliseconds, Some(&eastern)).unwrap();
        assert_eq!(format(1), "Dec 31, 1969 19:00");
        assert_eq!(format(1_710_053_999_999), "Mar 10, 2024 01:59");
        assert_eq!(format(1_710_054_000_000), "Mar 10, 2024 03:00");
        assert_eq!(format(1_730_613_599_999), "Nov 3, 2024 01:59");
        assert_eq!(format(1_730_613_600_000), "Nov 3, 2024 01:00");
    }
}
