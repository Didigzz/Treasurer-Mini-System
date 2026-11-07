import { z } from "zod";

import { publicProcedure, router } from "../index";
import { MAX_AMOUNT_MINOR } from "../treasurer/money";
import {
  createBill,
  findDuplicateBills,
  getBillDetail,
  getDashboard,
  setBillArchived,
  updateBill,
  updateBillMeta,
} from "../treasurer/repository";

const idInput = z.object({ id: z.string().min(1) });
const isoDate = z.string().regex(/^\d{4}-\d{2}-\d{2}$/);

const createBillInput = z.object({
  billTypeId: z.string().min(1),
  periodYear: z.number().int().min(2000).max(2100),
  periodMonth: z.number().int().min(1).max(12),
  totalMinor: z.number().int().min(1).max(MAX_AMOUNT_MINOR),
  dueDate: isoDate.nullable(),
  note: z.string().max(500).nullable(),
  shares: z
    .array(
      z.object({
        boarderId: z.string().min(1),
        amountMinor: z.number().int().min(1).max(MAX_AMOUNT_MINOR),
      }),
    )
    .min(1),
});

export const billsRouter = router({
  dashboard: publicProcedure
    .input(
      z.object({
        year: z.number().int().min(2000).max(2100),
        month: z.number().int().min(1).max(12),
        includeArchived: z.boolean().default(false),
      }),
    )
    .query(({ ctx, input }) =>
      getDashboard(ctx.db, input.year, input.month, input.includeArchived),
    ),
  get: publicProcedure.input(idInput).query(({ ctx, input }) =>
    getBillDetail(ctx.db, input.id),
  ),
  create: publicProcedure
    .input(createBillInput)
    .mutation(({ ctx, input }) => createBill(ctx.db, input)),
  update: publicProcedure
    .input(idInput.extend({ input: createBillInput }))
    .mutation(async ({ ctx, input }) => {
      await updateBill(ctx.db, input.id, input.input);
    }),
  setArchived: publicProcedure
    .input(idInput.extend({ isArchived: z.boolean() }))
    .mutation(async ({ ctx, input }) => {
      await setBillArchived(ctx.db, input.id, input.isArchived);
    }),
  updateMeta: publicProcedure
    .input(
      idInput.extend({
        dueDate: isoDate.nullable(),
        note: z.string().max(500).nullable(),
      }),
    )
    .mutation(async ({ ctx, input }) => {
      await updateBillMeta(ctx.db, input.id, input.dueDate, input.note);
    }),
  findDuplicates: publicProcedure
    .input(
      z.object({
        billTypeId: z.string().min(1),
        periodYear: z.number().int().min(2000).max(2100),
        periodMonth: z.number().int().min(1).max(12),
        excludeBillId: z.string().min(1).optional(),
      }),
    )
    .query(({ ctx, input }) =>
      findDuplicateBills(
        ctx.db,
        input.billTypeId,
        input.periodYear,
        input.periodMonth,
        input.excludeBillId,
      ),
    ),
});
