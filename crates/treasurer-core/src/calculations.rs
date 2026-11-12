use std::collections::{HashMap, HashSet};

use crate::models::{
    BillStatus, BillSummary, BoarderPeriodSummary, HistoryBillRef, HistoryFilter, HistoryPeriod,
    HistoryTotals, PaymentRecord, PaymentWithComputedChange, ShareFilter, ShareState, ShareStatus,
    ShareWithState, SplitBoarder, SplitResult,
};
use crate::money::MINOR_UNITS_PER_UNIT;

pub struct SplitSuccess {
    pub shares: Vec<SplitResult>,
    pub leftover_minor: Option<i64>,
    pub pool_count: Option<usize>,
    /// True when whole-peso mode had to hand out a larger unit than the base share.
    pub rounded: bool,
}

pub struct SplitFailure {
    pub error: String,
    pub over_by_minor: Option<i64>,
}

pub enum SplitOutcome {
    Ok(SplitSuccess),
    Err(SplitFailure),
}

pub struct CustomShareValidation {
    pub valid: bool,
    pub difference_minor: i64,
    pub error: Option<String>,
}

pub struct PaymentPreview {
    pub paid_after_minor: i64,
    pub balance_after_minor: i64,
    pub change_from_this_payment_minor: i64,
    pub change_outstanding_after_minor: i64,
    pub message: &'static str,
}

fn split_step(allow_centavos: bool) -> i64 {
    if allow_centavos {
        1
    } else {
        MINOR_UNITS_PER_UNIT
    }
}

pub fn sum(values: &[i64]) -> i64 {
    values.iter().copied().sum()
}

fn success(shares: Vec<SplitResult>) -> SplitOutcome {
    SplitOutcome::Ok(SplitSuccess {
        shares,
        leftover_minor: None,
        pool_count: None,
        rounded: false,
    })
}

fn failure(message: impl Into<String>) -> SplitOutcome {
    SplitOutcome::Err(SplitFailure {
        error: message.into(),
        over_by_minor: None,
    })
}

pub fn equal_split(
    total_minor: i64,
    boarders: &[SplitBoarder],
    allow_centavos: bool,
) -> SplitOutcome {
    if boarders.is_empty() {
        return failure("Select at least one boarder.");
    }

    let step = split_step(allow_centavos);
    let count = boarders.len() as i64;

    if total_minor < count * step {
        return failure("The bill total is too small to split among the selected boarders.");
    }

    let base = (total_minor / (count * step)) * step;
    let remainder_units = (total_minor - base * count) / step;

    success(
        boarders
            .iter()
            .enumerate()
            .map(|(index, boarder)| SplitResult {
                boarder_id: boarder.boarder_id.clone(),
                amount_minor: base + if (index as i64) < remainder_units { step } else { 0 },
            })
            .collect(),
    )
}

pub fn split_with_overrides(
    total_minor: i64,
    boarders: &[SplitBoarder],
    overrides: &HashMap<String, i64>,
    allow_centavos: bool,
) -> SplitOutcome {
    if boarders.is_empty() {
        return failure("Select at least one boarder.");
    }
    if total_minor <= 0 {
        return failure("Enter a bill total greater than zero.");
    }

    let step = split_step(allow_centavos);
    let mut pinned: HashMap<String, i64> = HashMap::new();

    for boarder in boarders {
        let Some(&amount) = overrides.get(&boarder.boarder_id) else {
            continue;
        };
        if amount < 0 {
            return failure("Amounts cannot be negative.");
        }
        if amount % step != 0 {
            return failure("Whole pesos only. Turn on centavos to use decimals.");
        }
        pinned.insert(boarder.boarder_id.clone(), amount);
    }

    let custom_total: i64 = pinned.values().copied().sum();
    let leftover_minor = total_minor - custom_total;

    if leftover_minor < 0 {
        return SplitOutcome::Err(SplitFailure {
            error: "Custom amounts are over the bill total.".to_string(),
            over_by_minor: Some(-leftover_minor),
        });
    }

    let pool: Vec<&SplitBoarder> = boarders
        .iter()
        .filter(|boarder| !pinned.contains_key(&boarder.boarder_id))
        .collect();

    if pool.is_empty() {
        if leftover_minor != 0 {
            return failure("The assigned shares are short of the bill total.");
        }

        return SplitOutcome::Ok(SplitSuccess {
            shares: boarders
                .iter()
                .map(|boarder| SplitResult {
                    boarder_id: boarder.boarder_id.clone(),
                    amount_minor: pinned.get(&boarder.boarder_id).copied().unwrap_or(0),
                })
                .collect(),
            leftover_minor: Some(0),
            pool_count: Some(0),
            rounded: false,
        });
    }

    let base = (leftover_minor / (pool.len() as i64 * step)) * step;
    let remainder_units = (leftover_minor - base * pool.len() as i64) / step;

    let auto_shares: HashMap<&str, i64> = pool
        .iter()
        .enumerate()
        .map(|(index, boarder)| {
            (
                boarder.boarder_id.as_str(),
                base + if (index as i64) < remainder_units { step } else { 0 },
            )
        })
        .collect();

    SplitOutcome::Ok(SplitSuccess {
        shares: boarders
            .iter()
            .map(|boarder| SplitResult {
                boarder_id: boarder.boarder_id.clone(),
                amount_minor: pinned
                    .get(&boarder.boarder_id)
                    .copied()
                    .or_else(|| auto_shares.get(boarder.boarder_id.as_str()).copied())
                    .unwrap_or(0),
            })
            .collect(),
        leftover_minor: Some(leftover_minor),
        pool_count: Some(pool.len()),
        rounded: step > 1 && remainder_units > 0,
    })
}

pub fn validate_custom_shares(total_minor: i64, shares: &[SplitResult]) -> CustomShareValidation {
    if shares.is_empty() {
        return CustomShareValidation {
            valid: false,
            difference_minor: total_minor,
            error: Some("Select at least one boarder.".to_string()),
        };
    }

    if shares.iter().any(|share| share.amount_minor < 0) {
        let difference = total_minor - sum(&shares.iter().map(|s| s.amount_minor).collect::<Vec<_>>());
        return CustomShareValidation {
            valid: false,
            difference_minor: difference,
            error: Some("Amounts cannot be negative.".to_string()),
        };
    }

    let assigned: i64 = shares.iter().map(|share| share.amount_minor).sum();
    let difference_minor = total_minor - assigned;

    if difference_minor != 0 {
        return CustomShareValidation {
            valid: false,
            difference_minor,
            error: Some(
                if difference_minor > 0 {
                    "The assigned shares are short of the bill total."
                } else {
                    "The assigned shares are over the bill total."
                }
                .to_string(),
            ),
        };
    }

    CustomShareValidation {
        valid: true,
        difference_minor: 0,
        error: None,
    }
}

pub fn sort_payments_chronologically(payments: &[PaymentRecord]) -> Vec<PaymentRecord> {
    let mut ordered = payments.to_vec();
    ordered.sort_by(|a, b| {
        a.paid_at
            .cmp(&b.paid_at)
            .then(a.created_at.cmp(&b.created_at))
            .then(a.id.cmp(&b.id))
    });
    ordered
}

/// Assign the overpayment portion to the payment that crosses the amount due.
/// Voided payments are retained for history but never contribute to totals.
pub fn recompute_payment_changes(
    amount_due_minor: i64,
    payments: &[PaymentRecord],
) -> Vec<PaymentWithComputedChange> {
    let ordered = sort_payments_chronologically(payments);
    let mut running = 0i64;

    ordered
        .into_iter()
        .map(|payment| {
            if !payment.is_active() {
                return PaymentWithComputedChange {
                    payment,
                    computed_change_minor: 0,
                    needs_review: false,
                };
            }

            let before = (running - amount_due_minor).max(0);
            running += payment.amount_minor;
            let after = (running - amount_due_minor).max(0);
            let computed_change_minor = (after - before).max(0);
            let returned_amount = if payment.change_returned_at.is_some() {
                payment.change_minor
            } else {
                0
            };

            PaymentWithComputedChange {
                needs_review: payment.change_returned_at.is_some()
                    && computed_change_minor < returned_amount,
                payment,
                computed_change_minor,
            }
        })
        .collect()
}

pub fn share_status(amount_due_minor: i64, paid_minor: i64) -> ShareStatus {
    if paid_minor >= amount_due_minor {
        ShareStatus::Paid
    } else if paid_minor > 0 {
        ShareStatus::Partial
    } else {
        ShareStatus::Unpaid
    }
}

pub fn compute_share_state(amount_due_minor: i64, payments: &[PaymentRecord]) -> ShareState {
    let with_changes = recompute_payment_changes(amount_due_minor, payments);
    let active: Vec<&PaymentWithComputedChange> = with_changes
        .iter()
        .filter(|payment| payment.payment.is_active())
        .collect();

    let paid_minor: i64 = active.iter().map(|payment| payment.payment.amount_minor).sum();

    let change_outstanding_minor: i64 = active
        .iter()
        .filter(|payment| payment.payment.change_returned_at.is_none())
        .map(|payment| payment.computed_change_minor)
        .sum();
    let change_returned_minor: i64 = active
        .iter()
        .filter(|payment| payment.payment.change_returned_at.is_some())
        .map(|payment| payment.computed_change_minor)
        .sum();

    let has_returned_change = active
        .iter()
        .any(|payment| payment.payment.change_returned_at.is_some());
    let needs_review = active.iter().any(|payment| payment.needs_review);

    ShareState {
        paid_minor,
        applied_minor: paid_minor.min(amount_due_minor),
        balance_minor: (amount_due_minor - paid_minor).max(0),
        change_outstanding_minor,
        change_returned_minor,
        status: share_status(amount_due_minor, paid_minor),
        payments: with_changes,
        has_returned_change,
        needs_review,
    }
}

pub fn compute_bill_summary(shares: &[ShareWithState], archived_at: Option<i64>) -> BillSummary {
    let expected_minor: i64 = shares.iter().map(|share| share.share.amount_due_minor).sum();
    let applied_minor: i64 = shares.iter().map(|share| share.state.applied_minor).sum();
    let cash_received_minor: i64 = shares.iter().map(|share| share.state.paid_minor).sum();
    let change_outstanding_minor: i64 = shares
        .iter()
        .map(|share| share.state.change_outstanding_minor)
        .sum();
    let change_returned_minor: i64 = shares
        .iter()
        .map(|share| share.state.change_returned_minor)
        .sum();
    let paid_share_count = shares
        .iter()
        .filter(|share| share.state.balance_minor == 0)
        .count();
    let partial_share_count = shares
        .iter()
        .filter(|share| share.state.balance_minor > 0 && share.state.paid_minor > 0)
        .count();
    let unpaid_share_count = shares
        .iter()
        .filter(|share| share.state.paid_minor == 0)
        .count();
    let settled = !shares.is_empty() && shares.iter().all(|share| share.state.balance_minor == 0);

    BillSummary {
        expected_minor,
        applied_minor,
        outstanding_minor: expected_minor - applied_minor,
        cash_received_minor,
        change_outstanding_minor,
        change_returned_minor,
        progress: if expected_minor == 0 {
            0.0
        } else {
            (applied_minor as f64 / expected_minor as f64).min(1.0)
        },
        paid_share_count,
        partial_share_count,
        unpaid_share_count,
        total_share_count: shares.len(),
        status: if archived_at.is_some() {
            BillStatus::Archived
        } else if settled {
            BillStatus::Settled
        } else {
            BillStatus::Open
        },
        needs_review: shares.iter().any(|share| share.state.needs_review),
    }
}

pub fn compute_boarder_period_summaries(
    shares: &[BoarderPeriodSummaryInput<'_>],
) -> Vec<BoarderPeriodSummary> {
    let mut grouped: HashMap<String, BoarderPeriodSummary> = HashMap::new();

    for item in shares {
        let entry = grouped
            .entry(item.boarder_id.to_string())
            .or_insert_with(|| BoarderPeriodSummary {
                boarder_id: item.boarder_id.to_string(),
                boarder_name: item.boarder_name.to_string(),
                boarder_archived: item.boarder_archived,
                expected_minor: 0,
                applied_minor: 0,
                outstanding_minor: 0,
                change_outstanding_minor: 0,
                status: ShareStatus::Unpaid,
                share_count: 0,
            });

        entry.expected_minor += item.amount_due_minor;
        entry.applied_minor += item.state.applied_minor;
        entry.change_outstanding_minor += item.state.change_outstanding_minor;
        entry.outstanding_minor = (entry.expected_minor - entry.applied_minor).max(0);
        entry.status = if entry.applied_minor >= entry.expected_minor {
            ShareStatus::Paid
        } else if entry.applied_minor > 0 {
            ShareStatus::Partial
        } else {
            ShareStatus::Unpaid
        };
        entry.share_count += 1;
    }

    let mut summaries: Vec<BoarderPeriodSummary> = grouped.into_values().collect();
    summaries.sort_by(|a, b| {
        a.boarder_name
            .to_lowercase()
            .cmp(&b.boarder_name.to_lowercase())
            .then(a.boarder_name.cmp(&b.boarder_name))
    });
    summaries
}

pub struct BoarderPeriodSummaryInput<'a> {
    pub boarder_id: &'a str,
    pub boarder_name: &'a str,
    pub boarder_archived: bool,
    pub amount_due_minor: i64,
    pub state: &'a ShareState,
}

pub fn matches_share_filter(share: &ShareWithState, filter: ShareFilter) -> bool {
    match filter {
        ShareFilter::All => true,
        ShareFilter::Unpaid => share.state.balance_minor > 0,
        ShareFilter::Paid => share.state.balance_minor == 0,
        ShareFilter::Change => share.state.change_outstanding_minor > 0,
    }
}

pub fn matches_boarder_filter(summary: &BoarderPeriodSummary, filter: ShareFilter) -> bool {
    match filter {
        ShareFilter::All => true,
        ShareFilter::Unpaid => summary.outstanding_minor > 0,
        ShareFilter::Paid => summary.outstanding_minor == 0,
        ShareFilter::Change => summary.change_outstanding_minor > 0,
    }
}

pub fn preview_payment(
    amount_due_minor: i64,
    existing_payments: &[PaymentRecord],
    amount_minor: i64,
    now: i64,
) -> PaymentPreview {
    let preview = PaymentRecord {
        id: "__preview__".to_string(),
        bill_share_id: "preview".to_string(),
        amount_minor,
        paid_at: now,
        note: None,
        change_minor: 0,
        change_returned_at: None,
        change_return_note: None,
        voided_at: None,
        void_reason: None,
        created_at: now,
    };

    let mut payments = existing_payments.to_vec();
    payments.push(preview);

    let state = compute_share_state(amount_due_minor, &payments);
    let change_from_this_payment_minor = state
        .payments
        .iter()
        .find(|payment| payment.payment.id == "__preview__")
        .map(|payment| payment.computed_change_minor)
        .unwrap_or(0);

    let message = if state.balance_minor > 0 {
        "still owed"
    } else if change_from_this_payment_minor > 0 {
        "change to return"
    } else {
        "paid in full"
    };

    PaymentPreview {
        paid_after_minor: state.paid_minor,
        balance_after_minor: state.balance_minor,
        change_from_this_payment_minor,
        change_outstanding_after_minor: state.change_outstanding_minor,
        message,
    }
}

pub fn bill_status_label(status: BillStatus) -> &'static str {
    status.label()
}

pub fn share_status_label(status: ShareStatus) -> &'static str {
    status.label()
}

pub fn matches_history_filter(bill: &HistoryBillRef, filter: &HistoryFilter) -> bool {
    if let Some(year) = filter.year {
        if bill.period_year != year {
            return false;
        }
    }
    if let Some(month) = filter.month {
        if bill.period_month != month {
            return false;
        }
    }
    if let Some(ref bill_type_id) = filter.bill_type_id {
        if &bill.bill_type_id != bill_type_id {
            return false;
        }
    }
    true
}

/// Newest period first, then the most recently created bill inside the month.
pub fn sort_history_bills(items: &[HistoryBillRef]) -> Vec<HistoryBillRef> {
    let mut sorted = items.to_vec();
    sorted.sort_by(|a, b| {
        b.period_year
            .cmp(&a.period_year)
            .then(b.period_month.cmp(&a.period_month))
            .then(b.created_at.cmp(&a.created_at))
            .then(a.id.cmp(&b.id))
    });
    sorted
}

pub fn list_history_periods(bills: &[HistoryBillRef]) -> Vec<HistoryPeriod> {
    let mut periods: HashMap<(i32, i32), HistoryPeriod> = HashMap::new();

    for bill in bills {
        let entry = periods
            .entry((bill.period_year, bill.period_month))
            .or_insert(HistoryPeriod {
                year: bill.period_year,
                month: bill.period_month,
                bill_count: 0,
            });
        entry.bill_count += 1;
    }

    let mut list: Vec<HistoryPeriod> = periods.into_values().collect();
    list.sort_by(|a, b| b.year.cmp(&a.year).then(b.month.cmp(&a.month)));
    list
}

pub fn compute_history_totals(summaries: &[HistoryTotals]) -> HistoryTotals {
    HistoryTotals {
        expected_minor: summaries.iter().map(|item| item.expected_minor).sum(),
        applied_minor: summaries.iter().map(|item| item.applied_minor).sum(),
        outstanding_minor: summaries.iter().map(|item| item.outstanding_minor).sum(),
        change_outstanding_minor: summaries
            .iter()
            .map(|item| item.change_outstanding_minor)
            .sum(),
    }
}

pub fn unique_boarder_ids(shares: &[SplitResult]) -> bool {
    let mut seen = HashSet::with_capacity(shares.len());
    shares
        .iter()
        .all(|share| seen.insert(share.boarder_id.as_str()))
}
