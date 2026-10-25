//! The icon of the window follows the calendar: 5 days before a holiday, on the day and 2 days after it the programs
//! show a festive variant. Which holidays and how they look is up to each program; this module only knows the dates.

/// A PNG of the logo in two sizes: 256 px (the logo in the program) and 512 px (the icon of the window and the Dock).
pub type Icon = (&'static [u8], &'static [u8]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Holiday {
    Christmas,
    NewYearsEve,
    Halloween,
    Easter,
}

/// The festive variants of a program.
pub struct Variants {
    pub christmas: Icon,
    pub new_years_eve: Icon,
    pub halloween: Icon,
    pub easter: Icon,
}

const DAYS_BEFORE: i64 = 5;
const DAYS_AFTER: i64 = 2;

/// Days since 1970-01-01 of a civil date (proleptic Gregorian).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Easter Sunday of the Western churches (the anonymous Gregorian algorithm): (month, day).
fn easter(y: i64) -> (i64, i64) {
    let (a, b, c) = (y % 19, y / 100, y % 100);
    let (d, e) = (b / 4, b % 4);
    let g = (8 * b + 13) / 25;
    let h = (19 * a + b - d - g + 15) % 30;
    let (i, k) = (c / 4, c % 4);
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 19 * l) / 433;
    let month = (h + l - 7 * m + 90) / 25;
    (month, (h + l - 7 * m + 33 * month + 19) % 32)
}

/// The holiday the date belongs to, if any. Near two holidays (26-27 December), the closer one wins.
pub fn holiday_on(year: i32, month: u32, day: u32) -> Option<Holiday> {
    let today = days_from_civil(year as i64, month as i64, day as i64);
    let mut best: Option<(i64, Holiday)> = None;
    for y in (year as i64 - 1)..=(year as i64 + 1) {
        let (em, ed) = easter(y);
        let dates = [
            (days_from_civil(y, 12, 25), Holiday::Christmas),
            (days_from_civil(y, 12, 31), Holiday::NewYearsEve),
            (days_from_civil(y, 10, 31), Holiday::Halloween),
            (days_from_civil(y, em, ed), Holiday::Easter),
        ];
        for (date, holiday) in dates {
            let left = date - today; // days until the holiday, negative once it is over
            if (-DAYS_AFTER..=DAYS_BEFORE).contains(&left) && best.is_none_or(|(d, _)| left.abs() < d) {
                best = Some((left.abs(), holiday));
            }
        }
    }
    best.map(|(_, h)| h)
}

/// Today's date (UTC) as year, month, day. `BASECREATOR_ICON_DATE=YYYY-MM-DD` pretends it is another day.
pub fn today() -> (i32, u32, u32) {
    if let Some(date) = std::env::var("BASECREATOR_ICON_DATE").ok().and_then(|s| parse_date(&s)) {
        return date;
    }
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    civil_from_days(secs.div_euclid(86_400))
}

fn parse_date(s: &str) -> Option<(i32, u32, u32)> {
    let mut parts = s.trim().split('-');
    let (y, m, d) = (parts.next()?.parse().ok()?, parts.next()?.parse().ok()?, parts.next()?.parse().ok()?);
    ((1..=12).contains(&m) && (1..=31).contains(&d) && parts.next().is_none()).then_some((y, m, d))
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    ((yoe + era * 400 + i64::from(m <= 2)) as i32, m as u32, d as u32)
}

/// The icon for the given date: the festive variant on a holiday, otherwise `normal`.
pub fn icon_for(date: (i32, u32, u32), variants: &Variants, normal: Icon) -> Icon {
    match holiday_on(date.0, date.1, date.2) {
        Some(Holiday::Christmas) => variants.christmas,
        Some(Holiday::NewYearsEve) => variants.new_years_eve,
        Some(Holiday::Halloween) => variants.halloween,
        Some(Holiday::Easter) => variants.easter,
        None => normal,
    }
}

/// The icon for today.
pub fn icon_today(variants: &Variants, normal: Icon) -> Icon {
    icon_for(today(), variants, normal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easter_dates_are_right() {
        assert_eq!(easter(2024), (3, 31));
        assert_eq!(easter(2025), (4, 20));
        assert_eq!(easter(2026), (4, 5));
        assert_eq!(easter(2038), (4, 25));
    }

    #[test]
    fn the_window_is_five_days_before_and_two_days_after() {
        use Holiday::*;
        assert_eq!(holiday_on(2026, 10, 25), None);
        assert_eq!(holiday_on(2026, 10, 26), Some(Halloween));
        assert_eq!(holiday_on(2026, 10, 31), Some(Halloween));
        assert_eq!(holiday_on(2026, 11, 2), Some(Halloween));
        assert_eq!(holiday_on(2026, 11, 3), None);
        assert_eq!(holiday_on(2026, 12, 19), None);
        assert_eq!(holiday_on(2026, 12, 20), Some(Christmas));
        assert_eq!(holiday_on(2026, 3, 30), None);
        assert_eq!(holiday_on(2026, 3, 31), Some(Easter));
        assert_eq!(holiday_on(2026, 4, 7), Some(Easter));
        assert_eq!(holiday_on(2026, 4, 8), None);
    }

    #[test]
    fn christmas_wins_after_the_day_and_the_new_year_follows() {
        use Holiday::*;
        assert_eq!(holiday_on(2026, 12, 26), Some(Christmas));
        assert_eq!(holiday_on(2026, 12, 27), Some(Christmas));
        assert_eq!(holiday_on(2026, 12, 28), Some(NewYearsEve));
        assert_eq!(holiday_on(2026, 12, 31), Some(NewYearsEve));
        assert_eq!(holiday_on(2027, 1, 2), Some(NewYearsEve));
        assert_eq!(holiday_on(2027, 1, 3), None);
    }

    #[test]
    fn the_calendar_arithmetic_round_trips() {
        assert_eq!(civil_from_days(days_from_civil(2026, 10, 25)), (2026, 10, 25));
        assert_eq!(civil_from_days(days_from_civil(2024, 2, 29)), (2024, 2, 29));
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(parse_date("2026-12-24"), Some((2026, 12, 24)));
        assert_eq!(parse_date("tomorrow"), None);
    }

    #[test]
    fn the_variant_follows_the_date() {
        let (n, c, ny, h, e): (Icon, Icon, Icon, Icon, Icon) =
            ((b"n", b"n"), (b"c", b"c"), (b"y", b"y"), (b"h", b"h"), (b"e", b"e"));
        let v = Variants { christmas: c, new_years_eve: ny, halloween: h, easter: e };
        assert_eq!(icon_for((2026, 7, 1), &v, n), n);
        assert_eq!(icon_for((2026, 12, 24), &v, n), c);
        assert_eq!(icon_for((2026, 10, 30), &v, n), h);
    }
}
