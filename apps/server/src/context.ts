import type { Context as ApiContext } from "@my-better-t-app/api/context";
import type { Context as HonoContext } from "hono";

import { db } from "./services";

export type CreateContextOptions = {
  context: HonoContext;
};

export async function createContext(_options: CreateContextOptions): Promise<ApiContext> {
  return {
    db,
  };
}

export type Context = Awaited<ReturnType<typeof createContext>>;
