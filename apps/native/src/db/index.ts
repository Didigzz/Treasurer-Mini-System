import { drizzle } from "drizzle-orm/expo-sqlite";
import { openDatabaseAsync, openDatabaseSync, type SQLiteDatabase } from "expo-sqlite";
import { Platform } from "react-native";

export const TREASURER_DATABASE_NAME = "treasurer.db";

export let sqlite: SQLiteDatabase;
export let db: TreasurerDatabase;
export type TreasurerDatabase = ReturnType<typeof drizzle>;

let initialization: Promise<void> | null = null;

/**
 * Open the local database once.
 *
 * On web, expo-sqlite's synchronous bridge blocks the main thread while the
 * SQLite worker starts. Warming the same database with the async API first
 * initializes the worker and reuses the open connection, so the sync handle
 * Drizzle needs can be created without timing out.
 */
export function initializeDatabase(): Promise<void> {
  if (!initialization) {
    initialization = (async () => {
      if (Platform.OS === "web") {
        const warm = await openDatabaseAsync(TREASURER_DATABASE_NAME);
        await warm.execAsync("PRAGMA journal_mode = WAL;");
        await warm.execAsync("PRAGMA foreign_keys = ON;");
      }

      sqlite = openDatabaseSync(TREASURER_DATABASE_NAME);
      sqlite.execSync("PRAGMA journal_mode = WAL;");
      sqlite.execSync("PRAGMA foreign_keys = ON;");
      db = drizzle(sqlite);
    })().catch((error: unknown) => {
      initialization = null;
      throw error;
    });
  }

  return initialization;
}

export function withTransaction(task: () => Promise<unknown>): Promise<void> {
  return sqlite.withTransactionAsync(async () => {
    await task();
  });
}

export async function resetDatabaseConnectionState(): Promise<void> {
  await sqlite.execAsync("PRAGMA wal_checkpoint(TRUNCATE);");
}
