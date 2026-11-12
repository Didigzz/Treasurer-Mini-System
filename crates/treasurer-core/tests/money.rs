use treasurer_core::money::{
    format_date_only, format_money, format_period, format_short_period, is_currency_code,
    minor_to_input_value, parse_amount_input, parse_money_to_minor, parse_month_key, shift_period,
    MAX_AMOUNT_MINOR,
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
