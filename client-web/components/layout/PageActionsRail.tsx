"use client";

import { createContext, useContext } from "react";

export const PageActionsHostContext = createContext<HTMLElement | null | undefined>(undefined);

export function usePageActionsHost() {
  return useContext(PageActionsHostContext);
}
