import { trpcServer } from "@hono/trpc-server";
import { appRouter } from "@my-better-t-app/api/routers/index";
import { seedDatabase } from "@my-better-t-app/api/treasurer/seed";
import { migrateDatabase } from "@my-better-t-app/db/migrate";
import { sql } from "drizzle-orm";
import { Hono } from "hono";
import { cors } from "hono/cors";
import { logger } from "hono/logger";

import { createContext } from "./context";
import { ENV } from "./env.server";
import { db } from "./services";

await migrateDatabase(db);
await seedDatabase(db);

const app = new Hono();

const allowedOrigins = ENV.CORS_ORIGIN.split(",")
  .map((origin) => origin.trim())
  .filter(Boolean);

app.use(logger());
app.use(
  "/*",
  cors({
    origin: (origin) => {
      if (!origin) return undefined;
      return allowedOrigins.includes("*") || allowedOrigins.includes(origin)
        ? origin
        : undefined;
    },
    allowMethods: ["GET", "POST", "OPTIONS"],
    allowHeaders: ["content-type"],
  }),
);

app.use(
  "/trpc/*",
  trpcServer({
    endpoint: "/trpc",
    router: appRouter,
    createContext: (_opts, context) => {
      return createContext({ context });
    },
  }),
);

app.get("/", (c) => {
  return c.text("OK");
});

app.get("/health", async (c) => {
  try {
    await db.run(sql`SELECT 1`);
    return c.json({ status: "ok", database: "ready" });
  } catch (error) {
    const message = error instanceof Error ? error.message : "Database unavailable.";
    return c.json({ status: "error", database: message }, 503);
  }
});

// Bun serves this default export automatically, honouring process.env.PORT.
// The Tauri sidecar passes PORT plus DATABASE_URL and CORS_ORIGIN.
export default app;
