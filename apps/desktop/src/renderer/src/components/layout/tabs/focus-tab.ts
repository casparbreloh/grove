export function focusTab(tabId: string) {
  window.requestAnimationFrame(() => document.getElementById(`${tabId}-tab`)?.focus());
}
