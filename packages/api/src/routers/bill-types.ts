import { z } from "zod";

import { publicProcedure, router } from "../index";
import {
  createBillType,
  getBillTypeById,
  listBillTypes,
  renameBillType,
  setBillTypeArchived,
} from "../treasurer/repository";

const idInput = z.object({ id: z.string().min(1) });
const nameInput = z.string().trim().min(1).max(40);

export const billTypesRouter = router({
  list: publicProcedure.query(({ ctx }) => listBillTypes(ctx.db)),
  create: publicProcedure.input(z.object({ name: nameInput })).mutation(({ ctx, input }) =>
    createBillType(ctx.db, input.name),
  ),
  rename: publicProcedure
    .input(idInput.extend({ name: nameInput }))
    .mutation(async ({ ctx, input }) => {
      await renameBillType(ctx.db, input.id, input.name);
    }),
  setArchived: publicProcedure
    .input(idInput.extend({ isArchived: z.boolean() }))
    .mutation(async ({ ctx, input }) => {
      await setBillTypeArchived(ctx.db, input.id, input.isArchived);
    }),
  getById: publicProcedure.input(idInput).query(({ ctx, input }) =>
    getBillTypeById(ctx.db, input.id),
  ),
});
