//! Small pure time-formatting helpers used by `controls`'s render.

/// Local wall-clock time `secs_ahead` from now, e.g. `8:07 PM`.
pub(super) fn clock(secs_ahead: f64) -> String {
    let secs_ahead = if secs_ahead.is_finite() {
        secs_ahead.max(0.)
    } else {
        0.
    };
    // SAFETY: time/localtime_r only write to the locals passed in
    let tm = unsafe {
        let t = libc::time(std::ptr::null_mut()) + secs_ahead as libc::time_t;
        let mut tm = std::mem::zeroed::<libc::tm>();
        libc::localtime_r(&t, &mut tm);
        tm
    };
    format_clock(tm.tm_hour, tm.tm_min)
}

fn format_clock(hour: i32, minute: i32) -> String {
    let half = if hour < 12 { "AM" } else { "PM" };
    format!("{}:{minute:02} {half}", (hour + 11) % 12 + 1)
}

pub(super) fn format_time(seconds: f64) -> String {
    let total = if seconds.is_finite() {
        seconds.max(0.) as u64
    } else {
        0
    };
    let (h, m, s) = (total / 3600, total / 60 % 60, total % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::{format_clock, format_time};

    #[test]
    fn formats_clock() {
        assert_eq!(format_clock(0, 5), "12:05 AM");
        assert_eq!(format_clock(11, 59), "11:59 AM");
        assert_eq!(format_clock(12, 0), "12:00 PM");
        assert_eq!(format_clock(20, 7), "8:07 PM");
    }

    #[test]
    fn formats_time() {
        assert_eq!(format_time(0.), "0:00");
        assert_eq!(format_time(65.9), "1:05");
        assert_eq!(format_time(3725.), "1:02:05");
        assert_eq!(format_time(f64::NAN), "0:00");
    }
}
