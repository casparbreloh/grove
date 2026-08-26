import { useCallback, useLayoutEffect, useRef, useState } from "react";

type ScrollAxis = "horizontal" | "vertical";

const noHiddenContent = {
  hasHiddenContentAtEnd: false,
  hasHiddenContentAtStart: false,
};

export function useScrollOverflow(axis: ScrollAxis, contentCount: number) {
  const scrollElementRef = useRef<HTMLDivElement>(null);
  const [hiddenContent, setHiddenContent] = useState(noHiddenContent);

  const updateHiddenContent = useCallback(() => {
    const scrollElement = scrollElementRef.current;
    if (!scrollElement) return;

    const scrollPosition =
      axis === "horizontal" ? scrollElement.scrollLeft : scrollElement.scrollTop;
    const scrollSize =
      axis === "horizontal" ? scrollElement.scrollWidth : scrollElement.scrollHeight;
    const viewportSize =
      axis === "horizontal" ? scrollElement.clientWidth : scrollElement.clientHeight;
    const maximumScrollPosition = Math.max(0, scrollSize - viewportSize);
    const nextHiddenContent = {
      hasHiddenContentAtStart: scrollPosition > 1,
      hasHiddenContentAtEnd: scrollPosition < maximumScrollPosition - 1,
    };

    setHiddenContent((currentHiddenContent) =>
      currentHiddenContent.hasHiddenContentAtStart === nextHiddenContent.hasHiddenContentAtStart &&
      currentHiddenContent.hasHiddenContentAtEnd === nextHiddenContent.hasHiddenContentAtEnd
        ? currentHiddenContent
        : nextHiddenContent,
    );
  }, [axis]);

  useLayoutEffect(() => {
    const scrollElement = scrollElementRef.current;
    if (!scrollElement) return;

    const resizeObserver = new ResizeObserver(updateHiddenContent);
    resizeObserver.observe(scrollElement);
    for (const child of scrollElement.children) resizeObserver.observe(child);
    updateHiddenContent();

    return () => resizeObserver.disconnect();
  }, [contentCount, updateHiddenContent]);

  return { ...hiddenContent, onScroll: updateHiddenContent, scrollElementRef };
}
