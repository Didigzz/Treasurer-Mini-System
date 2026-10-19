import { QueryClient } from "@tanstack/react-query";

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      // The database is local; keep data fresh when screens are revisited.
      staleTime: 5_000,
      retry: 1,
    },
  },
});
