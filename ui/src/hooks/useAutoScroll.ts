import { useEffect, useRef } from "react";

export function useAutoScroll<E extends HTMLElement>(dep: unknown, enabled: boolean) {
  const ref = useRef<E>(null);
  useEffect(() => {
    if (enabled && ref.current) ref.current.scrollTop = ref.current.scrollHeight;
  }, [dep, enabled]);
  return ref;
}
