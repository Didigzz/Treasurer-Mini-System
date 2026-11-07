export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * In the packaged app the Rust shell picks a free port and passes it to the
 * sidecar; in a plain browser dev session we talk to the Bun dev server.
 */
export async function resolveServerUrl(): Promise<string> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return (await invoke<string>("server_url")).replace(/\/$/, "");
  }

  return (import.meta.env.VITE_SERVER_URL ?? "http://127.0.0.1:3000").replace(/\/$/, "");
}

export async function waitForServer(url: string, attempts = 100): Promise<void> {
  let lastError: unknown;

  for (let attempt = 0; attempt < attempts; attempt += 1) {
    try {
      const response = await fetch(`${url}/health`);
      if (response.ok) {
        const body = (await response.json()) as { status?: string; database?: string };
        if (body.status === "ok" && body.database === "ready") return;
      }
      lastError = new Error(`Health check returned ${response.status}.`);
    } catch (error) {
      lastError = error;
    }

    await new Promise((resolve) => setTimeout(resolve, 300));
  }

  throw lastError instanceof Error
    ? lastError
    : new Error("The treasurer server did not become ready.");
}
