import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";

const desktopDir = resolve(import.meta.dir, "..");
const repoRoot = resolve(desktopDir, "..", "..");
const serverDir = join(repoRoot, "apps", "server");

const webPort = 1420;
const apiUrl = (process.env.VITE_SERVER_URL ?? "http://127.0.0.1:3000").replace(/\/$/, "");

const requireFromDesktop = createRequire(join(desktopDir, "package.json"));
const viteBin = join(dirname(requireFromDesktop.resolve("vite/package.json")), "bin", "vite.js");

type Role = "web" | "api";
type Child = ReturnType<typeof Bun.spawn>;

let shuttingDown = false;

function log(role: Role, message: string): void {
  console.log(`[${role}] ${message}`);
}

async function pump(role: Role, stream: ReadableStream<Uint8Array> | undefined): Promise<void> {
  if (!stream) return;

  const decoder = new TextDecoder();
  let buffer = "";

  for await (const chunk of stream) {
    buffer += decoder.decode(chunk, { stream: true });
    const lines = buffer.split("\n");
    buffer = lines.pop() ?? "";
    for (const line of lines) {
      if (line.trim() !== "") log(role, line);
    }
  }

  if (buffer.trim() !== "") log(role, buffer);
}

function start(role: Role, command: string[], cwd: string, env: Record<string, string | undefined>): Child {
  const child = Bun.spawn(command, { cwd, env, stdout: "pipe", stderr: "pipe" });
  void pump(role, child.stdout);
  void pump(role, child.stderr);
  return child;
}

async function apiIsUp(): Promise<boolean> {
  try {
    const response = await fetch(`${apiUrl}/health`, { signal: AbortSignal.timeout(1_000) });
    return response.ok;
  } catch {
    return false;
  }
}

async function main(): Promise<number> {
  const web = start(
    "web",
    [process.execPath, viteBin, "--port", String(webPort), "--strictPort"],
    desktopDir,
    process.env,
  );

  const api = (await apiIsUp())
    ? null
    : start("api", [process.execPath, "--hot", join(serverDir, "src", "index.ts")], serverDir, {
        ...process.env,
        NODE_ENV: process.env.NODE_ENV ?? "development",
        // The Vite page runs on the web origin, so the API has to allow it.
        CORS_ORIGIN: process.env.CORS_ORIGIN ?? `http://localhost:${webPort}`,
      });

  log(
    api ? "api" : "web",
    api ? `starting the treasurer server on ${apiUrl}` : `${apiUrl} already answers, reusing the running server`,
  );

  const stop = (): void => {
    shuttingDown = true;
    for (const child of [web, api]) {
      if (child && child.exitCode === null) child.kill();
    }
  };

  process.on("SIGINT", stop);
  process.on("SIGTERM", stop);

  const apiExit = api
    ? api.exited.then((code) => ({ role: "api" as const, code }))
    : new Promise<{ role: "api"; code: number }>(() => {});
  const first = await Promise.race([
    web.exited.then((code) => ({ role: "web" as const, code })),
    apiExit,
  ]);

  if (first.role === "api" && !shuttingDown && (await apiIsUp())) {
    // Another `dev` task already owns the port (for example `bun run dev` at the root).
    log("api", "the server process ended, but the API still answers — keeping Vite running");
    return await web.exited;
  }

  if (!shuttingDown) log(first.role, `exited with code ${first.code}, stopping the other process`);
  stop();

  await Promise.all([web.exited, api?.exited ?? Promise.resolve(0)]);
  return first.code;
}

process.exit(await main());
