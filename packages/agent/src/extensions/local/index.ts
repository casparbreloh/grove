import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

/** Grove's local extension composition for Pi-compatible agent hosts. */
export function localExtension(pi: ExtensionAPI): void {
  pi.on("session_start", (_event, context) => {
    context.ui.setStatus("grove", context.ui.theme.fg("muted", "Grove"));
  });

  pi.on("session_shutdown", (_event, context) => {
    context.ui.setStatus("grove", undefined);
  });
}

export default localExtension;
