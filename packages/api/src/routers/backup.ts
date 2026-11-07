import { z } from "zod";

import { publicProcedure, router } from "../index";
import {
  backupCounts,
  createBackupPayload,
  MAX_BACKUP_BYTES,
  parseBackupJson,
  restoreBackup,
} from "../treasurer/backup";

const jsonInput = z.object({
  json: z.string().max(MAX_BACKUP_BYTES),
});

export const backupRouter = router({
  export: publicProcedure.query(({ ctx }) => createBackupPayload(ctx.db)),
  inspect: publicProcedure.input(jsonInput).query(({ input }) => {
    const payload = parseBackupJson(input.json);
    return {
      exportedAt: payload.exportedAt,
      counts: backupCounts(payload),
    };
  }),
  import: publicProcedure.input(jsonInput).mutation(async ({ ctx, input }) => {
    const payload = parseBackupJson(input.json);
    await restoreBackup(ctx.db, payload);
    return { counts: backupCounts(payload) };
  }),
});
