import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { createTRPCClient, httpBatchLink } from "@trpc/client";
import type { AppRouter } from "@my-better-t-app/api/routers/index";
import { StrictMode, useEffect, useMemo, useState, type ReactNode } from "react";
import { createRoot } from "react-dom/client";

import { resolveServerUrl, waitForServer } from "@/env";
import { router } from "@/router";
import { TRPCProvider } from "@/trpc";
import { Button } from "@/components/ui";
import "./styles.css";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 5_000,
      retry: 1,
      refetchOnWindowFocus: false,
    },
  },
});

type BootstrapState =
  | { status: "loading" }
  | { status: "error"; message: string }
  | { status: "ready"; url: string };

function Splash({ children }: { children: ReactNode }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-4 px-6 text-center">
      {children}
    </div>
  );
}

function Connected({ url }: { url: string }) {
  const client = useMemo(
    () => createTRPCClient<AppRouter>({ links: [httpBatchLink({ url: `${url}/trpc` })] }),
    [url],
  );

  return (
    <QueryClientProvider client={queryClient}>
      <TRPCProvider trpcClient={client} queryClient={queryClient}>
        <RouterProvider router={router} />
      </TRPCProvider>
    </QueryClientProvider>
  );
}

function Bootstrap() {
  const [state, setState] = useState<BootstrapState>({ status: "loading" });
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    let cancelled = false;
    setState({ status: "loading" });

    void (async () => {
      try {
        const url = await resolveServerUrl();
        await waitForServer(url);
        if (!cancelled) setState({ status: "ready", url });
      } catch (error) {
        if (!cancelled) {
          setState({
            status: "error",
            message:
              error instanceof Error
                ? error.message
                : "The treasurer server could not be reached.",
          });
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [attempt]);

  if (state.status === "loading") {
    return (
      <Splash>
        <div className="h-9 w-9 animate-spin rounded-full border-2 border-line border-t-accent" />
        <div>
          <p className="text-sm font-semibold text-ink">Starting the treasurer server…</p>
          <p className="mt-1 text-xs text-muted">Checking the local database and health endpoint.</p>
        </div>
      </Splash>
    );
  }

  if (state.status === "error") {
    return (
      <Splash>
        <div className="card w-full max-w-md p-6">
          <h1 className="text-base font-bold text-ink">The server is not ready</h1>
          <p className="mt-2 text-sm text-muted">{state.message}</p>
          <p className="mt-3 text-xs text-muted">
            Make sure the sidecar is bundled, or run the Bun server with{" "}
            <code className="rounded bg-canvas px-1 py-0.5">bun run dev:server</code>.
          </p>
          <Button className="mt-5 w-full" onClick={() => setAttempt((value) => value + 1)}>
            Retry
          </Button>
        </div>
      </Splash>
    );
  }

  return <Connected url={state.url} />;
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Bootstrap />
  </StrictMode>,
);
