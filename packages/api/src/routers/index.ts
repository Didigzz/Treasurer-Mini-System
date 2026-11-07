import { publicProcedure, router } from "../index";
import { backupRouter } from "./backup";
import { billTypesRouter } from "./bill-types";
import { billsRouter } from "./bills";
import { boardersRouter } from "./boarders";
import { paymentsRouter } from "./payments";
import { settingsRouter } from "./settings";

export const appRouter = router({
  healthCheck: publicProcedure.query(() => {
    return "OK";
  }),
  settings: settingsRouter,
  billTypes: billTypesRouter,
  boarders: boardersRouter,
  bills: billsRouter,
  payments: paymentsRouter,
  backup: backupRouter,
});
export type AppRouter = typeof appRouter;
