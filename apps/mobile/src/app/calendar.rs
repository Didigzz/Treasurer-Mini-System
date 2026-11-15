use std::cell::RefCell;
use std::rc::Rc;

use chrono::{Datelike, Local, NaiveDate};
use slint::{ModelRc, VecModel};

use treasurer_core::db::repository::now_ms;
use treasurer_core::money;

use super::App;
use crate::AppWindow;
use crate::CalendarDay;

pub(super) const TARGET_BILL_DUE: i32 = 1;
pub(super) const TARGET_PAYMENT_DATE: i32 = 2;
pub(super) const TARGET_BILL_PERIOD: i32 = 3;
pub(super) const TARGET_DASHBOARD_PERIOD: i32 = 4;
pub(super) const TARGET_CHANGE_RETURNED: i32 = 5;

pub(super) fn parse_iso_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").ok()
}

pub(super) fn parse_hhmm(value: &str) -> Option<(i32, i32)> {
    let (hour, minute) = value.trim().split_once(':')?;
    let hour: i32 = hour.parse().ok()?;
    let minute: i32 = minute.parse().ok()?;
    if !(0..24).contains(&hour) || !(0..60).contains(&minute) {
        return None;
    }
    Some((hour, minute))
}

fn last_day(year: i32, month: i32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(next_year, next_month as u32, 1)
        .and_then(|date| date.pred_opt())
        .map(|date| date.day())
        .unwrap_or(28)
}

impl App {
    pub(super) fn open_calendar(&mut self, target: i32) {
        let today = Local::now().date_naive();
        let ui = self.ui();
        self.calendar_target = target;

        let mut month_only = false;
        let mut allow_clear = false;
        let mut include_time = false;
        let mut title = "Date".to_string();

        let date = match target {
            TARGET_BILL_DUE => {
                title = "Due date".to_string();
                allow_clear = true;
                parse_iso_date(&ui.get_bill_due()).unwrap_or(today)
            }
            TARGET_PAYMENT_DATE => {
                title = "Payment date".to_string();
                include_time = true;
                parse_iso_date(&ui.get_payment_date()).unwrap_or(today)
            }
            TARGET_BILL_PERIOD => {
                title = "Billing month".to_string();
                month_only = true;
                let year = ui.get_bill_year().trim().parse().unwrap_or(today.year());
                let month = ui
                    .get_bill_month()
                    .trim()
                    .parse()
                    .unwrap_or(today.month() as i32);
                NaiveDate::from_ymd_opt(
                    year.clamp(money::MIN_YEAR, money::MAX_YEAR),
                    month.clamp(1, 12) as u32,
                    1,
                )
                .unwrap_or(today)
            }
            TARGET_DASHBOARD_PERIOD => {
                title = "Month".to_string();
                month_only = true;
                NaiveDate::from_ymd_opt(
                    self.year.clamp(money::MIN_YEAR, money::MAX_YEAR),
                    self.month.clamp(1, 12) as u32,
                    1,
                )
                .unwrap_or(today)
            }
            TARGET_CHANGE_RETURNED => {
                title = "Returned on".to_string();
                if self.change_correction_payment_id.is_some() {
                    self.detail_returned_at
                        .map(|epoch| money::split_local_datetime(epoch).0)
                        .unwrap_or(today)
                } else {
                    self.return_change_date
                }
            }
            _ => today,
        };

        let (hour, minute, pm) = if target == TARGET_PAYMENT_DATE {
            match parse_hhmm(&ui.get_payment_time()) {
                Some((hour24, minute)) => (
                    if hour24 % 12 == 0 { 12 } else { hour24 % 12 },
                    minute,
                    hour24 >= 12,
                ),
                None => {
                    let (_, hour, minute, pm) = money::split_local_datetime(now_ms());
                    (hour as i32, minute as i32, pm)
                }
            }
        } else {
            (self.calendar_hour, self.calendar_minute, self.calendar_pm)
        };

        self.calendar_date = date;
        self.calendar_year = date.year();
        self.calendar_month = date.month() as i32;
        self.calendar_month_only = month_only;
        self.calendar_hour = hour.clamp(1, 12);
        self.calendar_minute = minute.clamp(0, 59);
        self.calendar_pm = pm;

        ui.set_calendar_title(title.into());
        ui.set_calendar_month_only(month_only);
        ui.set_calendar_allow_clear(allow_clear);
        ui.set_calendar_include_time(include_time);
        ui.set_calendar_hour(self.calendar_hour);
        ui.set_calendar_minute(self.calendar_minute);
        ui.set_calendar_pm(self.calendar_pm);
        ui.set_calendar_time_label(self.calendar_time_label().into());
        self.refresh_calendar_days();
    }

    pub(super) fn refresh_calendar_days(&mut self) {
        let today = Local::now().date_naive();
        let days: Vec<CalendarDay> = money::calendar_weeks(self.calendar_year, self.calendar_month)
            .into_iter()
            .map(|(date, in_month)| CalendarDay {
                day: date.day() as i32,
                in_month,
                selected: date == self.calendar_date,
                today: date == today,
                enabled: (money::MIN_YEAR..=money::MAX_YEAR).contains(&date.year()),
                iso: money::to_iso_date(date).into(),
            })
            .collect();

        let header = if self.calendar_month_only {
            self.calendar_year.to_string()
        } else {
            money::format_period(self.calendar_year, self.calendar_month)
        };

        let ui = self.ui();
        ui.set_calendar_year(self.calendar_year);
        ui.set_calendar_month(self.calendar_month);
        ui.set_calendar_header_label(header.into());
        ui.set_calendar_days(ModelRc::new(VecModel::from(days)));
    }

    fn calendar_time_label(&self) -> String {
        format!(
            "{}:{:02} {}",
            self.calendar_hour,
            self.calendar_minute,
            if self.calendar_pm { "PM" } else { "AM" }
        )
    }

    fn shift_calendar(&mut self, delta: i32) {
        if self.calendar_month_only {
            self.calendar_year =
                (self.calendar_year + delta).clamp(money::MIN_YEAR, money::MAX_YEAR);
            self.calendar_month = self.calendar_month.clamp(1, 12);
            self.calendar_date =
                NaiveDate::from_ymd_opt(self.calendar_year, self.calendar_month as u32, 1)
                    .unwrap_or(self.calendar_date);
        } else {
            let (year, month) = money::add_months(self.calendar_year, self.calendar_month, delta);
            self.calendar_year = year;
            self.calendar_month = month;
            let day = self.calendar_date.day().min(last_day(year, month));
            self.calendar_date =
                NaiveDate::from_ymd_opt(year, month as u32, day).unwrap_or(self.calendar_date);
        }
        self.refresh_calendar_days();
    }

    pub(super) fn calendar_prev_month(&mut self) {
        self.shift_calendar(-1);
    }

    pub(super) fn calendar_next_month(&mut self) {
        self.shift_calendar(1);
    }

    pub(super) fn calendar_pick_year(&mut self, year: i32) {
        self.calendar_year = year.clamp(money::MIN_YEAR, money::MAX_YEAR);
        self.calendar_date = NaiveDate::from_ymd_opt(
            self.calendar_year,
            self.calendar_month as u32,
            self.calendar_date
                .day()
                .min(last_day(self.calendar_year, self.calendar_month)),
        )
        .unwrap_or(self.calendar_date);
        self.refresh_calendar_days();
    }

    pub(super) fn calendar_pick_month(&mut self, month: i32) {
        self.calendar_month = month.clamp(1, 12);
        self.calendar_date = NaiveDate::from_ymd_opt(
            self.calendar_year,
            self.calendar_month as u32,
            self.calendar_date
                .day()
                .min(last_day(self.calendar_year, self.calendar_month)),
        )
        .unwrap_or(self.calendar_date);
        self.refresh_calendar_days();
        if self.calendar_month_only {
            self.commit_calendar_period();
        }
    }

    pub(super) fn calendar_pick(&mut self, iso: &str) {
        let Some(date) = parse_iso_date(iso) else {
            return;
        };
        self.calendar_date = date;
        self.calendar_year = date.year();
        self.calendar_month = date.month() as i32;
        self.refresh_calendar_days();
        if self.calendar_target != TARGET_PAYMENT_DATE {
            self.commit_calendar_date();
        }
    }

    pub(super) fn calendar_today(&mut self) {
        let today = Local::now().date_naive();
        self.calendar_date = today;
        self.calendar_year = today.year();
        self.calendar_month = today.month() as i32;
        self.refresh_calendar_days();

        if self.calendar_month_only {
            self.commit_calendar_period();
        } else if self.calendar_target != TARGET_PAYMENT_DATE {
            self.commit_calendar_date();
        }
    }

    pub(super) fn calendar_clear(&mut self) {
        if self.calendar_target != TARGET_BILL_DUE {
            return;
        }
        let ui = self.ui();
        ui.set_bill_due("".into());
        ui.set_bill_due_label("".into());
    }

    pub(super) fn calendar_confirm(&mut self) {
        if self.calendar_month_only {
            self.commit_calendar_period();
        } else {
            self.commit_calendar_date();
        }
    }

    pub(super) fn calendar_time_changed(&mut self, hour: i32, minute: i32, pm: bool) {
        self.calendar_hour = hour.clamp(1, 12);
        self.calendar_minute = minute.clamp(0, 59);
        self.calendar_pm = pm;

        let ui = self.ui();
        ui.set_calendar_hour(self.calendar_hour);
        ui.set_calendar_minute(self.calendar_minute);
        ui.set_calendar_pm(self.calendar_pm);
        ui.set_calendar_time_label(self.calendar_time_label().into());
    }

    fn commit_calendar_period(&mut self) {
        match self.calendar_target {
            TARGET_BILL_PERIOD => {
                let ui = self.ui();
                ui.set_bill_year(self.calendar_year.to_string().into());
                ui.set_bill_month(self.calendar_month.to_string().into());
                ui.set_bill_period_label(
                    money::format_period(self.calendar_year, self.calendar_month).into(),
                );
            }
            TARGET_DASHBOARD_PERIOD => {
                self.year = self.calendar_year;
                self.month = self.calendar_month;
                self.refresh_dashboard();
            }
            _ => {}
        }
    }

    fn commit_calendar_date(&mut self) {
        let iso = money::to_iso_date(self.calendar_date);
        let ui = self.ui();

        match self.calendar_target {
            TARGET_BILL_DUE => {
                ui.set_bill_due(iso.clone().into());
                ui.set_bill_due_label(money::format_date_only(&iso).into());
            }
            TARGET_PAYMENT_DATE => {
                let hour = if self.calendar_pm {
                    if self.calendar_hour == 12 {
                        12
                    } else {
                        self.calendar_hour + 12
                    }
                } else if self.calendar_hour == 12 {
                    0
                } else {
                    self.calendar_hour
                };
                ui.set_payment_date(iso.clone().into());
                ui.set_payment_time(
                    format!("{hour:02}:{:02}", self.calendar_minute).into(),
                );
                ui.set_payment_date_label(
                    format!(
                        "{} · {}",
                        money::format_date_only(&iso),
                        self.calendar_time_label()
                    )
                    .into(),
                );
            }
            TARGET_CHANGE_RETURNED => {
                if let Some(payment_id) = self.change_correction_payment_id.clone() {
                    let at = money::local_datetime_ms(self.calendar_date, 12, 0);
                    match self.db.update_change_returned_at(&payment_id, at) {
                        Ok(()) => {
                            self.change_correction_payment_id = None;
                            self.refresh_bill_detail();
                            self.refresh_dashboard();
                            if let (Some(detail_id), Some(share_id)) = (
                                self.detail_payment_id.clone(),
                                self.detail_payment_share_id.clone(),
                            ) {
                                self.show_payment_detail(&detail_id, &share_id);
                            }
                        }
                        Err(error) => self.message(format!("{error:#}")),
                    }
                } else {
                    self.return_change_date = self.calendar_date;
                    ui.set_return_change_date_label(
                        money::format_date_only(&iso).into(),
                    );
                }
            }
            _ => {}
        }
    }
}

pub(super) fn wire(app: &Rc<RefCell<App>>, ui: &AppWindow) {
    {
        let app = app.clone();
        ui.on_open_calendar(move |target| app.borrow_mut().open_calendar(target));
    }
    {
        let app = app.clone();
        ui.on_calendar_prev_month(move || app.borrow_mut().calendar_prev_month());
    }
    {
        let app = app.clone();
        ui.on_calendar_next_month(move || app.borrow_mut().calendar_next_month());
    }
    {
        let app = app.clone();
        ui.on_calendar_pick(move |iso| app.borrow_mut().calendar_pick(&iso));
    }
    {
        let app = app.clone();
        ui.on_calendar_today(move || app.borrow_mut().calendar_today());
    }
    {
        let app = app.clone();
        ui.on_calendar_clear(move || app.borrow_mut().calendar_clear());
    }
    {
        let app = app.clone();
        ui.on_calendar_confirm(move || app.borrow_mut().calendar_confirm());
    }
    {
        let app = app.clone();
        ui.on_calendar_time_changed(move |hour, minute, pm| {
            app.borrow_mut().calendar_time_changed(hour, minute, pm)
        });
    }
    {
        let app = app.clone();
        ui.on_calendar_pick_month(move |month| {
            app.borrow_mut().calendar_pick_month(month)
        });
    }
    {
        let app = app.clone();
        ui.on_calendar_pick_year(move |year| app.borrow_mut().calendar_pick_year(year));
    }
}
