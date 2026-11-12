use std::collections::HashMap;

use anyhow::Result;

use crate::calculations::{
    compute_boarder_period_summaries,
    list_history_periods, sort_history_bills, BoarderPeriodSummaryInput,
};
use crate::models::*;

use super::Treasurer;

impl Treasurer {
    pub fn dashboard(
        &self,
        period_year: i32,
        period_month: i32,
        include_archived: bool,
    ) -> Result<DashboardData> {
        let all_bills = self.all_bills()?;
        let shares = self.shares_with_states()?;
        let types = self.list_bill_types()?;
        let sort_order: HashMap<&str, i64> = types
            .iter()
            .map(|row| (row.bill_type.id.as_str(), row.bill_type.sort_order))
            .collect();

        let mut bills: Vec<BillWithDetails> = all_bills
            .iter()
            .filter(|bill| {
                bill.period_year == period_year
                    && bill.period_month == period_month
                    && (include_archived || bill.archived_at.is_none())
            })
            .map(|bill| {
                let bill_shares: Vec<ShareWithState> = shares
                    .iter()
                    .filter(|share| share.share.bill_id == bill.id)
                    .cloned()
                    .collect();
                self.bill_with_details(bill, &bill_shares)
            })
            .collect();

        bills.sort_by(|a, b| {
            let rank = |status: BillStatus| match status {
                BillStatus::Open => 0,
                BillStatus::Settled => 1,
                BillStatus::Archived => 2,
            };
            rank(a.summary.status)
                .cmp(&rank(b.summary.status))
                .then_with(|| {
                    sort_order
                        .get(a.bill.bill_type_id.as_str())
                        .copied()
                        .unwrap_or(0)
                        .cmp(&sort_order.get(b.bill.bill_type_id.as_str()).copied().unwrap_or(0))
                })
                .then(a.bill.created_at.cmp(&b.bill.created_at))
        });

        let active_bills: Vec<&BillWithDetails> = bills
            .iter()
            .filter(|item| item.bill.archived_at.is_none())
            .collect();

        let summaries_input: Vec<BoarderPeriodSummaryInput<'_>> = active_bills
            .iter()
            .flat_map(|item| item.shares.iter())
            .map(|share| BoarderPeriodSummaryInput {
                boarder_id: &share.share.boarder_id,
                boarder_name: share.display_name(),
                boarder_archived: share.boarder_archived,
                amount_due_minor: share.share.amount_due_minor,
                state: &share.state,
            })
            .collect();
        let boarder_summaries = compute_boarder_period_summaries(&summaries_input);

        let totals = active_bills.iter().fold(DashboardTotals::default(), |mut acc, item| {
            acc.expected_minor += item.summary.expected_minor;
            acc.applied_minor += item.summary.applied_minor;
            acc.outstanding_minor += item.summary.outstanding_minor;
            acc.change_outstanding_minor += item.summary.change_outstanding_minor;
            acc.paid_share_count += item.summary.paid_share_count;
            acc.partial_share_count += item.summary.partial_share_count;
            acc.unpaid_share_count += item.summary.unpaid_share_count;
            acc.total_share_count += item.summary.total_share_count;
            acc
        });

        Ok(DashboardData {
            archived_bill_count: all_bills
                .iter()
                .filter(|bill| {
                    bill.period_year == period_year
                        && bill.period_month == period_month
                        && bill.archived_at.is_some()
                })
                .count(),
            bills,
            boarder_summaries,
            totals,
        })
    }

    pub fn bill_detail(&self, id: &str) -> Result<Option<BillWithDetails>> {
        let bills = self.all_bills()?;
        let Some(bill) = bills.iter().find(|bill| bill.id == id) else {
            return Ok(None);
        };
        let shares: Vec<ShareWithState> = self
            .shares_with_states()?
            .into_iter()
            .filter(|share| share.share.bill_id == id)
            .collect();
        Ok(Some(self.bill_with_details(bill, &shares)))
    }

    pub fn bill_history(&self) -> Result<BillHistoryData> {
        let all_bills = self.all_bills()?;
        let shares = self.shares_with_states()?;
        let types = self.list_bill_types()?;
        let type_names: HashMap<&str, &str> = types
            .iter()
            .map(|row| (row.bill_type.id.as_str(), row.bill_type.name.as_str()))
            .collect();

        let mut details: Vec<BillWithDetails> = all_bills
            .iter()
            .map(|bill| {
                let bill_shares: Vec<ShareWithState> = shares
                    .iter()
                    .filter(|share| share.share.bill_id == bill.id)
                    .cloned()
                    .collect();
                self.bill_with_details(bill, &bill_shares)
            })
            .collect();

        let refs: Vec<HistoryBillRef> = details
            .iter()
            .map(|item| HistoryBillRef {
                id: item.bill.id.clone(),
                bill_type_id: item.bill.bill_type_id.clone(),
                period_year: item.bill.period_year,
                period_month: item.bill.period_month,
                created_at: item.bill.created_at,
            })
            .collect();
        let sorted_refs = sort_history_bills(&refs);
        let positions: HashMap<&str, usize> = sorted_refs
            .iter()
            .enumerate()
            .map(|(index, reference)| (reference.id.as_str(), index))
            .collect();
        details.sort_by_key(|item| positions.get(item.bill.id.as_str()).copied().unwrap_or(0));

        let periods = list_history_periods(&refs);

        let mut counts: HashMap<&str, usize> = HashMap::new();
        for bill in &all_bills {
            *counts.entry(bill.bill_type_id.as_str()).or_insert(0) += 1;
        }
        let mut bill_types: Vec<HistoryBillType> = counts
            .into_iter()
            .map(|(id, bill_count)| HistoryBillType {
                id: id.to_string(),
                name: type_names
                    .get(id)
                    .copied()
                    .unwrap_or(id)
                    .to_string(),
                bill_count,
            })
            .collect();
        bill_types.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then(a.name.cmp(&b.name))
        });

        Ok(BillHistoryData {
            bills: details,
            periods,
            bill_types,
        })
    }

    pub fn boarder_ledger(&self, id: &str) -> Result<Option<BoarderLedger>> {
        let Some(boarder) = self.get_boarder(id)? else {
            return Ok(None);
        };

        let bills = self.all_bills()?;
        let bills_by_id: HashMap<&str, &Bill> =
            bills.iter().map(|bill| (bill.id.as_str(), bill)).collect();

        let shares: Vec<ShareWithState> = self
            .shares_with_states()?
            .into_iter()
            .filter(|share| share.share.boarder_id == id)
            .collect();

        let mut items: Vec<BoarderLedgerItem> = shares
            .iter()
            .filter_map(|share| {
                let bill = bills_by_id.get(share.share.bill_id.as_str())?;
                Some(BoarderLedgerItem {
                    bill: (*bill).clone(),
                    share: share.clone(),
                    state: share.state.clone(),
                })
            })
            .collect();
        items.sort_by(|a, b| {
            b.bill
                .period_year
                .cmp(&a.bill.period_year)
                .then(b.bill.period_month.cmp(&a.bill.period_month))
                .then(b.bill.created_at.cmp(&a.bill.created_at))
        });

        let summary = LedgerSummary {
            expected_minor: items.iter().map(|item| item.share.share.amount_due_minor).sum(),
            applied_minor: items.iter().map(|item| item.state.applied_minor).sum(),
            outstanding_minor: items.iter().map(|item| item.state.balance_minor).sum(),
            change_outstanding_minor: items
                .iter()
                .map(|item| item.state.change_outstanding_minor)
                .sum(),
        };

        let mut payments: Vec<BoarderLedgerPayment> = items
            .iter()
            .flat_map(|item| {
                item.state.payments.iter().map(|payment| BoarderLedgerPayment {
                    payment: payment.clone(),
                    bill_id: item.bill.id.clone(),
                    bill_type_name: item.bill.bill_type_name_snapshot.clone(),
                    period_year: item.bill.period_year,
                    period_month: item.bill.period_month,
                })
            })
            .collect();
        payments.sort_by(|a, b| b.payment.payment.paid_at.cmp(&a.payment.payment.paid_at));

        Ok(Some(BoarderLedger {
            boarder,
            items,
            summary,
            payments,
        }))
    }
}
