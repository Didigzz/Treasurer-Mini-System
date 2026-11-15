use chrono::{Datelike, Duration, NaiveDate};
use treasurer_core::money::{
    add_months, calendar_weeks, format_date_only, format_money, format_period,
    format_short_period, is_currency_code, iso_date_to_epoch_noon, local_datetime_ms,
    minor_to_input_value, parse_amount_input, parse_money_to_minor, parse_month_key, shift_period,
    split_local_datetime, MAX_AMOUNT_MINOR, MAX_YEAR, MIN_YEAR,
};

#[test]
fn parses_whole_and_fractional_amounts() {
    assert_eq!(parse_money_to_minor("250"), Some(25000));
    assert_eq!(parse_money_to_minor("250.5"), Some(25050));
    assert_eq!(parse_money_to_minor("250.50"), Some(25050));
    assert_eq!(parse_money_to_minor("1,234.56"), Some(123456));
    assert_eq!(parse_money_to_minor(".5"), Some(50));
    assert_eq!(parse_money_to_minor("0"), Some(0));
}

#[test]
fn rejects_invalid_or_unsafe_values() {
    assert_eq!(parse_money_to_minor(""), None);
    assert_eq!(parse_money_to_minor("   "), None);
    assert_eq!(parse_money_to_minor("-1"), None);
    assert_eq!(parse_money_to_minor("1.234"), None);
    assert_eq!(parse_money_to_minor("1,2,3"), None);
    assert_eq!(parse_money_to_minor("abc"), None);
    assert_eq!(parse_money_to_minor("."), None);
    assert_eq!(parse_money_to_minor("1.2.3"), None);
}

#[test]
fn rejects_amounts_above_the_maximum() {
    assert_eq!(parse_money_to_minor("100000000.00"), None);
    assert_eq!(parse_money_to_minor("99999999.99"), Some(MAX_AMOUNT_MINOR));
}

#[test]
fn whole_peso_mode_rejects_fractional_input_instead_of_rounding() {
    assert_eq!(parse_amount_input("250.50", false), None);
    assert_eq!(parse_amount_input("250.00", false), Some(25000));
    assert_eq!(parse_amount_input("250.50", true), Some(25050));
}

#[test]
fn minor_to_input_value_produces_two_decimals() {
    assert_eq!(minor_to_input_value(25000), "250.00");
    assert_eq!(minor_to_input_value(1), "0.01");
    assert_eq!(minor_to_input_value(MAX_AMOUNT_MINOR), "99999999.99");
}

#[test]
fn formats_currency_the_same_way_as_the_typescript_app() {
    assert_eq!(format_money(25000, "PHP"), "\u{20b1}250.00");
    assert_eq!(format_money(123456789, "PHP"), "\u{20b1}1,234,567.89");
    assert_eq!(format_money(123456789, "USD"), "$1,234,567.89");
    assert_eq!(format_money(123456789, "AUD"), "$1,234,567.89");
    assert_eq!(format_money(123456789, "EUR"), "\u{20ac}1,234,567.89");
    assert_eq!(format_money(123456789, "GBP"), "\u{00a3}1,234,567.89");
    assert_eq!(format_money(123456789, "MYR"), "RM\u{00a0}1,234,567.89");
    assert_eq!(format_money(123456789, "THB"), "\u{0e3f}1,234,567.89");
    assert_eq!(format_money(123456789, "AED"), "AED\u{00a0}1,234,567.89");
    assert_eq!(format_money(123456789, "SAR"), "SAR\u{00a0}1,234,567.89");
    assert_eq!(format_money(123456789, "XYZ"), "XYZ 1,234,567.89");
}

#[test]
fn formats_periods_and_dates() {
    assert_eq!(format_period(2026, 11), "November 2026");
    assert_eq!(format_short_period(2026, 11), "Nov 2026");
    assert_eq!(format_date_only("2026-11-08"), "Nov 8, 2026");
    assert_eq!(format_date_only("not-a-date"), "not-a-date");
}

#[test]
fn parses_and_shifts_month_keys() {
    assert_eq!(parse_month_key("2026-11"), Some((2026, 11)));
    assert_eq!(parse_month_key("1999-11"), None);
    assert_eq!(parse_month_key("2026-13"), None);
    assert_eq!(shift_period(2026, 1, -1), (2025, 12));
    assert_eq!(shift_period(2026, 12, 1), (2027, 1));
}

#[test]
fn knows_the_supported_currencies() {
    assert!(is_currency_code("PHP"));
    assert!(is_currency_code("SAR"));
    assert!(!is_currency_code("XYZ"));
}

#[test]
fn calendar_grid_starts_on_sunday_and_covers_the_month() {
    let days = calendar_weeks(2026, 11);
    assert_eq!(days.len(), 42);
    assert_eq!(days[0].0.weekday().num_days_from_sunday(), 0);
    for pair in days.windows(2) {
        assert_eq!(pair[1].0 - pair[0].0, Duration::days(1));
    }

    let in_month: Vec<NaiveDate> = days
        .iter()
        .filter(|(_, in_month)| *in_month)
        .map(|(date, _)| *date)
        .collect();
    assert_eq!(in_month.len(), 30);
    assert_eq!(in_month[0], NaiveDate::from_ymd_opt(2026, 11, 1).unwrap());
    assert_eq!(in_month[29], NaiveDate::from_ymd_opt(2026, 11, 30).unwrap());

    // February 2027 starts on a Monday: one leading cell from January.
    let february = calendar_weeks(2027, 2);
    assert_eq!(february.len(), 42);
    assert!(!february[0].1);
    assert!(february[1].1);
}

#[test]
fn add_months_clamps_to_the_supported_years() {
    assert_eq!(add_months(2026, 1, -1), (2025, 12));
    assert_eq!(add_months(2026, 12, 1), (2027, 1));
    assert_eq!(add_months(MIN_YEAR, 1, -1), (MIN_YEAR, 1));
    assert_eq!(add_months(MAX_YEAR, 12, 1), (MAX_YEAR, 12));
    assert_eq!(add_months(2026, 11, 0), (2026, 11));
}

#[test]
fn local_datetime_round_trips_through_the_local_zone() {
    let date = NaiveDate::from_ymd_opt(2026, 11, 15).unwrap();
    let epoch = local_datetime_ms(date, 14, 35);
    let (round_date, hour, minute, pm) = split_local_datetime(epoch);
    assert_eq!(round_date, date);
    assert_eq!((hour, minute, pm), (2, 35, true));

    let midnight = split_local_datetime(local_datetime_ms(date, 0, 5));
    assert_eq!((midnight.1, midnight.2, midnight.3), (12, 5, false));
    let noon = split_local_datetime(local_datetime_ms(date, 12, 0));
    assert_eq!((noon.1, noon.2, noon.3), (12, 0, true));
}

#[test]
fn iso_dates_convert_to_noon_of_that_day() {
    let epoch = iso_date_to_epoch_noon("2026-11-15").unwrap();
    let (date, hour, minute, _) = split_local_datetime(epoch);
    assert_eq!(date, NaiveDate::from_ymd_opt(2026, 11, 15).unwrap());
    assert_eq!((hour, minute), (12, 0));
    assert_eq!(iso_date_to_epoch_noon("not-a-date"), None);
}
