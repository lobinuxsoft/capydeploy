/**
 * usePanelState - Persists panel expanded/collapsed state across unmount cycles.
 * Uses a module-level Map so state survives QAM panel close/open.
 */

import { useState, useCallback } from "react";

const store = new Map<string, boolean>();

export function usePanelState(key: string, defaultValue = true): [boolean, () => void] {
  const [expanded, setExpanded] = useState(() => store.has(key) ? store.get(key)! : defaultValue);

  const toggle = useCallback(() => {
    setExpanded((prev) => {
      const next = !prev;
      store.set(key, next);
      return next;
    });
  }, [key]);

  return [expanded, toggle];
}
