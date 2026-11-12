#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillStatus {
    Open,
    Settled,
    Archived,
}

impl BillStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            BillStatus::Open => "open",
            BillStatus::Settled => "settled",
            BillStatus::Archived => "archived",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            BillStatus::Open => "Open",
            BillStatus::Settled => "Settled",
            BillStatus::Archived => "Archived",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareStatus {
    Unpaid,
    Partial,
    Paid,
}

impl ShareStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ShareStatus::Unpaid => "unpaid",
            ShareStatus::Partial => "partial",
            ShareStatus::Paid => "paid",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ShareStatus::Unpaid => "Unpaid",
            ShareStatus::Partial => "Partial",
            ShareStatus::Paid => "Paid",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoarderStatus {
    Active,
    Archived,
}

impl BoarderStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            BoarderStatus::Active => "active",
            BoarderStatus::Archived => "archived",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareFilter {
    All,
    Unpaid,
    Paid,
    Change,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillType {
    pub id: String,
    pub name: String,
    pub is_archived: bool,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillTypeWithCount {
    pub bill_type: BillType,
    pub bill_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Boarder {
    pub id: String,
    pub name: String,
    pub status: BoarderStatus,
    pub archived_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoarderWithBalance {
    pub boarder: Boarder,
    pub outstanding_minor: i64,
    pub change_outstanding_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bill {
    pub id: String,
    pub bill_type_id: String,
    pub bill_type_name_snapshot: String,
    pub period_year: i32,
    pub period_month: i32,
    pub total_minor: i64,
    pub due_date: Option<String>,
    pub note: Option<String>,
    pub archived_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillShare {
    pub id: String,
    pub bill_id: String,
    pub boarder_id: String,
    pub boarder_name_snapshot: String,
    pub amount_due_minor: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentRecord {
    pub id: String,
    pub bill_share_id: String,
    pub amount_minor: i64,
    pub paid_at: i64,
    pub note: Option<String>,
    pub change_minor: i64,
    pub change_returned_at: Option<i64>,
    pub change_return_note: Option<String>,
    pub voided_at: Option<i64>,
    pub void_reason: Option<String>,
    pub created_at: i64,
}

impl PaymentRecord {
    pub fn is_active(&self) -> bool {
        self.voided_at.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentWithComputedChange {
    pub payment: PaymentRecord,
    pub computed_change_minor: i64,
    pub needs_review: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareState {
    pub paid_minor: i64,
    pub applied_minor: i64,
    pub balance_minor: i64,
    pub change_outstanding_minor: i64,
    pub change_returned_minor: i64,
    pub status: ShareStatus,
    pub payments: Vec<PaymentWithComputedChange>,
    pub has_returned_change: bool,
    pub needs_review: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareWithState {
    pub share: BillShare,
    pub state: ShareState,
    pub current_boarder_name: Option<String>,
    pub boarder_archived: bool,
}

impl ShareWithState {
    pub fn display_name(&self) -> &str {
        self.current_boarder_name
            .as_deref()
            .unwrap_or(&self.share.boarder_name_snapshot)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BillSummary {
    pub expected_minor: i64,
    pub applied_minor: i64,
    pub outstanding_minor: i64,
    pub cash_received_minor: i64,
    pub change_outstanding_minor: i64,
    pub change_returned_minor: i64,
    pub progress: f64,
    pub paid_share_count: usize,
    pub partial_share_count: usize,
    pub unpaid_share_count: usize,
    pub total_share_count: usize,
    pub status: BillStatus,
    pub needs_review: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoarderPeriodSummary {
    pub boarder_id: String,
    pub boarder_name: String,
    pub boarder_archived: bool,
    pub expected_minor: i64,
    pub applied_minor: i64,
    pub outstanding_minor: i64,
    pub change_outstanding_minor: i64,
    pub status: ShareStatus,
    pub share_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BillWithDetails {
    pub bill: Bill,
    pub shares: Vec<ShareWithState>,
    pub summary: BillSummary,
    pub current_type_name: String,
    pub is_locked: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DashboardTotals {
    pub expected_minor: i64,
    pub applied_minor: i64,
    pub outstanding_minor: i64,
    pub change_outstanding_minor: i64,
    pub paid_share_count: usize,
    pub partial_share_count: usize,
    pub unpaid_share_count: usize,
    pub total_share_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DashboardData {
    pub bills: Vec<BillWithDetails>,
    pub boarder_summaries: Vec<BoarderPeriodSummary>,
    pub archived_bill_count: usize,
    pub totals: DashboardTotals,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoarderLedgerItem {
    pub bill: Bill,
    pub share: ShareWithState,
    pub state: ShareState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedgerSummary {
    pub expected_minor: i64,
    pub applied_minor: i64,
    pub outstanding_minor: i64,
    pub change_outstanding_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoarderLedgerPayment {
    pub payment: PaymentWithComputedChange,
    pub bill_id: String,
    pub bill_type_name: String,
    pub period_year: i32,
    pub period_month: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoarderLedger {
    pub boarder: Boarder,
    pub items: Vec<BoarderLedgerItem>,
    pub summary: LedgerSummary,
    pub payments: Vec<BoarderLedgerPayment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryPeriod {
    pub year: i32,
    pub month: i32,
    pub bill_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryFilter {
    pub year: Option<i32>,
    pub month: Option<i32>,
    pub bill_type_id: Option<String>,
}

impl HistoryFilter {
    pub fn all() -> Self {
        Self {
            year: None,
            month: None,
            bill_type_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryBillRef {
    pub id: String,
    pub bill_type_id: String,
    pub period_year: i32,
    pub period_month: i32,
    pub created_at: i64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HistoryTotals {
    pub expected_minor: i64,
    pub applied_minor: i64,
    pub outstanding_minor: i64,
    pub change_outstanding_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryBillType {
    pub id: String,
    pub name: String,
    pub bill_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BillHistoryData {
    pub bills: Vec<BillWithDetails>,
    pub periods: Vec<HistoryPeriod>,
    pub bill_types: Vec<HistoryBillType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitBoarder {
    pub boarder_id: String,
    pub name: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitResult {
    pub boarder_id: String,
    pub amount_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppSettings {
    pub currency_code: String,
    pub allow_centavos: bool,
    pub last_backup_at: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            currency_code: "PHP".to_string(),
            allow_centavos: true,
            last_backup_at: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateBillInput {
    pub bill_type_id: String,
    pub period_year: i32,
    pub period_month: i32,
    pub total_minor: i64,
    pub due_date: Option<String>,
    pub note: Option<String>,
    pub shares: Vec<SplitResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatePaymentInput {
    pub bill_share_id: String,
    pub amount_minor: i64,
    pub paid_at: i64,
    pub note: Option<String>,
}
