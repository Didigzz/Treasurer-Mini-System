use std::collections::HashMap;

use treasurer_core::calculations::{
    compute_bill_summary, compute_boarder_period_summaries, compute_history_totals,
    compute_share_state, equal_split, list_history_periods, matches_boarder_filter,
    matches_history_filter, matches_share_filter, preview_payment, recompute_payment_changes,
    sort_history_bills, split_with_overrides, validate_custom_shares, BoarderPeriodSummaryInput,
    SplitFailure, SplitOutcome, SplitSuccess,
};
use treasurer_core::models::{
    BillShare, HistoryBillRef, HistoryFilter, HistoryTotals, PaymentRecord, ShareFilter,
    ShareStatus, ShareWithState, SplitBoarder, SplitResult,
};

fn payment(id: &str, amount_minor: i64, paid_at: i64) -> PaymentRecord {
    PaymentRecord {
        id: id.to_string(),
        bill_share_id: "share-1".to_string(),
        amount_minor,
        paid_at,
        note: None,
        change_minor: 0,
        change_returned_at: None,
        change_return_note: None,
        voided_at: None,
        void_reason: None,
        created_at: paid_at,
    }
}

fn boarder(id: &str, name: &str, created_at: i64) -> SplitBoarder {
    SplitBoarder {
        boarder_id: id.to_string(),
        name: name.to_string(),
        created_at,
    }
}

fn share(id: &str, name: &str, amount_due_minor: i64, payments: &[PaymentRecord]) -> ShareWithState {
    let amount = amount_due_minor;
    let paid = payments;
    ShareWithState {
        share: BillShare {
            id: id.to_string(),
            bill_id: "bill-1".to_string(),
            boarder_id: id.to_string(),
            boarder_name_snapshot: name.to_string(),
            amount_due_minor,
            created_at: 0,
        },
        state: compute_share_state(amount, paid),
        current_boarder_name: None,
        boarder_archived: false,
    }
}

fn expect_ok(outcome: SplitOutcome) -> SplitSuccess {
    match outcome {
        SplitOutcome::Ok(success) => success,
        SplitOutcome::Err(failure) => panic!("{}", failure.error),
    }
}

fn expect_err(outcome: SplitOutcome) -> SplitFailure {
    match outcome {
        SplitOutcome::Err(failure) => failure,
        SplitOutcome::Ok(_) => panic!("expected a split error"),
    }
}

#[test]
fn equal_split_distributes_centavo_remainders_in_selection_order() {
    let result = expect_ok(equal_split(
        100000,
        &[boarder("c", "Cara", 3), boarder("a", "Ana", 1), boarder("b", "Ben", 2)],
        true,
    ));

    assert_eq!(
        result.shares,
        vec![
            SplitResult { boarder_id: "c".into(), amount_minor: 33334 },
            SplitResult { boarder_id: "a".into(), amount_minor: 33333 },
            SplitResult { boarder_id: "b".into(), amount_minor: 33333 },
        ]
    );
    assert_eq!(result.shares.iter().map(|s| s.amount_minor).sum::<i64>(), 100000);
}

#[test]
fn equal_split_supports_one_centavo_totals() {
    let result = expect_ok(equal_split(1, &[boarder("a", "Ana", 1)], true));
    assert_eq!(result.shares[0].amount_minor, 1);
}

#[test]
fn equal_split_rejects_totals_too_small_for_the_count() {
    let outcome = equal_split(
        2,
        &[boarder("a", "Ana", 1), boarder("b", "Ben", 2), boarder("c", "Cara", 3)],
        true,
    );
    assert!(matches!(outcome, SplitOutcome::Err(_)));
}

#[test]
fn equal_split_rounds_to_whole_pesos_when_centavos_are_disabled() {
    let result = expect_ok(equal_split(
        10_000,
        &[boarder("a", "Ana", 1), boarder("b", "Ben", 2), boarder("c", "Cara", 3)],
        false,
    ));
    assert_eq!(
        result.shares.iter().map(|s| s.amount_minor).collect::<Vec<_>>(),
        vec![3400, 3300, 3300]
    );
}

#[test]
fn validate_custom_shares_accepts_an_exact_total() {
    assert!(
        validate_custom_shares(
            10000,
            &[
                SplitResult { boarder_id: "a".into(), amount_minor: 5000 },
                SplitResult { boarder_id: "b".into(), amount_minor: 5000 },
            ]
        )
        .valid
    );
}

#[test]
fn validate_custom_shares_rejects_short_and_over_shares() {
    assert!(
        !validate_custom_shares(
            10000,
            &[
                SplitResult { boarder_id: "a".into(), amount_minor: 4000 },
                SplitResult { boarder_id: "b".into(), amount_minor: 5000 },
            ]
        )
        .valid
    );
    assert!(
        !validate_custom_shares(
            10000,
            &[
                SplitResult { boarder_id: "a".into(), amount_minor: 6000 },
                SplitResult { boarder_id: "b".into(), amount_minor: 5000 },
            ]
        )
        .valid
    );
}

#[test]
fn validate_custom_shares_accepts_a_zero_share_when_balanced() {
    assert!(
        validate_custom_shares(
            10000,
            &[
                SplitResult { boarder_id: "a".into(), amount_minor: 10000 },
                SplitResult { boarder_id: "b".into(), amount_minor: 0 },
            ]
        )
        .valid
    );
}

#[test]
fn validate_custom_shares_rejects_negative_amounts() {
    assert!(
        !validate_custom_shares(
            10000,
            &[
                SplitResult { boarder_id: "a".into(), amount_minor: 12000 },
                SplitResult { boarder_id: "b".into(), amount_minor: -2000 },
            ]
        )
        .valid
    );
}

fn four_boarders() -> Vec<SplitBoarder> {
    vec![
        boarder("ana", "Ana", 1),
        boarder("ben", "Ben", 2),
        boarder("cara", "Cara", 3),
        boarder("dan", "Dan", 4),
    ]
}

#[test]
fn split_with_overrides_splits_the_leftover_in_selection_order() {
    let overrides = HashMap::from([("ana".to_string(), 30_000)]);
    let result = expect_ok(split_with_overrides(100_000, &four_boarders(), &overrides, true));

    assert_eq!(
        result.shares,
        vec![
            SplitResult { boarder_id: "ana".into(), amount_minor: 30_000 },
            SplitResult { boarder_id: "ben".into(), amount_minor: 23_334 },
            SplitResult { boarder_id: "cara".into(), amount_minor: 23_333 },
            SplitResult { boarder_id: "dan".into(), amount_minor: 23_333 },
        ]
    );
    assert_eq!(result.pool_count, Some(3));
    assert_eq!(result.leftover_minor, Some(70_000));
    assert!(!result.rounded);
}

#[test]
fn split_with_overrides_rounds_to_whole_pesos() {
    let overrides = HashMap::from([("ana".to_string(), 30_000)]);
    let result = expect_ok(split_with_overrides(100_000, &four_boarders(), &overrides, false));

    assert_eq!(
        result.shares.iter().map(|s| s.amount_minor).collect::<Vec<_>>(),
        vec![30_000, 23_400, 23_300, 23_300]
    );
    assert!(result.rounded);
}

#[test]
fn split_with_overrides_allows_zero_shares_when_custom_amounts_consume_the_total() {
    let boarders = four_boarders()[..3].to_vec();
    let overrides = HashMap::from([("ana".to_string(), 10_000)]);
    let result = expect_ok(split_with_overrides(10_000, &boarders, &overrides, true));

    assert_eq!(
        result.shares.iter().map(|s| s.amount_minor).collect::<Vec<_>>(),
        vec![10_000, 0, 0]
    );
}

#[test]
fn split_with_overrides_hands_out_the_last_centavos_one_at_a_time() {
    let boarders = four_boarders()[..3].to_vec();
    let overrides = HashMap::from([("ana".to_string(), 999)]);
    let result = expect_ok(split_with_overrides(1_000, &boarders, &overrides, true));

    assert_eq!(
        result.shares.iter().map(|s| s.amount_minor).collect::<Vec<_>>(),
        vec![999, 1, 0]
    );
}

#[test]
fn split_with_overrides_rejects_a_custom_total_above_the_bill() {
    let overrides = HashMap::from([("ana".to_string(), 12_000)]);
    let failure = expect_err(split_with_overrides(10_000, &four_boarders(), &overrides, true));
    assert_eq!(failure.over_by_minor, Some(2_000));
}

#[test]
fn split_with_overrides_requires_pinned_amounts_to_balance_when_pool_is_empty() {
    let all = HashMap::from([
        ("ana".to_string(), 2_500),
        ("ben".to_string(), 2_500),
        ("cara".to_string(), 2_500),
        ("dan".to_string(), 2_500),
    ]);
    assert!(matches!(
        split_with_overrides(10_000, &four_boarders(), &all, true),
        SplitOutcome::Ok(_)
    ));
    assert!(matches!(
        split_with_overrides(12_000, &four_boarders(), &all, true),
        SplitOutcome::Err(_)
    ));
}

#[test]
fn split_with_overrides_keeps_pinned_rows_in_place() {
    let overrides = HashMap::from([("ben".to_string(), 3_000)]);
    let result = expect_ok(split_with_overrides(9_000, &four_boarders(), &overrides, true));

    assert_eq!(
        result.shares,
        vec![
            SplitResult { boarder_id: "ana".into(), amount_minor: 2_000 },
            SplitResult { boarder_id: "ben".into(), amount_minor: 3_000 },
            SplitResult { boarder_id: "cara".into(), amount_minor: 2_000 },
            SplitResult { boarder_id: "dan".into(), amount_minor: 2_000 },
        ]
    );
}

#[test]
fn split_with_overrides_rejects_a_fractional_pin_in_whole_peso_mode() {
    let overrides = HashMap::from([("ana".to_string(), 1_050)]);
    assert!(matches!(
        split_with_overrides(10_000, &four_boarders(), &overrides, false),
        SplitOutcome::Err(_)
    ));
}

#[test]
fn exact_payment_has_no_change() {
    let state = compute_share_state(25000, &[payment("p1", 25000, 1000)]);
    assert_eq!(state.status, ShareStatus::Paid);
    assert_eq!(state.change_outstanding_minor, 0);
}

#[test]
fn overpayment_creates_change() {
    let state = compute_share_state(25000, &[payment("p1", 30000, 1000)]);
    assert_eq!(state.status, ShareStatus::Paid);
    assert_eq!(state.balance_minor, 0);
    assert_eq!(state.change_outstanding_minor, 5000);
}

#[test]
fn partial_payments_accumulate() {
    let state = compute_share_state(
        25000,
        &[payment("p1", 10000, 1000), payment("p2", 10000, 2000)],
    );
    assert_eq!(state.status, ShareStatus::Partial);
    assert_eq!(state.paid_minor, 20000);
    assert_eq!(state.balance_minor, 5000);
}

#[test]
fn change_is_assigned_to_the_payment_crossing_the_due_amount() {
    let payments = [payment("p1", 10000, 1000), payment("p2", 20000, 2000)];
    let changes = recompute_payment_changes(25000, &payments);
    assert_eq!(
        changes.iter().find(|item| item.payment.id == "p1").unwrap().computed_change_minor,
        0
    );
    assert_eq!(
        changes.iter().find(|item| item.payment.id == "p2").unwrap().computed_change_minor,
        5000
    );
}

#[test]
fn voided_payments_are_excluded_and_change_is_recomputed() {
    let mut voided = payment("p1", 10000, 1000);
    voided.voided_at = Some(3000);
    voided.void_reason = Some("Mistake".to_string());
    let state = compute_share_state(25000, &[voided, payment("p2", 20000, 2000)]);

    assert_eq!(state.paid_minor, 20000);
    assert_eq!(state.balance_minor, 5000);
    assert_eq!(state.change_outstanding_minor, 0);
}

#[test]
fn preview_distinguishes_partial_exact_and_change() {
    assert_eq!(preview_payment(25000, &[], 10000, 1000).message, "still owed");
    assert_eq!(preview_payment(25000, &[], 25000, 1000).message, "paid in full");
    assert_eq!(preview_payment(25000, &[], 30000, 1000).message, "change to return");
}

#[test]
fn bill_totals_keep_expected_equal_to_applied_plus_outstanding() {
    let shares = vec![
        share("a", "Ana", 25000, &[payment("p1", 30000, 1000)]),
        share("b", "Ben", 25000, &[payment("p2", 10000, 2000)]),
        share("c", "Cara", 25000, &[]),
    ];
    let summary = compute_bill_summary(&shares, None);

    assert_eq!(summary.expected_minor, 75000);
    assert_eq!(summary.applied_minor, 35000);
    assert_eq!(summary.outstanding_minor, 40000);
    assert_eq!(
        summary.applied_minor + summary.outstanding_minor,
        summary.expected_minor
    );
}

#[test]
fn bill_counts_paid_shares_with_change_separately() {
    let shares = vec![
        share("a", "Ana", 25000, &[payment("p1", 30000, 1000)]),
        share("b", "Ben", 25000, &[payment("p2", 10000, 2000)]),
        share("c", "Cara", 25000, &[]),
    ];
    let summary = compute_bill_summary(&shares, None);

    assert_eq!(summary.paid_share_count, 1);
    assert_eq!(summary.partial_share_count, 1);
    assert_eq!(summary.unpaid_share_count, 1);
    assert_eq!(summary.change_outstanding_minor, 5000);
    assert_eq!(summary.status, treasurer_core::models::BillStatus::Open);
}

#[test]
fn filters_classify_shares_independently() {
    let shares = vec![
        share("a", "Ana", 25000, &[payment("p1", 30000, 1000)]),
        share("b", "Ben", 25000, &[payment("p2", 10000, 2000)]),
    ];
    assert!(matches_share_filter(&shares[0], ShareFilter::Paid));
    assert!(matches_share_filter(&shares[0], ShareFilter::Change));
    assert!(matches_share_filter(&shares[1], ShareFilter::Unpaid));
    assert!(!matches_share_filter(&shares[1], ShareFilter::Paid));
}

#[test]
fn archived_bills_have_archived_status() {
    let shares = vec![share("a", "Ana", 25000, &[])];
    assert_eq!(
        compute_bill_summary(&shares, Some(123)).status,
        treasurer_core::models::BillStatus::Archived
    );
}

#[test]
fn bills_settle_when_every_share_is_covered_even_with_change() {
    let shares = vec![
        share("a", "Ana", 25000, &[payment("p1", 30000, 1000)]),
        share("b", "Ben", 25000, &[payment("p2", 25000, 2000)]),
    ];
    let summary = compute_bill_summary(&shares, None);
    assert_eq!(summary.status, treasurer_core::models::BillStatus::Settled);
    assert_eq!(summary.change_outstanding_minor, 5000);
}

#[test]
fn boarder_summaries_aggregate_across_bills() {
    let first = share("a", "Ana", 25000, &[payment("p1", 30000, 1000)]);
    let second_state = compute_share_state(10000, &[payment("p3", 5000, 3000)]);

    let summaries = compute_boarder_period_summaries(&[
        BoarderPeriodSummaryInput {
            boarder_id: "a",
            boarder_name: "Ana",
            boarder_archived: false,
            amount_due_minor: 25000,
            state: &first.state,
        },
        BoarderPeriodSummaryInput {
            boarder_id: "a",
            boarder_name: "Ana",
            boarder_archived: false,
            amount_due_minor: 10000,
            state: &second_state,
        },
    ]);

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].expected_minor, 35000);
    assert_eq!(summaries[0].applied_minor, 30000);
    assert_eq!(summaries[0].outstanding_minor, 5000);
    assert!(matches_boarder_filter(&summaries[0], ShareFilter::Unpaid));
}

fn history_bill(id: &str, year: i32, month: i32, created_at: i64) -> HistoryBillRef {
    HistoryBillRef {
        id: id.to_string(),
        bill_type_id: "water".to_string(),
        period_year: year,
        period_month: month,
        created_at,
    }
}

#[test]
fn history_filter_matches_every_bill_by_default() {
    let bills = vec![
        history_bill("a", 2026, 10, 1),
        history_bill("b", 2026, 11, 2),
    ];
    assert!(bills
        .iter()
        .all(|bill| matches_history_filter(bill, &HistoryFilter::all())));
}

#[test]
fn history_filter_filters_by_month_and_type() {
    let bills = vec![
        history_bill("a", 2026, 10, 1),
        history_bill("b", 2026, 11, 2),
    ];

    let november = HistoryFilter {
        year: Some(2026),
        month: Some(11),
        bill_type_id: None,
    };
    assert_eq!(
        bills
            .iter()
            .filter(|bill| matches_history_filter(bill, &november))
            .map(|bill| bill.id.as_str())
            .collect::<Vec<_>>(),
        vec!["b"]
    );
    assert!(!matches_history_filter(&bills[1], &HistoryFilter {
        year: Some(2026),
        month: Some(10),
        bill_type_id: None,
    }));
    assert!(matches_history_filter(&bills[1], &HistoryFilter {
        year: None,
        month: None,
        bill_type_id: Some("water".into()),
    }));
    assert!(!matches_history_filter(&bills[1], &HistoryFilter {
        year: None,
        month: None,
        bill_type_id: Some("rent".into()),
    }));
}

#[test]
fn history_sort_is_newest_period_then_newest_created() {
    let bills = vec![
        history_bill("a", 2026, 10, 1),
        history_bill("b", 2026, 11, 2),
        history_bill("c", 2025, 12, 3),
        history_bill("old", 2026, 11, 1),
        history_bill("new", 2026, 11, 5),
    ];

    let sorted = sort_history_bills(&bills);
    assert_eq!(
        sorted.iter().map(|bill| bill.id.as_str()).collect::<Vec<_>>(),
        vec!["new", "b", "old", "a", "c"]
    );
    assert_eq!(
        bills.iter().map(|bill| bill.id.as_str()).collect::<Vec<_>>(),
        vec!["a", "b", "c", "old", "new"]
    );
}

#[test]
fn history_periods_are_listed_newest_first_with_counts() {
    let periods = list_history_periods(&[
        history_bill("a", 2026, 10, 1),
        history_bill("b", 2026, 11, 2),
        history_bill("c", 2025, 12, 3),
        history_bill("d", 2026, 11, 4),
    ]);

    assert_eq!(periods.len(), 3);
    assert_eq!((periods[0].year, periods[0].month, periods[0].bill_count), (2026, 11, 2));
    assert_eq!((periods[1].year, periods[1].month, periods[1].bill_count), (2026, 10, 1));
    assert_eq!((periods[2].year, periods[2].month, periods[2].bill_count), (2025, 12, 1));
}

#[test]
fn history_totals_sum_and_default_to_zero() {
    let totals = compute_history_totals(&[
        HistoryTotals {
            expected_minor: 1000,
            applied_minor: 600,
            outstanding_minor: 400,
            change_outstanding_minor: 0,
        },
        HistoryTotals {
            expected_minor: 500,
            applied_minor: 500,
            outstanding_minor: 0,
            change_outstanding_minor: 50,
        },
    ]);
    assert_eq!(
        totals,
        HistoryTotals {
            expected_minor: 1500,
            applied_minor: 1100,
            outstanding_minor: 400,
            change_outstanding_minor: 50,
        }
    );
    assert_eq!(compute_history_totals(&[]), HistoryTotals::default());
}
