use treasurer_core::db::repository::Treasurer;
use treasurer_core::models::*;

fn setup(names: &[&str]) -> (Treasurer, Vec<String>) {
    let db = Treasurer::open_in_memory().unwrap();
    let ids = names
        .iter()
        .map(|name| db.create_boarder(name).unwrap())
        .collect();
    (db, ids)
}

fn create_bill(db: &Treasurer, ids: &[String], total_minor: i64) -> String {
    let count = ids.len() as i64;
    let base = total_minor / count;
    let remainder = total_minor - base * count;
    let shares = ids
        .iter()
        .enumerate()
        .map(|(index, id)| SplitResult {
            boarder_id: id.clone(),
            amount_minor: base + if (index as i64) < remainder { 1 } else { 0 },
        })
        .collect();

    db.create_bill(&CreateBillInput {
        bill_type_id: "type-rent".to_string(),
        period_year: 2026,
        period_month: 11,
        total_minor,
        due_date: Some("2026-11-30".to_string()),
        note: None,
        shares,
    })
    .unwrap()
}

fn base_split(detail: &BillWithDetails) -> Vec<SplitResult> {
    detail
        .shares
        .iter()
        .map(|share| SplitResult {
            boarder_id: share.share.boarder_id.clone(),
            amount_minor: share.share.amount_due_minor,
        })
        .collect()
}

#[test]
fn excluding_a_boarder_keeps_the_total_and_re_splits_evenly() {
    let (db, ids) = setup(&["Maria", "Juan", "Ana"]);
    let bill_id = create_bill(&db, &ids, 90_000);

    let detail = db.bill_detail(&bill_id).unwrap().unwrap();
    let split = base_split(&detail);
    db.save_share_edit(&bill_id, &split, &ids[0], 6_500).unwrap();

    db.exclude_share(&bill_id, &ids[2]).unwrap();

    let detail = db.bill_detail(&bill_id).unwrap().unwrap();
    assert_eq!(detail.bill.total_minor, 90_000);
    assert_eq!(detail.shares.len(), 2);
    assert_eq!(detail.summary.expected_minor, 96_500);

    let maria = detail
        .shares
        .iter()
        .find(|share| share.share.boarder_id == ids[0])
        .unwrap();
    assert_eq!(maria.share.amount_due_minor, 45_000);
    assert_eq!(maria.share.additional_charge_minor, 6_500);
    assert_eq!(maria.share.total_due_minor(), 51_500);

    let juan = detail
        .shares
        .iter()
        .find(|share| share.share.boarder_id == ids[1])
        .unwrap();
    assert_eq!(juan.share.amount_due_minor, 45_000);
    assert_eq!(juan.share.additional_charge_minor, 0);

    // Exclusion is per bill: the boarder itself is untouched.
    assert!(db.get_boarder(&ids[2]).unwrap().is_some());
}

#[test]
fn excluding_in_whole_peso_mode_keeps_leftover_with_the_earliest_share() {
    let (db, ids) = setup(&["Ana", "Ben", "Cara", "Dan"]);
    db.set_allow_centavos(false).unwrap();
    let bill_id = create_bill(&db, &ids, 100_000);

    db.exclude_share(&bill_id, &ids[3]).unwrap();

    let detail = db.bill_detail(&bill_id).unwrap().unwrap();
    assert_eq!(detail.bill.total_minor, 100_000);
    assert_eq!(detail.shares.len(), 3);
    assert_eq!(
        detail
            .shares
            .iter()
            .map(|share| share.share.amount_due_minor)
            .collect::<Vec<_>>(),
        vec![33_400, 33_300, 33_300]
    );
    assert_eq!(
        detail
            .shares
            .iter()
            .map(|share| share.share.amount_due_minor)
            .sum::<i64>(),
        100_000
    );
}

#[test]
fn excluding_the_last_boarder_or_an_unknown_one_is_refused() {
    let (db, ids) = setup(&["Solo", "Other"]);
    let solo_bill = create_bill(&db, &ids[..1], 50_000);
    assert!(db.exclude_share(&solo_bill, &ids[0]).is_err());

    let bill_id = create_bill(&db, &ids, 80_000);
    assert!(db.exclude_share(&bill_id, "missing-boarder").is_err());
}

#[test]
fn excluding_is_locked_once_payments_exist() {
    let (db, ids) = setup(&["Maria", "Juan", "Ana"]);
    let bill_id = create_bill(&db, &ids, 90_000);
    let detail = db.bill_detail(&bill_id).unwrap().unwrap();
    let share_id = detail
        .shares
        .iter()
        .find(|share| share.share.boarder_id == ids[0])
        .unwrap()
        .share
        .id
        .clone();

    db.record_payment(&CreatePaymentInput {
        bill_share_id: share_id,
        amount_minor: 30_000,
        paid_at: 1_760_000_000_000,
        note: None,
        payment_type: PaymentType::Cash,
        used_full_balance: false,
    })
    .unwrap();

    let error = db.exclude_share(&bill_id, &ids[1]).unwrap_err().to_string();
    assert!(error.contains("locked"), "unexpected error: {error}");
}

#[test]
fn adding_a_boarder_re_splits_across_everyone() {
    let (db, ids) = setup(&["Maria", "Juan", "Ana"]);
    let initial = create_bill(&db, &ids[..2], 90_000);

    db.add_share_to_bill(&initial, &ids[2]).unwrap();

    let detail = db.bill_detail(&initial).unwrap().unwrap();
    assert_eq!(detail.bill.total_minor, 90_000);
    assert_eq!(detail.shares.len(), 3);
    for share in &detail.shares {
        assert_eq!(share.share.amount_due_minor, 30_000);
    }

    assert!(db.add_share_to_bill(&initial, &ids[2]).is_err());
}

#[test]
fn adding_an_archived_boarder_or_after_a_payment_is_refused() {
    let (db, ids) = setup(&["Maria", "Juan", "Ana"]);
    let bill_id = create_bill(&db, &ids[..2], 90_000);

    db.set_boarder_archived(&ids[2], true).unwrap();
    let error = db
        .add_share_to_bill(&bill_id, &ids[2])
        .unwrap_err()
        .to_string();
    assert!(error.contains("Archived"), "unexpected error: {error}");

    let detail = db.bill_detail(&bill_id).unwrap().unwrap();
    let share_id = detail.shares[0].share.id.clone();
    db.record_payment(&CreatePaymentInput {
        bill_share_id: share_id,
        amount_minor: 45_000,
        paid_at: 1_760_000_000_000,
        note: None,
        payment_type: PaymentType::Cash,
        used_full_balance: false,
    })
    .unwrap();

    db.set_boarder_archived(&ids[2], false).unwrap();
    assert!(db.add_share_to_bill(&bill_id, &ids[2]).is_err());
}

#[test]
fn deleting_a_bill_removes_its_shares_and_payments() {
    let (db, ids) = setup(&["Maria", "Juan"]);
    let bill_id = create_bill(&db, &ids, 90_000);
    let detail = db.bill_detail(&bill_id).unwrap().unwrap();
    let share_id = detail.shares[0].share.id.clone();

    db.record_payment(&CreatePaymentInput {
        bill_share_id: share_id.clone(),
        amount_minor: 45_000,
        paid_at: 1_760_000_000_000,
        note: None,
        payment_type: PaymentType::Cash,
        used_full_balance: false,
    })
    .unwrap();

    db.delete_bill(&bill_id).unwrap();

    assert!(db.bill_detail(&bill_id).unwrap().is_none());
    assert!(db.share_payment_context(&share_id).unwrap().is_none());
    assert!(db.delete_bill(&bill_id).is_err());
}

#[test]
fn change_return_date_can_be_corrected_but_not_before_it_is_returned() {
    let (db, ids) = setup(&["Ana"]);
    let bill_id = create_bill(&db, &ids, 60_000);
    let detail = db.bill_detail(&bill_id).unwrap().unwrap();
    let share_id = detail.shares[0].share.id.clone();

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

    assert!(db.update_change_returned_at(&payment_id, 1_000).is_err());

    db.mark_change_returned(&payment_id, 1_000, Some("handed back"))
        .unwrap();
    db.update_change_returned_at(&payment_id, 2_000).unwrap();

    let detail = db.bill_detail(&bill_id).unwrap().unwrap();
    let payment = detail.shares[0]
        .state
        .payments
        .iter()
        .find(|item| item.payment.id == payment_id)
        .unwrap();
    assert_eq!(payment.payment.change_returned_at, Some(2_000));
    assert_eq!(payment.payment.change_return_note.as_deref(), Some("handed back"));
    assert_eq!(payment.computed_change_minor, 10_000);
    assert_eq!(detail.summary.change_returned_minor, 10_000);

    // The change was already returned, so the reversal is blocked and the
    // payment stays active with its returned date still editable.
    assert!(db.undo_payment(&payment_id).is_err());
    db.update_change_returned_at(&payment_id, 3_000).unwrap();
}
