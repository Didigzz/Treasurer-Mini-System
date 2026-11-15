use treasurer_core::calculations::{
    resplit_with_pinned, validate_additional_charge, SplitOutcome,
};
use treasurer_core::db::repository::Treasurer;
use treasurer_core::models::*;

fn boarder(id: &str, name: &str) -> SplitBoarder {
    SplitBoarder {
        boarder_id: id.to_string(),
        name: name.to_string(),
        created_at: 0,
    }
}

fn three_boarders() -> Vec<SplitBoarder> {
    vec![
        boarder("maria", "Maria"),
        boarder("juan", "Juan"),
        boarder("ana", "Ana"),
    ]
}

fn amounts(outcome: SplitOutcome) -> Vec<i64> {
    match outcome {
        SplitOutcome::Ok(success) => success
            .shares
            .iter()
            .map(|share| share.amount_minor)
            .collect(),
        SplitOutcome::Err(failure) => panic!("{}", failure.error),
    }
}

fn error_message(outcome: SplitOutcome) -> String {
    match outcome {
        SplitOutcome::Err(failure) => failure.error,
        SplitOutcome::Ok(_) => panic!("expected a split error"),
    }
}

#[test]
fn resplit_pins_one_share_and_distributes_the_rest_equally() {
    let outcome = resplit_with_pinned(30_000, &three_boarders(), "maria", 5_000, true);
    assert_eq!(amounts(outcome), vec![5_000, 12_500, 12_500]);
}

#[test]
fn resplit_keeps_bill_order_when_the_pinned_share_is_not_first() {
    let outcome = resplit_with_pinned(30_000, &three_boarders(), "juan", 6_000, true);
    assert_eq!(amounts(outcome), vec![12_000, 6_000, 12_000]);
}

#[test]
fn resplit_allows_a_zero_share() {
    let outcome = resplit_with_pinned(30_000, &three_boarders(), "ana", 0, true);
    assert_eq!(amounts(outcome), vec![15_000, 15_000, 0]);
}

#[test]
fn resplit_hands_leftover_centavos_to_the_earliest_non_pinned_shares() {
    let outcome = resplit_with_pinned(1_000, &three_boarders(), "maria", 997, true);
    assert_eq!(amounts(outcome), vec![997, 2, 1]);

    let exact = resplit_with_pinned(1_000, &three_boarders(), "maria", 998, true);
    assert_eq!(amounts(exact), vec![998, 1, 1]);
}

#[test]
fn resplit_rejects_a_single_share_below_the_total() {
    let only = vec![boarder("ana", "Ana")];
    assert_eq!(
        error_message(resplit_with_pinned(10_000, &only, "ana", 9_000, true)),
        "The only share must equal the bill total."
    );
}

#[test]
fn resplit_rejects_taking_the_whole_total_from_one_of_several_shares() {
    assert_eq!(
        error_message(resplit_with_pinned(30_000, &three_boarders(), "maria", 30_000, true)),
        "Leave room for the other boarders."
    );
    assert_eq!(
        error_message(resplit_with_pinned(30_000, &three_boarders(), "maria", 40_000, true)),
        "Leave room for the other boarders."
    );
}

#[test]
fn resplit_rejects_a_remainder_too_small_for_the_other_shares() {
    assert_eq!(
        error_message(resplit_with_pinned(1_000, &three_boarders(), "maria", 999, false)),
        "Whole pesos only. Turn on centavos to use decimals."
    );

    let outcome = resplit_with_pinned(10_200, &three_boarders(), "maria", 10_000, false);
    assert_eq!(amounts(outcome), vec![10_000, 100, 100]);

    let too_small = resplit_with_pinned(10_100, &three_boarders(), "maria", 10_000, false);
    assert_eq!(
        error_message(too_small),
        "The remaining amount can't be split among the other boarders."
    );
}

#[test]
fn resplit_rejects_a_fractional_pin_with_centavos_off() {
    assert_eq!(
        error_message(resplit_with_pinned(30_000, &three_boarders(), "maria", 5_050, false)),
        "Whole pesos only. Turn on centavos to use decimals."
    );
}

#[test]
fn resplit_rejects_a_share_that_is_not_on_the_bill() {
    assert_eq!(
        error_message(resplit_with_pinned(30_000, &three_boarders(), "ghost", 5_000, true)),
        "That share is no longer available."
    );
}

#[test]
fn additional_charge_validation_follows_the_centavos_setting() {
    assert_eq!(validate_additional_charge(0, true), None);
    assert_eq!(validate_additional_charge(6_500, true), None);
    assert_eq!(
        validate_additional_charge(-1, true).as_deref(),
        Some("Additional charges cannot be negative.")
    );
    assert_eq!(
        validate_additional_charge(6_550, false).as_deref(),
        Some("Whole pesos only. Turn on centavos to use decimals.")
    );
    assert_eq!(
        validate_additional_charge(treasurer_core::money::MAX_AMOUNT_MINOR + 1, true).as_deref(),
        Some("That amount is too large.")
    );
}

// ---------------------------------------------------------------------------
// Storage
// ---------------------------------------------------------------------------

struct Fixture {
    db: Treasurer,
    maria: String,
    juan: String,
    ana: String,
    bill_id: String,
}

fn three_way_bill() -> Fixture {
    let db = Treasurer::open_in_memory().unwrap();
    let maria = db.create_boarder("Maria").unwrap();
    let juan = db.create_boarder("Juan").unwrap();
    let ana = db.create_boarder("Ana").unwrap();
    let bill_id = db
        .create_bill(&CreateBillInput {
            bill_type_id: "type-rent".to_string(),
            period_year: 2026,
            period_month: 11,
            total_minor: 30_000,
            due_date: None,
            note: None,
            shares: vec![
                SplitResult {
                    boarder_id: maria.clone(),
                    amount_minor: 10_000,
                },
                SplitResult {
                    boarder_id: juan.clone(),
                    amount_minor: 10_000,
                },
                SplitResult {
                    boarder_id: ana.clone(),
                    amount_minor: 10_000,
                },
            ],
        })
        .unwrap();
    Fixture {
        db,
        maria,
        juan,
        ana,
        bill_id,
    }
}

fn current_split(fixture: &Fixture) -> Vec<SplitResult> {
    fixture
        .db
        .bill_detail(&fixture.bill_id)
        .unwrap()
        .unwrap()
        .shares
        .iter()
        .map(|share| SplitResult {
            boarder_id: share.share.boarder_id.clone(),
            amount_minor: share.share.amount_due_minor,
        })
        .collect()
}

fn pinned_split(fixture: &Fixture, pinned_id: &str, amount: i64) -> Vec<SplitResult> {
    let detail = fixture
        .db
        .bill_detail(&fixture.bill_id)
        .unwrap()
        .unwrap();
    let boarders: Vec<SplitBoarder> = detail
        .shares
        .iter()
        .map(|share| SplitBoarder {
            boarder_id: share.share.boarder_id.clone(),
            name: share.display_name().to_string(),
            created_at: 0,
        })
        .collect();
    match resplit_with_pinned(
        detail.bill.total_minor,
        &boarders,
        pinned_id,
        amount,
        true,
    ) {
        SplitOutcome::Ok(success) => success.shares,
        SplitOutcome::Err(failure) => panic!("{}", failure.error),
    }
}

fn base_due(fixture: &Fixture, boarder_id: &str) -> i64 {
    fixture
        .db
        .bill_detail(&fixture.bill_id)
        .unwrap()
        .unwrap()
        .shares
        .iter()
        .find(|share| share.share.boarder_id == boarder_id)
        .unwrap()
        .share
        .amount_due_minor
}

#[test]
fn saving_a_pinned_split_redistributes_the_other_boarders() {
    let fixture = three_way_bill();
    let split = pinned_split(&fixture, &fixture.maria, 5_000);

    fixture
        .db
        .save_share_edit(&fixture.bill_id, &split, &fixture.maria, 0)
        .unwrap();

    let detail = fixture.db.bill_detail(&fixture.bill_id).unwrap().unwrap();
    assert_eq!(detail.bill.total_minor, 30_000);
    assert_eq!(base_due(&fixture, &fixture.maria), 5_000);
    assert_eq!(base_due(&fixture, &fixture.juan), 12_500);
    assert_eq!(base_due(&fixture, &fixture.ana), 12_500);
    assert_eq!(detail.shares.len(), 3);
}

#[test]
fn a_resplit_also_normalizes_a_previously_custom_share() {
    let fixture = three_way_bill();
    let first = pinned_split(&fixture, &fixture.juan, 6_000);
    fixture
        .db
        .save_share_edit(&fixture.bill_id, &first, &fixture.juan, 0)
        .unwrap();
    assert_eq!(base_due(&fixture, &fixture.maria), 12_000);

    let second = pinned_split(&fixture, &fixture.maria, 5_000);
    fixture
        .db
        .save_share_edit(&fixture.bill_id, &second, &fixture.maria, 0)
        .unwrap();

    assert_eq!(base_due(&fixture, &fixture.maria), 5_000);
    assert_eq!(base_due(&fixture, &fixture.juan), 12_500);
    assert_eq!(base_due(&fixture, &fixture.ana), 12_500);
}

#[test]
fn an_additional_charge_raises_expected_and_balance_but_not_the_total() {
    let fixture = three_way_bill();
    let split = current_split(&fixture);
    fixture
        .db
        .save_share_edit(&fixture.bill_id, &split, &fixture.maria, 6_500)
        .unwrap();

    let detail = fixture.db.bill_detail(&fixture.bill_id).unwrap().unwrap();
    assert_eq!(detail.bill.total_minor, 30_000);
    assert_eq!(detail.summary.expected_minor, 36_500);
    assert_eq!(detail.summary.outstanding_minor, 36_500);

    let maria = detail
        .shares
        .iter()
        .find(|share| share.share.boarder_id == fixture.maria)
        .unwrap();
    assert_eq!(maria.share.amount_due_minor, 10_000);
    assert_eq!(maria.share.additional_charge_minor, 6_500);
    assert_eq!(maria.share.total_due_minor(), 16_500);
    assert_eq!(maria.state.balance_minor, 16_500);

    for boarder_id in [&fixture.juan, &fixture.ana] {
        let share = detail
            .shares
            .iter()
            .find(|share| &share.share.boarder_id == boarder_id)
            .unwrap();
        assert_eq!(share.share.additional_charge_minor, 0);
        assert_eq!(share.state.balance_minor, 10_000);
    }

    let dashboard = fixture.db.dashboard(2026, 11, false).unwrap();
    assert_eq!(dashboard.totals.expected_minor, 36_500);
    assert_eq!(dashboard.totals.outstanding_minor, 36_500);

    let maria_summary = dashboard
        .boarder_summaries
        .iter()
        .find(|summary| summary.boarder_id == fixture.maria)
        .unwrap();
    assert_eq!(maria_summary.expected_minor, 16_500);
    assert_eq!(maria_summary.outstanding_minor, 16_500);

    let ledger = fixture.db.boarder_ledger(&fixture.maria).unwrap().unwrap();
    assert_eq!(ledger.summary.expected_minor, 16_500);
    assert_eq!(ledger.summary.outstanding_minor, 16_500);

    let juan_ledger = fixture.db.boarder_ledger(&fixture.juan).unwrap().unwrap();
    assert_eq!(juan_ledger.summary.expected_minor, 10_000);

    let balances = fixture.db.list_boarders_with_balances().unwrap();
    let maria_balance = balances
        .iter()
        .find(|row| row.boarder.id == fixture.maria)
        .unwrap();
    assert_eq!(maria_balance.outstanding_minor, 16_500);
}

#[test]
fn payments_apply_against_the_charged_total() {
    let fixture = three_way_bill();
    let detail = fixture.db.bill_detail(&fixture.bill_id).unwrap().unwrap();
    let maria_share = detail
        .shares
        .iter()
        .find(|share| share.share.boarder_id == fixture.maria)
        .unwrap()
        .share
        .id
        .clone();

    fixture
        .db
        .save_share_edit(&fixture.bill_id, &current_split(&fixture), &fixture.maria, 6_500)
        .unwrap();

    fixture
        .db
        .record_payment(&CreatePaymentInput {
            bill_share_id: maria_share,
            amount_minor: 10_000,
            paid_at: 1_760_000_000_000,
            note: None,
        })
        .unwrap();

    let detail = fixture.db.bill_detail(&fixture.bill_id).unwrap().unwrap();
    let maria = detail
        .shares
        .iter()
        .find(|share| share.share.boarder_id == fixture.maria)
        .unwrap();
    assert_eq!(maria.state.balance_minor, 6_500);
    assert_eq!(detail.summary.outstanding_minor, 26_500);
}

#[test]
fn locked_shares_keep_their_split_but_accept_charge_changes() {
    let fixture = three_way_bill();
    let detail = fixture.db.bill_detail(&fixture.bill_id).unwrap().unwrap();
    let maria_share = detail
        .shares
        .iter()
        .find(|share| share.share.boarder_id == fixture.maria)
        .unwrap()
        .share
        .id
        .clone();
    fixture
        .db
        .record_payment(&CreatePaymentInput {
            bill_share_id: maria_share,
            amount_minor: 5_000,
            paid_at: 1_760_000_000_000,
            note: None,
        })
        .unwrap();

    let changed = pinned_split(&fixture, &fixture.maria, 4_000);
    let error = fixture
        .db
        .save_share_edit(&fixture.bill_id, &changed, &fixture.maria, 0)
        .unwrap_err();
    assert!(format!("{error:#}").contains("locked"));

    fixture
        .db
        .save_share_edit(&fixture.bill_id, &current_split(&fixture), &fixture.maria, 2_000)
        .unwrap();
    let detail = fixture.db.bill_detail(&fixture.bill_id).unwrap().unwrap();
    let maria = detail
        .shares
        .iter()
        .find(|share| share.share.boarder_id == fixture.maria)
        .unwrap();
    assert_eq!(maria.share.amount_due_minor, 10_000);
    assert_eq!(maria.state.balance_minor, 7_000);
}

#[test]
fn charging_a_settled_bill_reopens_it() {
    let db = Treasurer::open_in_memory().unwrap();
    let ana = db.create_boarder("Ana").unwrap();
    let bill_id = db
        .create_bill(&CreateBillInput {
            bill_type_id: "type-wifi".to_string(),
            period_year: 2026,
            period_month: 11,
            total_minor: 10_000,
            due_date: None,
            note: None,
            shares: vec![SplitResult {
                boarder_id: ana.clone(),
                amount_minor: 10_000,
            }],
        })
        .unwrap();

    let detail = db.bill_detail(&bill_id).unwrap().unwrap();
    db.record_payment(&CreatePaymentInput {
        bill_share_id: detail.shares[0].share.id.clone(),
        amount_minor: 10_000,
        paid_at: 1_760_000_000_000,
        note: None,
    })
    .unwrap();
    assert_eq!(
        db.bill_detail(&bill_id).unwrap().unwrap().summary.status,
        BillStatus::Settled
    );

    db.save_share_edit(
        &bill_id,
        &[SplitResult {
            boarder_id: ana,
            amount_minor: 10_000,
        }],
        &detail.shares[0].share.boarder_id,
        500,
    )
    .unwrap();

    let detail = db.bill_detail(&bill_id).unwrap().unwrap();
    assert_eq!(detail.summary.status, BillStatus::Open);
    assert_eq!(detail.summary.outstanding_minor, 500);
}

#[test]
fn a_legacy_bill_shares_table_gains_the_charge_column() {
    let db = Treasurer::open_in_memory().unwrap();
    let ana = db.create_boarder("Ana").unwrap();
    let bill_id = db
        .create_bill(&CreateBillInput {
            bill_type_id: "type-water".to_string(),
            period_year: 2026,
            period_month: 11,
            total_minor: 10_000,
            due_date: None,
            note: None,
            shares: vec![SplitResult {
                boarder_id: ana.clone(),
                amount_minor: 10_000,
            }],
        })
        .unwrap();

    db.conn
        .execute("ALTER TABLE bill_shares DROP COLUMN additional_charge_minor", [])
        .unwrap();
    treasurer_core::db::initialize(&db.conn).unwrap();

    let detail = db.bill_detail(&bill_id).unwrap().unwrap();
    assert_eq!(detail.shares[0].share.additional_charge_minor, 0);

    db.save_share_edit(
        &bill_id,
        &[SplitResult {
            boarder_id: ana,
            amount_minor: 10_000,
        }],
        &detail.shares[0].share.boarder_id,
        700,
    )
    .unwrap();
    assert_eq!(
        db.bill_detail(&bill_id)
            .unwrap()
            .unwrap()
            .summary
            .expected_minor,
        10_700
    );
}

#[test]
fn charges_survive_a_backup_round_trip_and_old_backups_default_to_zero() {
    let fixture = three_way_bill();
    fixture
        .db
        .save_share_edit(
            &fixture.bill_id,
            &current_split(&fixture),
            &fixture.maria,
            6_500,
        )
        .unwrap();

    let json = fixture.db.export_backup_json().unwrap();
    assert!(json.contains("additionalChargeMinor"));

    let restored = Treasurer::open_in_memory().unwrap();
    restored.import_backup_json(&json).unwrap();
    let detail = restored.bill_detail(&fixture.bill_id).unwrap().unwrap();
    let maria = detail
        .shares
        .iter()
        .find(|share| share.share.boarder_id == fixture.maria)
        .unwrap();
    assert_eq!(maria.share.additional_charge_minor, 6_500);
    assert_eq!(detail.summary.expected_minor, 36_500);

    let mut old = serde_json::from_str::<serde_json::Value>(&json).unwrap();
    for share in old["data"]["billShares"].as_array_mut().unwrap() {
        share
            .as_object_mut()
            .unwrap()
            .remove("additionalChargeMinor");
    }

    let legacy = Treasurer::open_in_memory().unwrap();
    legacy.import_backup_json(&old.to_string()).unwrap();
    let detail = legacy.bill_detail(&fixture.bill_id).unwrap().unwrap();
    assert!(detail
        .shares
        .iter()
        .all(|share| share.share.additional_charge_minor == 0));
    assert_eq!(detail.summary.expected_minor, 30_000);
}
