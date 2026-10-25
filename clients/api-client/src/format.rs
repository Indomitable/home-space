use time::OffsetDateTime;

const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
const STEP: f64 = 1024.0;

/// Formats a byte count the way file managers do, e.g. `1.5 MB`.
pub fn human_size(bytes: i64) -> String {
    let bytes = bytes.max(0);
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= STEP && unit + 1 < UNITS.len() {
        value /= STEP;
        unit += 1;
    }
    let label = UNITS.get(unit).copied().unwrap_or("B");
    if unit == 0 {
        format!("{bytes} {label}")
    } else {
        format!("{value:.1} {label}")
    }
}

/// Formats a UTC timestamp as `YYYY-MM-DD HH:MM`.
pub fn short_date(time: OffsetDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        time.year(),
        u8::from(time.month()),
        time.day(),
        time.hour(),
        time.minute()
    )
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;

    #[test]
    fn small_sizes_are_shown_in_bytes() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(1023), "1023 B");
    }

    #[test]
    fn larger_sizes_use_binary_units_with_one_decimal() {
        assert_eq!(human_size(1536), "1.5 KB");
        assert_eq!(human_size(5 * 1024 * 1024), "5.0 MB");
    }

    #[test]
    fn negative_sizes_clamp_to_zero() {
        assert_eq!(human_size(-5), "0 B");
    }

    #[test]
    fn dates_are_zero_padded() {
        assert_eq!(
            short_date(datetime!(2026-03-04 05:06 UTC)),
            "2026-03-04 05:06"
        );
    }
}
