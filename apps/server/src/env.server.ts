/**
 * Server runtime environment.
 *
 * Bun loads `apps/server/.env` automatically in development (see bunfig.toml).
 * The packaged desktop sidecar instead receives the values from the Tauri
 * shell, so the same accessor works in both runtimes.
 *
 * Varlock still owns `.env.schema` and the generated `src/env.ts` types; its
 * auto-loader crashes Bun 1.3.x on Windows, so the server reads the resolved
 * `process.env` directly at startup.
 */
export type ServerEnv = {
  NODE_ENV: "development" | "production" | "test";
  CORS_ORIGIN: string;
  DATABASE_URL: string;
};

function readNodeEnv(): ServerEnv["NODE_ENV"] {
  const value = process.env.NODE_ENV;
  return value === "production" || value === "test" || value === "development"
    ? value
    : "development";
}

export const ENV: ServerEnv = {
  NODE_ENV: readNodeEnv(),
  CORS_ORIGIN: process.env.CORS_ORIGIN ?? "http://localhost:1420",
  DATABASE_URL: process.env.DATABASE_URL ?? "file:local.db",
};
