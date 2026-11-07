import { z } from "zod";

import { publicProcedure, router } from "../index";
import {
  createBoarder,
  getAllActiveBoardersForPicker,
  getActiveBoarders,
  getBoarder,
  getBoarderLedger,
  getBoarderRow,
  listBoardersWithBalances,
  renameBoarder,
  setBoarderArchived,
} from "../treasurer/repository";

const idInput = z.object({ id: z.string().min(1) });
const nameInput = z.string().trim().min(1).max(80);

export const boardersRouter = router({
  listWithBalances: publicProcedure.query(({ ctx }) => listBoardersWithBalances(ctx.db)),
  active: publicProcedure.query(({ ctx }) => getActiveBoarders(ctx.db)),
  pickerList: publicProcedure.query(({ ctx }) => getAllActiveBoardersForPicker(ctx.db)),
  get: publicProcedure.input(idInput).query(({ ctx, input }) => getBoarder(ctx.db, input.id)),
  getRow: publicProcedure
    .input(idInput)
    .query(({ ctx, input }) => getBoarderRow(ctx.db, input.id)),
  create: publicProcedure
    .input(z.object({ name: nameInput }))
    .mutation(({ ctx, input }) => createBoarder(ctx.db, input.name)),
  rename: publicProcedure
    .input(idInput.extend({ name: nameInput }))
    .mutation(async ({ ctx, input }) => {
      await renameBoarder(ctx.db, input.id, input.name);
    }),
  setArchived: publicProcedure
    .input(idInput.extend({ isArchived: z.boolean() }))
    .mutation(async ({ ctx, input }) => {
      await setBoarderArchived(ctx.db, input.id, input.isArchived);
    }),
  ledger: publicProcedure.input(idInput).query(({ ctx, input }) =>
    getBoarderLedger(ctx.db, input.id),
  ),
});
