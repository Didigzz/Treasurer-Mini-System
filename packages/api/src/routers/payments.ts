import { z } from "zod";

import { publicProcedure, router } from "../index";
import { MAX_AMOUNT_MINOR } from "../treasurer/money";
import {
  getSharePaymentContext,
  markChangeReturned,
  recordPayment,
  voidPayment,
} from "../treasurer/repository";

export const paymentsRouter = router({
  context: publicProcedure
    .input(z.object({ shareId: z.string().min(1) }))
    .query(({ ctx, input }) => getSharePaymentContext(ctx.db, input.shareId)),
  record: publicProcedure
    .input(
      z.object({
        billShareId: z.string().min(1),
        amountMinor: z.number().int().min(1).max(MAX_AMOUNT_MINOR),
        paidAt: z.number().int().nonnegative(),
        note: z.string().max(500).nullable(),
      }),
    )
    .mutation(({ ctx, input }) => recordPayment(ctx.db, input)),
  void: publicProcedure
    .input(
      z.object({
        paymentId: z.string().min(1),
        reason: z.string().trim().min(1).max(200),
      }),
    )
    .mutation(async ({ ctx, input }) => {
      await voidPayment(ctx.db, input.paymentId, input.reason);
    }),
  markChangeReturned: publicProcedure
    .input(
      z.object({
        paymentId: z.string().min(1),
        returnedAt: z.number().int().nonnegative(),
        note: z.string().max(200).nullable(),
      }),
    )
    .mutation(async ({ ctx, input }) => {
      await markChangeReturned(ctx.db, input.paymentId, input.returnedAt, input.note);
    }),
});
