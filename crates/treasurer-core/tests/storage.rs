use serde_json::json;
use treasurer_core::backup::{backup_counts, parse_backup_json};
use treasurer_core::db::repository::Treasurer;
use treasurer_core::models::*;

fn setup() -> (Treasurer, String, String) {
    let db = Treasurer::open_in_memory().unwrap();
    let ana = db.create_boarder("Ana").unwrap();
    let ben = db.create_boarder("Ben").unwrap();
    (db, ana, ben)
}

fn create_rent_bill(db: &Treasurer, ana: &str, ben: &str) -> String {
    db.create_bill(&CreateBillInput {
        bill_type_id: "type-rent".to_string(),
        period_year: 2026,
        period_month: 11,
        total_minor: 100_000,
        due_date: Some("2026-11-30".to_string()),
        note: Some("November rent".to_string()),
        shares: vec![
            SplitResult {
                boarder_id: ana.to_string(),
                amount_minor: 60_000,
            },
            SplitResult {
                boarder_id: ben.to_string(),
                amount_minor: 40_000,
            },
        ],
    })
    .unwrap()
}

#[test]
fn seeds_default_bill_types_and_settings() {
    let db = Treasurer::open_in_memory().unwrap();
    let types = db.list_bill_types().unwrap();
    let names: Vec<&str> = types
        .iter()
        .map(|row| row.bill_type.name.as_str())
        .collect();
    assert_eq!(names, vec!["Water", "Electricity", "Rent", "Wi-Fi", "Other"]);

    let settings = db.app_settings().unwrap();
    assert_eq!(settings.currency_code, "PHP");
    assert!(settings.allow_centavos);
    assert_eq!(settings.last_backup_at, None);
}

#[test]
fn creating_a_bill_splits_and_locks_money_after_a_payment() {
    let (db, ana, ben) = setup();
    let bill_id = create_rent_bill(&db, &ana, &ben);

    let detail = db.bill_detail(&bill_id).unwrap().unwrap();
    assert_eq!(detail.shares.len(), 2);
    assert_eq!(detail.summary.expected_minor, 100_000);
    assert!(!detail.is_locked);

    let share_id = detail
        .shares
        .iter()
        .find(|share| share.share.boarder_id == ana)
        .unwrap()
        .share
        .id
        .clone();

    db.record_payment(&CreatePaymentInput {
        bill_share_id: share_id,
        amount_minor: 60_000,
        paid_at: 1_760_000_000_000,
        note: None,
        payment_type: PaymentType::Cash,
        used_full_balance: false,
    })
    .unwrap();

    let detail = db.bill_detail(&bill_id).unwrap().unwrap();
    assert!(detail.is_locked);
    assert_eq!(detail.summary.applied_minor, 60_000);
    assert_eq!(detail.summary.outstanding_minor, 40_000);

    let update = db.update_bill(&bill_id, &CreateBillInput {
        bill_type_id: "type-rent".to_string(),
        period_year: 2026,
        period_month: 11,
        total_minor: 100_000,
        due_date: None,
        note: None,
        shares: vec![
            SplitResult { boarder_id: ana.clone(), amount_minor: 50_000 },
            SplitResult { boarder_id: ben.clone(), amount_minor: 50_000 },
        ],
    });
    assert!(update.is_err());
}

#[test]
fn overpayment_creates_change_and_marking_it_returned_clears_it() {
    let (db, ana, ben) = setup();
    let bill_id = create_rent_bill(&db, &ana, &ben);
    let detail = db.bill_detail(&bill_id).unwrap().unwrap();
    let share_id = detail
        .shares
        .iter()
        .find(|share| share.share.boarder_id == ana)
        .unwrap()
        .share
        .id
        .clone();

    let payment_id = db
        .record_payment(&CreatePaymentInput {
            bill_share_id: share_id,
            amount_minor: 70_000,
            paid_at: 1_760_000_000_000,
            note: Some("cash".to_string()),
            payment_type: PaymentType::Cash,
            used_full_balance: false,
        })
        .unwrap();

    let detail = db.bill_detail(&bill_id).unwrap().unwrap();
    assert_eq!(detail.summary.change_outstanding_minor, 10_000);
    assert_eq!(detail.summary.cash_received_minor, 70_000);
    assert_eq!(detail.summary.paid_share_count, 1);

    db.mark_change_returned(&payment_id, 1_760_000_100_000, Some("handed back"))
        .unwrap();

    let detail = db.bill_detail(&bill_id).unwrap().unwrap();
    assert_eq!(detail.summary.change_outstanding_minor, 0);
    assert_eq!(detail.summary.change_returned_minor, 10_000);
}

fn share_id_for(db: &Treasurer, bill_id: &str, boarder_id: &str) -> String {
    db.bill_detail(bill_id)
        .unwrap()
        .unwrap()
        .shares
        .iter()
        .find(|share| share.share.boarder_id == boarder_id)
        .unwrap()
        .share
        .id
        .clone()
}

#[test]
fn undoing_a_payment_recomputes_totals_and_keeps_the_row() {
    let (db, ana, ben) = setup();
    let bill_id = create_rent_bill(&db, &ana, &ben);
    let share_id = share_id_for(&db, &bill_id, &ana);

    let payment_id = db
        .record_payment(&CreatePaymentInput {
            bill_share_id: share_id.clone(),
            amount_minor: 70_000,
            paid_at: 1_760_000_000_000,
            note: None,
            payment_type: PaymentType::Cash,
            used_full_balance: false,
        })
        .unwrap();

    db.undo_payment(&payment_id).unwrap();

    let detail = db.bill_detail(&bill_id).unwrap().unwrap();
    assert_eq!(detail.summary.applied_minor, 0);
    assert_eq!(detail.summary.change_outstanding_minor, 0);
    assert!(!detail.is_locked);

    let payment = detail
        .shares
        .iter()
        .flat_map(|share| share.state.payments.iter())
        .find(|payment| payment.payment.id == payment_id)
        .unwrap();
    assert!(payment.payment.voided_at.is_some());
    assert_eq!(payment.payment.void_reason, None);
    assert_eq!(payment.payment.amount_minor, 70_000);
    assert_eq!(payment.payment.paid_at, 1_760_000_000_000);
    assert_eq!(payment.payment.change_minor, 0);

    let share = detail
        .shares
        .iter()
        .find(|share| share.share.id == share_id)
        .unwrap();
    assert_eq!(share.state.status, ShareStatus::Unpaid);
    assert_eq!(share.state.balance_minor, 60_000);

    // A second undo of the same payment is rejected.
    let error = db.undo_payment(&payment_id).unwrap_err().to_string();
    assert!(error.contains("already undone"), "unexpected error: {error}");
}

#[test]
fn undoing_one_of_two_payments_leaves_the_share_partial() {
    let (db, ana, ben) = setup();
    let bill_id = create_rent_bill(&db, &ana, &ben);
    let share_id = share_id_for(&db, &bill_id, &ana);

    // Written before payments had to cover the balance, so it is inserted
    // directly: a legacy partial share stays readable and completable.
    db.conn
        .execute(
            "INSERT INTO payments (id, bill_share_id, amount_minor, paid_at, note, change_minor,
                                   change_returned_at, change_return_note, voided_at, void_reason,
                                   created_at, payment_type, used_full_balance)
             VALUES ('legacy-partial', ?1, 30000, 1760000000000, NULL, 0, NULL, NULL, NULL, NULL,
                     1760000000000, 'cash', 0)",
            [share_id.as_str()],
        )
        .unwrap();
    let first = "legacy-partial".to_string();
    let second = db
        .record_payment(&CreatePaymentInput {
            bill_share_id: share_id.clone(),
            amount_minor: 40_000,
            paid_at: 1_760_000_100_000,
            note: None,
            payment_type: PaymentType::Cash,
            used_full_balance: false,
        })
        .unwrap();

    db.undo_payment(&second).unwrap();

    let share = db
        .bill_detail(&bill_id)
        .unwrap()
        .unwrap()
        .shares
        .into_iter()
        .find(|share| share.share.id == share_id)
        .unwrap();
    assert_eq!(share.state.status, ShareStatus::Partial);
    assert_eq!(share.state.paid_minor, 30_000);
    assert_eq!(share.state.balance_minor, 30_000);

    let remaining = share
        .state
        .payments
        .iter()
        .find(|item| item.payment.id == first)
        .unwrap();
    assert_eq!(remaining.computed_change_minor, 0);
    assert_eq!(remaining.payment.change_minor, 0);
}

#[test]
fn undoing_an_overpayment_clears_the_change_obligation() {
    let (db, ana, ben) = setup();
    let bill_id = create_rent_bill(&db, &ana, &ben);
    let share_id = share_id_for(&db, &bill_id, &ana);

    let payment_id = db
        .record_payment(&CreatePaymentInput {
            bill_share_id: share_id,
            amount_minor: 70_000,
            paid_at: 1_760_000_000_000,
            note: None,
            payment_type: PaymentType::Cash,
            used_full_balance: false,
        })
        .unwrap();

    assert_eq!(
        db.bill_detail(&bill_id)
            .unwrap()
            .unwrap()
            .summary
            .change_outstanding_minor,
        10_000
    );

    db.undo_payment(&payment_id).unwrap();

    let detail = db.bill_detail(&bill_id).unwrap().unwrap();
    assert_eq!(detail.summary.change_outstanding_minor, 0);
    assert_eq!(detail.summary.change_returned_minor, 0);
    let payment = detail
        .shares
        .iter()
        .flat_map(|share| share.state.payments.iter())
        .find(|item| item.payment.id == payment_id)
        .unwrap();
    assert_eq!(payment.payment.change_minor, 0);
}

#[test]
fn returned_change_blocks_undo() {
    let (db, ana, ben) = setup();
    let bill_id = create_rent_bill(&db, &ana, &ben);
    let share_id = share_id_for(&db, &bill_id, &ana);

    let payment_id = db
        .record_payment(&CreatePaymentInput {
            bill_share_id: share_id,
            amount_minor: 70_000,
            paid_at: 1_760_000_000_000,
            note: None,
            payment_type: PaymentType::Cash,
            used_full_balance: false,
        })
        .unwrap();
    db.mark_change_returned(&payment_id, 1_760_000_100_000, None)
        .unwrap();

    let error = db.undo_payment(&payment_id).unwrap_err().to_string();
    assert!(
        error.contains("change was already returned"),
        "unexpected error: {error}"
    );
}

#[test]
fn an_archived_bill_blocks_undo_until_it_is_restored() {
    let (db, ana, ben) = setup();
    let bill_id = create_rent_bill(&db, &ana, &ben);
    let share_id = share_id_for(&db, &bill_id, &ana);

    let payment_id = db
        .record_payment(&CreatePaymentInput {
            bill_share_id: share_id,
            amount_minor: 60_000,
            paid_at: 1_760_000_000_000,
            note: None,
            payment_type: PaymentType::Cash,
            used_full_balance: false,
        })
        .unwrap();

    db.set_bill_archived(&bill_id, true).unwrap();
    let error = db.undo_payment(&payment_id).unwrap_err().to_string();
    assert!(
        error.contains("Restore this bill"),
        "unexpected error: {error}"
    );

    db.set_bill_archived(&bill_id, false).unwrap();
    db.undo_payment(&payment_id).unwrap();
    assert_eq!(
        db.bill_detail(&bill_id)
            .unwrap()
            .unwrap()
            .summary
            .applied_minor,
        0
    );
}

#[test]
fn a_legacy_void_reason_survives_undo_labelling_and_backup() {
    let (db, ana, ben) = setup();
    let bill_id = create_rent_bill(&db, &ana, &ben);
    let share_id = share_id_for(&db, &bill_id, &ana);

    let payment_id = db
        .record_payment(&CreatePaymentInput {
            bill_share_id: share_id,
            amount_minor: 70_000,
            paid_at: 1_760_000_000_000,
            note: None,
            payment_type: PaymentType::Cash,
            used_full_balance: false,
        })
        .unwrap();

    // Rows written by the old Void flow carry a stored reason.
    db.conn
        .execute(
            "UPDATE payments SET voided_at = 1, void_reason = 'Recorded twice' WHERE id = ?1",
            [&payment_id],
        )
        .unwrap();

    let detail = db.bill_detail(&bill_id).unwrap().unwrap();
    let payment = detail
        .shares
        .iter()
        .flat_map(|share| share.state.payments.iter())
        .find(|item| item.payment.id == payment_id)
        .unwrap();
    assert_eq!(payment.payment.void_reason.as_deref(), Some("Recorded twice"));
    assert!(payment.payment.voided_at.is_some());
    assert_eq!(detail.summary.applied_minor, 0);

    let json = db.export_backup_json().unwrap();
    let restored = Treasurer::open_in_memory().unwrap();
    restored.import_backup_json(&json).unwrap();

    let detail = restored.bill_detail(&bill_id).unwrap().unwrap();
    let payment = detail
        .shares
        .iter()
        .flat_map(|share| share.state.payments.iter())
        .find(|item| item.payment.id == payment_id)
        .unwrap();
    assert_eq!(payment.payment.void_reason.as_deref(), Some("Recorded twice"));
    assert!(payment.payment.voided_at.is_some());
    assert_eq!(detail.summary.applied_minor, 0);
}

#[test]
fn dashboard_totals_cover_the_selected_month_only() {
    let (db, ana, ben) = setup();
    create_rent_bill(&db, &ana, &ben);

    let november = db.dashboard(2026, 11, false).unwrap();
    assert_eq!(november.totals.expected_minor, 100_000);
    assert_eq!(november.bills.len(), 1);
    assert_eq!(november.boarder_summaries.len(), 2);

    let december = db.dashboard(2026, 12, false).unwrap();
    assert_eq!(december.totals.expected_minor, 0);
    assert!(december.bills.is_empty());
}

#[test]
fn archiving_a_boarder_blocks_new_bills_but_keeps_history() {
    let (db, ana, _ben) = setup();
    db.set_boarder_archived(&ana, true).unwrap();

    let result = db.create_bill(&CreateBillInput {
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
    });
    assert!(result.is_err());

    let ledger = db.boarder_ledger(&ana).unwrap().unwrap();
    assert_eq!(ledger.boarder.status, BoarderStatus::Archived);
}

#[test]
fn deleting_a_boarder_without_history_removes_them() {
    let (db, ana, _ben) = setup();
    db.delete_boarder(&ana).unwrap();

    assert!(db.get_boarder(&ana).unwrap().is_none());
    let names: Vec<String> = db
        .list_boarders_with_balances()
        .unwrap()
        .into_iter()
        .map(|row| row.boarder.name)
        .collect();
    assert_eq!(names, vec!["Ben".to_string()]);
}

#[test]
fn deleting_a_boarder_with_bill_history_is_refused() {
    let (db, ana, ben) = setup();
    create_rent_bill(&db, &ana, &ben);

    let result = db.delete_boarder(&ana);
    assert!(result.is_err());
    assert!(db.get_boarder(&ana).unwrap().is_some());
}

#[test]
fn backup_export_and_restore_round_trip_everything() {
    let (db, ana, ben) = setup();
    let bill_id = create_rent_bill(&db, &ana, &ben);
    let detail = db.bill_detail(&bill_id).unwrap().unwrap();
    let share_id = detail
        .shares
        .iter()
        .find(|share| share.share.boarder_id == ana)
        .unwrap()
        .share
        .id
        .clone();
    db.record_payment(&CreatePaymentInput {
        bill_share_id: share_id,
        amount_minor: 70_000,
        paid_at: 1_760_000_000_000,
        note: Some("cash".to_string()),
        payment_type: PaymentType::Cash,
        used_full_balance: false,
    })
    .unwrap();
    db.set_currency_code("USD").unwrap();
    db.set_allow_centavos(false).unwrap();

    let json = db.export_backup_json().unwrap();
    let payload = parse_backup_json(&json).unwrap();
    let counts = backup_counts(&payload);
    assert_eq!(counts.boarders, 2);
    assert_eq!(counts.bills, 1);
    assert_eq!(counts.shares, 2);
    assert_eq!(counts.payments, 1);

    let restored = Treasurer::open_in_memory().unwrap();
    restored.import_backup_json(&json).unwrap();

    let settings = restored.app_settings().unwrap();
    assert_eq!(settings.currency_code, "USD");
    assert!(!settings.allow_centavos);

    let dashboard = restored.dashboard(2026, 11, false).unwrap();
    assert_eq!(dashboard.totals.expected_minor, 100_000);
    assert_eq!(dashboard.totals.applied_minor, 60_000);
    assert_eq!(dashboard.totals.change_outstanding_minor, 10_000);

    let detail = restored.bill_detail(&bill_id).unwrap().unwrap();
    assert_eq!(detail.summary.cash_received_minor, 70_000);
    assert_eq!(detail.shares.len(), 2);
    assert_eq!(detail.bill.note.as_deref(), Some("November rent"));
}

#[test]
fn payment_type_and_full_balance_usage_persist_and_back_up() {
    let (db, ana, ben) = setup();
    let bill_id = create_rent_bill(&db, &ana, &ben);
    let detail = db.bill_detail(&bill_id).unwrap().unwrap();
    let share_id = detail
        .shares
        .iter()
        .find(|share| share.share.boarder_id == ana)
        .unwrap()
        .share
        .id
        .clone();
    db.record_payment(&CreatePaymentInput {
        bill_share_id: share_id,
        amount_minor: 60_000,
        paid_at: 1_760_000_000_000,
        note: None,
        payment_type: PaymentType::Online,
        used_full_balance: true,
    })
    .unwrap();

    let detail = db.bill_detail(&bill_id).unwrap().unwrap();
    let payment = &detail.shares[0].state.payments[0].payment;
    assert_eq!(payment.payment_type, PaymentType::Online);
    assert!(payment.used_full_balance);
    assert_eq!(payment.status_label(), "Paid – Online");

    let json = db.export_backup_json().unwrap();
    let restored = Treasurer::open_in_memory().unwrap();
    restored.import_backup_json(&json).unwrap();

    let detail = restored.bill_detail(&bill_id).unwrap().unwrap();
    let payment = &detail.shares[0].state.payments[0].payment;
    assert_eq!(payment.payment_type, PaymentType::Online);
    assert!(payment.used_full_balance);
}

#[test]
fn backup_validation_rejects_broken_payloads() {
    let (db, ana, ben) = setup();
    create_rent_bill(&db, &ana, &ben);
    let json = db.export_backup_json().unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    let mut wrong_format = value.clone();
    wrong_format["format"] = json!("something-else");
    assert!(parse_backup_json(&wrong_format.to_string()).is_err());

    let mut duplicate_boarder = value.clone();
    duplicate_boarder["data"]["boarders"][1]["id"] =
        duplicate_boarder["data"]["boarders"][0]["id"].clone();
    assert!(parse_backup_json(&duplicate_boarder.to_string()).is_err());

    let mut mismatched_total = value.clone();
    mismatched_total["data"]["bills"][0]["totalMinor"] = json!(999);
    assert!(parse_backup_json(&mismatched_total.to_string()).is_err());

    let mut missing_share = value.clone();
    missing_share["data"]["payments"] = json!([]);
    missing_share["data"]["bills"][0]["totalMinor"] = json!(1);
    assert!(parse_backup_json(&missing_share.to_string()).is_err());

    assert!(parse_backup_json("not json").is_err());
}

#[test]
fn a_payment_below_the_remaining_balance_is_rejected_and_writes_nothing() {
    let (db, ana, ben) = setup();
    let bill_id = create_rent_bill(&db, &ana, &ben);
    let share_id = share_id_for(&db, &bill_id, &ana);

    let error = db
        .record_payment(&CreatePaymentInput {
            bill_share_id: share_id.clone(),
            amount_minor: 50_000,
            paid_at: 1_760_000_000_000,
            note: None,
            payment_type: PaymentType::Cash,
            used_full_balance: false,
        })
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("Partial payments aren't allowed"),
        "unexpected error: {error}"
    );
    assert!(error.contains("60.00") || error.contains("600.00"), "unexpected error: {error}");

    let detail = db.bill_detail(&bill_id).unwrap().unwrap();
    let share = detail
        .shares
        .iter()
        .find(|share| share.share.id == share_id)
        .unwrap();
    assert!(share.state.payments.is_empty());
    assert_eq!(detail.summary.applied_minor, 0);

    // Exactly the remaining balance settles the share.
    db.record_payment(&CreatePaymentInput {
        bill_share_id: share_id.clone(),
        amount_minor: 60_000,
        paid_at: 1_760_000_000_000,
        note: None,
        payment_type: PaymentType::Cash,
        used_full_balance: true,
    })
    .unwrap();

    let share = db
        .bill_detail(&bill_id)
        .unwrap()
        .unwrap()
        .shares
        .into_iter()
        .find(|share| share.share.id == share_id)
        .unwrap();
    assert_eq!(share.state.status, ShareStatus::Paid);
    assert_eq!(share.state.balance_minor, 0);
}

#[test]
fn a_legacy_partial_share_can_only_be_completed() {
    let (db, ana, ben) = setup();
    let bill_id = create_rent_bill(&db, &ana, &ben);
    let share_id = share_id_for(&db, &bill_id, &ana);

    db.conn
        .execute(
            "INSERT INTO payments (id, bill_share_id, amount_minor, paid_at, note, change_minor,
                                   change_returned_at, change_return_note, voided_at, void_reason,
                                   created_at, payment_type, used_full_balance)
             VALUES ('legacy-partial', ?1, 40000, 1760000000000, NULL, 0, NULL, NULL, NULL, NULL,
                     1760000000000, 'cash', 0)",
            [share_id.as_str()],
        )
        .unwrap();

    let error = db
        .record_payment(&CreatePaymentInput {
            bill_share_id: share_id.clone(),
            amount_minor: 10_000,
            paid_at: 1_760_000_100_000,
            note: None,
            payment_type: PaymentType::Cash,
            used_full_balance: false,
        })
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("Partial payments aren't allowed"),
        "unexpected error: {error}"
    );

    db.record_payment(&CreatePaymentInput {
        bill_share_id: share_id.clone(),
        amount_minor: 20_000,
        paid_at: 1_760_000_100_000,
        note: None,
        payment_type: PaymentType::Cash,
        used_full_balance: false,
    })
    .unwrap();

    let share = db
        .bill_detail(&bill_id)
        .unwrap()
        .unwrap()
        .shares
        .into_iter()
        .find(|share| share.share.id == share_id)
        .unwrap();
    assert_eq!(share.state.status, ShareStatus::Paid);
    assert_eq!(share.state.paid_minor, 60_000);
}
