import { useCallback, useEffect, useRef } from "react";

type ScrollFadeOrientation = "horizontal" | "vertical";

export function useScrollFade(orientation: ScrollFadeOrientation) {
  const ref = useRef<HTMLDivElement>(null);

  const updateScrollFade = useCallback(() => {
    const element = ref.current;
    if (!element) return;

    const [position, viewportSize, contentSize] =
      orientation === "vertical"
        ? [element.scrollTop, element.clientHeight, element.scrollHeight]
        : [element.scrollLeft, element.clientWidth, element.scrollWidth];
    const overflow = contentSize - viewportSize;

    element.dataset.scrollFadeStart = String(position > 0);
    element.dataset.scrollFadeEnd = String(position < overflow - 1);
  }, [orientation]);

  useEffect(() => {
    const element = ref.current;
    if (!element) return;

    const observer = new ResizeObserver(updateScrollFade);
    const mutations = new MutationObserver(updateScrollFade);
    observer.observe(element);
    mutations.observe(element, { childList: true, subtree: true });
    updateScrollFade();
    return () => {
      mutations.disconnect();
      observer.disconnect();
    };
  }, [updateScrollFade]);

  return { onScroll: updateScrollFade, ref };
}
