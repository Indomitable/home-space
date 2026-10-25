use icu_datetime::{
    DateTimeFormatter,
    fieldsets::{self, YMD},
    input::{Date, DateTime, Time},
};
use icu_locale::Locale;
use time::OffsetDateTime;

type Formatter = DateTimeFormatter<fieldsets::YMDT>;

/// Formats `time` per the system's regional settings, falling back to the fixed
/// `YYYY-MM-DD HH:MM` form when the locale or timestamp can't be handled.
pub fn short_date(time: OffsetDateTime) -> String {
    thread_local! {
        static FORMATTER: Option<Formatter> = formatter();
    }
    FORMATTER
        .with(|formatter| {
            formatter
                .as_ref()
                .and_then(|formatter| localized(formatter, time))
        })
        .unwrap_or_else(|| api_client::format::short_date(time))
}

fn formatter() -> Option<Formatter> {
    let locale = sys_locale::get_locale()?.parse::<Locale>().ok()?;
    DateTimeFormatter::try_new(locale.into(), YMD::short().with_time_hm()).ok()
}

fn localized(formatter: &Formatter, time: OffsetDateTime) -> Option<String> {
    let date = Date::try_new_iso(time.year(), u8::from(time.month()), time.day()).ok()?;
    let time = Time::try_new(time.hour(), time.minute(), time.second(), 0).ok()?;
    Some(formatter.format(&DateTime { date, time }).to_string())
}
