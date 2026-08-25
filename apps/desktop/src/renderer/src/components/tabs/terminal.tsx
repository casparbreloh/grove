import { Terminal as WtermTerminal, useTerminal } from "@wterm/react";
import { useCallback, useRef } from "react";
import type { CSSProperties } from "react";

const prompt = "❯ ";

export function Terminal({ terminalId }: { terminalId: string }) {
  const { ref, write } = useTerminal();
  const input = useRef("");

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
    <WtermTerminal
      aria-label={`Terminal ${terminalId}`}
      autoResize
      className="h-full w-full rounded-none! shadow-none!"
      cursorBlink
      onData={handleData}
      onReady={() =>
        write(
          `Grove terminal prototype\r\nWorking directory: ~\r\nType 'help' for available commands.\r\n\r\n${prompt}`,
        )
      }
      ref={ref}
      style={
        {
          "--term-bg": "var(--background)",
          "--term-fg": "var(--foreground)",
          "--term-cursor": "var(--foreground)",
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
        } as CSSProperties
      }
    />
  );
}
