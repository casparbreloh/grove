import { Terminal as WtermTerminal, useTerminal } from "@wterm/react";
import { useCallback, useRef } from "react";
import type { CSSProperties } from "react";

const prompt = "\x1b[32m❯\x1b[0m ";

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
        } as CSSProperties
      }
    />
  );
}
