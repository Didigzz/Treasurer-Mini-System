use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone, Timelike};

pub const MINOR_UNITS_PER_UNIT: i64 = 100;
pub const MAX_AMOUNT_MINOR: i64 = 9_999_999_999;
pub const MIN_YEAR: i32 = 2000;
pub const MAX_YEAR: i32 = 2100;

pub const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

pub const CURRENCY_OPTIONS: [(&str, &str); 13] = [
    ("PHP", "Philippine peso (PHP)"),
    ("USD", "US dollar (USD)"),
    ("EUR", "Euro (EUR)"),
    ("GBP", "British pound (GBP)"),
    ("AUD", "Australian dollar (AUD)"),
    ("CAD", "Canadian dollar (CAD)"),
    ("SGD", "Singapore dollar (SGD)"),
    ("HKD", "Hong Kong dollar (HKD)"),
    ("NZD", "New Zealand dollar (NZD)"),
    ("MYR", "Malaysian ringgit (MYR)"),
    ("THB", "Thai baht (THB)"),
    ("AED", "UAE dirham (AED)"),
    ("SAR", "Saudi riyal (SAR)"),
];

pub fn is_currency_code(value: &str) -> bool {
    CURRENCY_OPTIONS
        .iter()
        .any(|(code, _)| *code == value)
}

/// Parse a user-entered amount into integer minor units without floating point.
pub fn parse_money_to_minor(input: &str) -> Option<i64> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.starts_with('-') {
        return None;
    }

    if trimmed.contains(',') && !valid_grouped(trimmed) {
        return None;
    }

    let normalized = trimmed.replace(',', "");
    if !valid_plain(&normalized) {
        return None;
    }

    let (whole_part, decimal_part) = match normalized.split_once('.') {
        Some((whole, decimals)) => (whole, decimals),
        None => (normalized.as_str(), ""),
    };

    let whole: i64 = if whole_part.is_empty() {
        0
    } else {
        whole_part.parse().ok()?
    };
    let mut cents_text = decimal_part.to_string();
    while cents_text.len() < 2 {
        cents_text.push('0');
    }
    let cents: i64 = if cents_text.is_empty() {
        0
    } else {
        cents_text.parse().ok()?
    };

    let minor = whole.checked_mul(MINOR_UNITS_PER_UNIT)?.checked_add(cents)?;
    if !(0..=MAX_AMOUNT_MINOR).contains(&minor) {
        return None;
    }

    Some(minor)
}

fn valid_plain(value: &str) -> bool {
    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or("");
    let decimals = parts.next();
    if parts.next().is_some() {
        return false;
    }
    if !whole.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    if let Some(decimals) = decimals {
        if decimals.len() > 2 || !decimals.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }
    let has_whole = whole.chars().any(|c| c.is_ascii_digit());
    let has_decimals = decimals
        .map(|decimals| decimals.chars().any(|c| c.is_ascii_digit()))
        .unwrap_or(false);
    has_whole || has_decimals
}

fn valid_grouped(value: &str) -> bool {
    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or("");
    let decimals = parts.next();
    if parts.next().is_some() {
        return false;
    }

    let groups: Vec<&str> = whole.split(',').collect();
    if groups.len() < 2 {
        return false;
    }
    let first = groups[0];
    if first.is_empty() || first.len() > 3 || !first.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    for group in &groups[1..] {
        if group.len() != 3 || !group.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }
    if let Some(decimals) = decimals {
        if decimals.len() > 2 || !decimals.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }
    true
}

/// Parse an amount honouring the centavos setting. Fractional values are
/// rejected rather than rounded when centavos are disabled.
pub fn parse_amount_input(input: &str, allow_centavos: bool) -> Option<i64> {
    if allow_centavos {
        return parse_money_to_minor(input);
    }

    let normalized = input.trim().replace(',', "");
    let mut parts = normalized.split('.');
    let whole = parts.next().unwrap_or("");
    let decimals = parts.next();
    if parts.next().is_some() {
        return None;
    }
    if whole.is_empty() || !whole.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if let Some(decimals) = decimals {
        if decimals.len() > 2 || !decimals.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        if decimals.chars().any(|c| c != '0') {
            return None;
        }
    }

    let minor = parse_money_to_minor(&normalized)?;
    if minor % MINOR_UNITS_PER_UNIT != 0 {
        return None;
    }
    Some(minor)
}

pub fn minor_to_input_value(minor: i64) -> String {
    format!(
        "{}.{:02}",
        minor.div_euclid(MINOR_UNITS_PER_UNIT),
        minor.rem_euclid(MINOR_UNITS_PER_UNIT)
    )
}

fn group_thousands(value: i64) -> String {
    let digits = value.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    grouped
}

fn currency_prefix(code: &str) -> Option<String> {
    let prefix = match code {
        "PHP" => "\u{20b1}".to_string(),
        "USD" | "AUD" | "CAD" | "SGD" | "HKD" | "NZD" => "$".to_string(),
        "EUR" => "\u{20ac}".to_string(),
        "GBP" => "\u{00a3}".to_string(),
        "MYR" => "RM\u{00a0}".to_string(),
        "THB" => "\u{0e3f}".to_string(),
        "AED" => "AED\u{00a0}".to_string(),
        "SAR" => "SAR\u{00a0}".to_string(),
        _ => return None,
    };
    Some(prefix)
}

pub fn format_money(minor: i64, currency_code: &str) -> String {
    let amount = minor.abs();
    let whole = amount / MINOR_UNITS_PER_UNIT;
    let cents = amount % MINOR_UNITS_PER_UNIT;
    let number = format!("{}.{:02}", group_thousands(whole), cents);

    match currency_prefix(currency_code) {
        Some(prefix) => format!("{prefix}{number}"),
        None => format!("{currency_code} {number}"),
    }
}

pub fn format_period(year: i32, month: i32) -> String {
    let name = MONTH_NAMES
        .get((month - 1).max(0) as usize)
        .copied()
        .unwrap_or("Month");
    format!("{name} {year}")
}

pub fn format_short_period(year: i32, month: i32) -> String {
    let name = MONTH_NAMES
        .get((month - 1).max(0) as usize)
        .copied()
        .unwrap_or("Month");
    format!("{} {year}", &name[..3.min(name.len())])
}

fn short_month(month: u32) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "",
    }
}

pub fn format_date(epoch_ms: i64) -> String {
    match Local.timestamp_millis_opt(epoch_ms).single() {
        Some(date) => format!(
            "{} {}, {}",
            short_month(date.month()),
            date.day(),
            date.year()
        ),
        None => String::new(),
    }
}

pub fn format_datetime(epoch_ms: i64) -> String {
    match Local.timestamp_millis_opt(epoch_ms).single() {
        Some(date) => {
            let hour = date.hour();
            let hour12 = if hour % 12 == 0 { 12 } else { hour % 12 };
            let meridiem = if hour < 12 { "AM" } else { "PM" };
            format!(
                "{} {}, {} at {}:{:02} {}",
                short_month(date.month()),
                date.day(),
                date.year(),
                hour12,
                date.minute(),
                meridiem
            )
        }
        None => String::new(),
    }
}

pub fn format_date_only(iso_date: &str) -> String {
    match NaiveDate::parse_from_str(iso_date, "%Y-%m-%d") {
        Ok(date) => format!(
            "{} {}, {}",
            short_month(date.month()),
            date.day(),
            date.year()
        ),
        Err(_) => iso_date.to_string(),
    }
}

pub fn to_iso_date(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

pub fn today_iso_date() -> String {
    to_iso_date(Local::now().date_naive())
}

pub fn is_overdue(due_date: Option<&str>, has_balance: bool) -> bool {
    match due_date {
        Some(due_date) if has_balance => due_date < today_iso_date().as_str(),
        _ => false,
    }
}

pub fn days_overdue(due_date: &str) -> i64 {
    match NaiveDate::parse_from_str(due_date, "%Y-%m-%d") {
        Ok(due) => {
            let today = Local::now().date_naive();
            today.signed_duration_since(due).num_days().max(0)
        }
        Err(_) => 0,
    }
}

pub fn month_key(year: i32, month: i32) -> String {
    format!("{year}-{month:02}")
}

pub fn parse_month_key(value: &str) -> Option<(i32, i32)> {
    let mut parts = value.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: i32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    if !(2000..=2100).contains(&year) || !(1..=12).contains(&month) {
        return None;
    }
    Some((year, month))
}

pub fn current_period() -> (i32, i32) {
    let now = Local::now().date_naive();
    (now.year(), now.month() as i32)
}

pub fn shift_period(year: i32, month: i32, delta: i32) -> (i32, i32) {
    let total = year * 12 + (month - 1) + delta;
    let shifted_year = total.div_euclid(12);
    let shifted_month = total.rem_euclid(12) + 1;
    (shifted_year, shifted_month)
}

/// Like `shift_period`, but stays inside the selectable year range so the
/// calendar can never navigate out of the supported bounds.
pub fn add_months(year: i32, month: i32, delta: i32) -> (i32, i32) {
    let (mut year, mut month) = shift_period(year, month.clamp(1, 12), delta);
    if year < MIN_YEAR {
        year = MIN_YEAR;
        month = 1;
    }
    if year > MAX_YEAR {
        year = MAX_YEAR;
        month = 12;
    }
    (year, month)
}

/// 42 cells (six Sunday-first weeks) that fully cover the month, including
/// leading and trailing days from the adjacent months.
pub fn calendar_weeks(year: i32, month: i32) -> Vec<(NaiveDate, bool)> {
    let month = month.clamp(1, 12);
    let Some(first) = NaiveDate::from_ymd_opt(year, month as u32, 1) else {
        return Vec::new();
    };
    let offset = first.weekday().num_days_from_sunday() as i64;
    let start = first - Duration::days(offset);

    (0..42)
        .map(|index| {
            let date = start + Duration::days(index);
            (date, date.year() == year && date.month() == month as u32)
        })
        .collect()
}

/// Local wall-clock date and time to epoch millis. Ambiguous or skipped local
/// times (DST gaps) fall back to the earliest match, then noon, so a payment
/// never lands outside the chosen day.
pub fn local_datetime_ms(date: NaiveDate, hour: u32, minute: u32) -> i64 {
    let hour = hour.min(23);
    let minute = minute.min(59);
    if let Some(naive) = date.and_hms_opt(hour, minute, 0) {
        let resolved = Local.from_local_datetime(&naive);
        if let Some(datetime) = resolved.single().or_else(|| resolved.earliest()) {
            return datetime.timestamp_millis();
        }
    }
    if let Some(noon) = date.and_hms_opt(12, 0, 0) {
        let resolved = Local.from_local_datetime(&noon);
        if let Some(datetime) = resolved.single().or_else(|| resolved.earliest()) {
            return datetime.timestamp_millis();
        }
        return Local.from_utc_datetime(&noon).timestamp_millis();
    }
    date.and_hms_opt(12, 0, 0)
        .map(|noon| Local.from_utc_datetime(&noon).timestamp_millis())
        .unwrap_or(0)
}

/// Splits an epoch into (date, hour 1-12, minute, is_pm) in the local zone.
pub fn split_local_datetime(epoch_ms: i64) -> (NaiveDate, u32, u32, bool) {
    let resolved = Local.timestamp_millis_opt(epoch_ms);
    let datetime = resolved
        .single()
        .or_else(|| Local.timestamp_millis_opt(epoch_ms).earliest());
    match datetime {
        Some(datetime) => {
            let hour = datetime.hour();
            (
                datetime.date_naive(),
                if hour % 12 == 0 { 12 } else { hour % 12 },
                datetime.minute(),
                hour >= 12,
            )
        }
        None => (
            NaiveDate::from_ymd_opt(1970, 1, 1).expect("1970-01-01 is a valid date"),
            12,
            0,
            false,
        ),
    }
}

/// Noon local time on an ISO `YYYY-MM-DD` date. Used for date-only facts such
/// as a bill due date or a change-return date.
pub fn iso_date_to_epoch_noon(iso: &str) -> Option<i64> {
    NaiveDate::parse_from_str(iso.trim(), "%Y-%m-%d")
        .ok()
        .map(|date| local_datetime_ms(date, 12, 0))
}
