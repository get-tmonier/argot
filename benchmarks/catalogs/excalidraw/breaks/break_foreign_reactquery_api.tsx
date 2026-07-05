import React from "react";
import { useQuery } from "@tanstack/react-query";

// Break: @tanstack/react-query useQuery to fetch published library items
// where excalidraw loads library data through its own promise-based data
// layer + jotai; react-query is 0-usage at the pinned SHA and absent from
// package.json.
export const useRemoteLibrary = (endpoint: string) => {
  const { data, isLoading } = useQuery({
    queryKey: ["library", endpoint],
    queryFn: async () => {
      const response = await fetch(endpoint);
      return response.json();
    },
  });
  return { items: data ?? [], isLoading };
};
