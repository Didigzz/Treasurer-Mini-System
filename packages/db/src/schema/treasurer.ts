import { index, integer, sqliteTable, text, uniqueIndex } from "drizzle-orm/sqlite-core";

export const settings = sqliteTable("settings", {
  key: text("key").primaryKey(),
  value: text("value").notNull(),
  updatedAt: integer("updated_at").notNull(),
});

export const billTypes = sqliteTable(
  "bill_types",
  {
    id: text("id").primaryKey(),
    name: text("name").notNull(),
    isArchived: integer("is_archived", { mode: "boolean" }).notNull().default(false),
    sortOrder: integer("sort_order").notNull().default(0),
    createdAt: integer("created_at").notNull(),
    updatedAt: integer("updated_at").notNull(),
  },
  (table) => [index("bill_types_archived_idx").on(table.isArchived, table.sortOrder)],
);

export const boarders = sqliteTable(
  "boarders",
  {
    id: text("id").primaryKey(),
    name: text("name").notNull(),
    status: text("status", { enum: ["active", "archived"] }).notNull().default("active"),
    archivedAt: integer("archived_at"),
    createdAt: integer("created_at").notNull(),
    updatedAt: integer("updated_at").notNull(),
  },
  (table) => [index("boarders_status_idx").on(table.status)],
);

export const bills = sqliteTable(
  "bills",
  {
    id: text("id").primaryKey(),
    billTypeId: text("bill_type_id")
      .notNull()
      .references(() => billTypes.id, { onDelete: "restrict" }),
    billTypeNameSnapshot: text("bill_type_name_snapshot").notNull(),
    periodYear: integer("period_year").notNull(),
    periodMonth: integer("period_month").notNull(),
    totalMinor: integer("total_minor").notNull(),
    dueDate: text("due_date"),
    note: text("note"),
    archivedAt: integer("archived_at"),
    createdAt: integer("created_at").notNull(),
    updatedAt: integer("updated_at").notNull(),
  },
  (table) => [
    index("bills_period_idx").on(table.periodYear, table.periodMonth),
    index("bills_type_period_idx").on(table.billTypeId, table.periodYear, table.periodMonth),
  ],
);

export const billShares = sqliteTable(
  "bill_shares",
  {
    id: text("id").primaryKey(),
    billId: text("bill_id")
      .notNull()
      .references(() => bills.id, { onDelete: "cascade" }),
    boarderId: text("boarder_id")
      .notNull()
      .references(() => boarders.id, { onDelete: "restrict" }),
    boarderNameSnapshot: text("boarder_name_snapshot").notNull(),
    amountDueMinor: integer("amount_due_minor").notNull(),
    createdAt: integer("created_at").notNull(),
  },
  (table) => [
    uniqueIndex("bill_shares_bill_boarder_idx").on(table.billId, table.boarderId),
    index("bill_shares_boarder_idx").on(table.boarderId),
  ],
);

export const payments = sqliteTable(
  "payments",
  {
    id: text("id").primaryKey(),
    billShareId: text("bill_share_id")
      .notNull()
      .references(() => billShares.id, { onDelete: "cascade" }),
    amountMinor: integer("amount_minor").notNull(),
    paidAt: integer("paid_at").notNull(),
    note: text("note"),
    changeMinor: integer("change_minor").notNull().default(0),
    changeReturnedAt: integer("change_returned_at"),
    changeReturnNote: text("change_return_note"),
    voidedAt: integer("voided_at"),
    voidReason: text("void_reason"),
    createdAt: integer("created_at").notNull(),
  },
  (table) => [
    index("payments_share_idx").on(table.billShareId),
    index("payments_paid_at_idx").on(table.paidAt),
  ],
);

export type BillTypeRow = typeof billTypes.$inferSelect;
export type BoarderRow = typeof boarders.$inferSelect;
export type BillRow = typeof bills.$inferSelect;
export type BillShareRow = typeof billShares.$inferSelect;
export type PaymentRow = typeof payments.$inferSelect;
export type SettingRow = typeof settings.$inferSelect;
