import { Terminal as WtermTerminal, useTerminal, type WTerm } from "@wterm/react";
import { useCallback, useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";

const prompt = "❯ ";
const defaultTerminalSize = { cols: 80, rows: 24 };
const terminalResizeSettleMs = 160;
// SAFETY: Wterm consumes these CSS custom properties as strings without type conversion.
const terminalTheme = {
  "--term-bg": "var(--background)",
  "--term-fg": "var(--foreground)",
  "--term-cursor": "var(--foreground)",
  "--term-font-family": "var(--font-mono)",
  "--term-font-size": "var(--text-xs)",
  "--term-line-height": "var(--text-xs--line-height)",
  "--term-row-height": "var(--text-xs--line-height)",
  "--term-color-0": "color-mix(in oklch, var(--foreground) 35%, var(--background))",
  "--term-color-1": "var(--destructive)",
  "--term-color-2": "var(--success)",
  "--term-color-3": "var(--warning)",
  "--term-color-4": "var(--info)",
  "--term-color-5": "color-mix(in oklch, var(--destructive) 55%, var(--info))",
  "--term-color-6": "color-mix(in oklch, var(--success) 55%, var(--info))",
  "--term-color-7": "color-mix(in oklch, var(--foreground) 75%, var(--background))",
  "--term-color-8": "var(--muted-foreground)",
  "--term-color-9": "color-mix(in oklch, var(--destructive) 80%, var(--foreground))",
  "--term-color-10": "color-mix(in oklch, var(--success) 80%, var(--foreground))",
  "--term-color-11": "color-mix(in oklch, var(--warning) 80%, var(--foreground))",
  "--term-color-12": "color-mix(in oklch, var(--info) 80%, var(--foreground))",
  "--term-color-13": "color-mix(in oklch, var(--destructive) 45%, var(--info))",
  "--term-color-14": "color-mix(in oklch, var(--success) 45%, var(--info))",
  "--term-color-15": "var(--foreground)",
} as CSSProperties;

type CellSize = Readonly<{
  height: number;
  width: number;
}>;

function measureCell(element: HTMLElement): CellSize | undefined {
  const row = document.createElement("div");
  const character = document.createElement("span");
  row.className = "term-row";
  row.style.position = "absolute";
  row.style.visibility = "hidden";
  character.textContent = "W";
  row.append(character);
  element.append(row);

  const width = character.getBoundingClientRect().width;
  const height = row.getBoundingClientRect().height;
  row.remove();

  return width > 0 && height > 0 ? { height, width } : undefined;
}

export function Terminal({ terminalId }: { terminalId: string }) {
  const { ref, write } = useTerminal();
  const [size, setSize] = useState(defaultTerminalSize);
  const hostRef = useRef<HTMLDivElement>(null);
  const instanceRef = useRef<WTerm | null>(null);
  const resizeTimerRef = useRef<number>(undefined);
  const initializedRef = useRef(false);
  const input = useRef("");

  const fitTerminal = useCallback((instance = instanceRef.current) => {
    if (!instance) return false;

    const { element } = instance;
    if (element.clientWidth <= 0 || element.clientHeight <= 0) return false;

    const cellSize = measureCell(element);
    if (!cellSize) return false;

    const style = window.getComputedStyle(element);
    const horizontalPadding =
      (Number.parseFloat(style.paddingLeft) || 0) + (Number.parseFloat(style.paddingRight) || 0);
    const verticalPadding =
      (Number.parseFloat(style.paddingTop) || 0) + (Number.parseFloat(style.paddingBottom) || 0);
    const nextSize = {
      cols: Math.max(1, Math.floor((element.clientWidth - horizontalPadding) / cellSize.width)),
      rows: Math.max(1, Math.floor((element.clientHeight - verticalPadding) / cellSize.height)),
    };

    if (nextSize.cols !== instance.cols || nextSize.rows !== instance.rows) {
      instance.resize(nextSize.cols, nextSize.rows);
    }
    setSize((currentSize) =>
      currentSize.cols === nextSize.cols && currentSize.rows === nextSize.rows
        ? currentSize
        : nextSize,
    );
    return true;
  }, []);

  const scheduleFit = useCallback(() => {
    window.clearTimeout(resizeTimerRef.current);
    resizeTimerRef.current = window.setTimeout(() => {
      if (!fitTerminal() || initializedRef.current) return;
      instanceRef.current?.write(
        `Grove terminal prototype\r\nWorking directory: ~\r\nType 'help' for available commands.\r\n\r\n${prompt}`,
      );
      initializedRef.current = true;
    }, terminalResizeSettleMs);
  }, [fitTerminal]);

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;

    const observer = new ResizeObserver(([entry]) => {
      if (!entry || entry.contentRect.width <= 0 || entry.contentRect.height <= 0) return;
      scheduleFit();
    });
    observer.observe(host);

    return () => {
      window.clearTimeout(resizeTimerRef.current);
      observer.disconnect();
      instanceRef.current = null;
    };
  }, [scheduleFit]);

  const handleData = useCallback(
    (data: string) => {
      if (data === "\r") {
        const command = input.current.trim();
        input.current = "";

        if (command === "clear") {
          write("\x1b[2J\x1b[H");
        } else {
          const output =
            command === "pwd"
              ? "~"
              : command === "help"
                ? "Prototype commands: help, pwd, clear"
                : command
                  ? `Command execution is not connected yet: ${command}`
                  : "";
          write(`\r\n${output}${output ? "\r\n" : ""}`);
        }
        write(prompt);
        return;
      }

      if (data === "\x7f") {
        if (!input.current) return;
        input.current = input.current.slice(0, -1);
        write("\b \b");
        return;
      }

      if (data.startsWith("\x1b") || data < " ") return;
      input.current += data;
      write(data);
    },
    [write],
  );

  return (
    <div className="size-full" ref={hostRef}>
      <WtermTerminal
        aria-label={`Terminal ${terminalId}`}
        autoResize={false}
        className="h-full! w-full! rounded-none! shadow-none!"
        cols={size.cols}
        cursorBlink
        onData={handleData}
        onReady={(instance) => {
          instanceRef.current = instance;
          initializedRef.current = false;
          scheduleFit();
        }}
        ref={ref}
        rows={size.rows}
        style={terminalTheme}
      />
    </div>
  );
}
