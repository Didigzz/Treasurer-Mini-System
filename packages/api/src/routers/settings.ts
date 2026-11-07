import { z } from "zod";

import { publicProcedure, router } from "../index";
import { isCurrencyCode } from "../treasurer/money";
import {
  getAppSettings,
  setAllowCentavos,
  setCurrencyCode,
  setLastBackupAt,
} from "../treasurer/settings";

export const settingsRouter = router({
  get: publicProcedure.query(({ ctx }) => getAppSettings(ctx.db)),
  setCurrency: publicProcedure
    .input(
      z.object({
        currencyCode: z.string().refine(isCurrencyCode, "Pick a supported currency."),
      }),
    )
    .mutation(async ({ ctx, input }) => {
      await setCurrencyCode(ctx.db, input.currencyCode);
    }),
  setAllowCentavos: publicProcedure
    .input(z.object({ allowCentavos: z.boolean() }))
    .mutation(async ({ ctx, input }) => {
      await setAllowCentavos(ctx.db, input.allowCentavos);
    }),
  setLastBackupAt: publicProcedure
    .input(z.object({ value: z.string().min(1) }))
    .mutation(async ({ ctx, input }) => {
      await setLastBackupAt(ctx.db, input.value);
    }),
});
