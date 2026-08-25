import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { PaneSplit, PaneSplitProvider, usePaneSplit } from "./pane-split";

afterEach(cleanup);

function PaneControls() {
  const { isSidePaneOpen, toggleSidePane } = usePaneSplit();

  return (
    <button aria-pressed={isSidePaneOpen} onClick={toggleSidePane} type="button">
      Toggle side pane
    </button>
  );
}

function StatefulSidePane() {
  const [count, setCount] = useState(0);

  return <button onClick={() => setCount((value) => value + 1)}>Count {count}</button>;
}

function renderOpenPaneSplit() {
  render(
    <PaneSplitProvider defaultSidePaneOpen>
      <PaneSplit mainPane={<div>Main</div>} sidePane={<StatefulSidePane />} />
      <PaneControls />
    </PaneSplitProvider>,
  );
}

describe("PaneSplit", () => {
  it("does not close the side pane when the resize rail is clicked", () => {
    renderOpenPaneSplit();

    fireEvent.click(screen.getByRole("separator", { name: "Main and side pane divider" }));

    expect(
      screen.getByRole("button", { name: "Toggle side pane" }).getAttribute("aria-pressed"),
    ).toBe("true");
  });

  it("resizes without closing the side pane", () => {
    renderOpenPaneSplit();
    const split = document.querySelector<HTMLElement>('[data-slot="pane-split"]');
    const mainPane = document.querySelector<HTMLElement>('[data-slot="main-pane"]');
    const rail = screen.getByRole("separator", { name: "Main and side pane divider" });
    Object.defineProperty(split, "clientWidth", { configurable: true, value: 1200 });
    vi.spyOn(mainPane!, "getBoundingClientRect").mockReturnValue({
      bottom: 0,
      height: 0,
      left: 0,
      right: 600,
      toJSON: () => ({}),
      top: 0,
      width: 600,
      x: 0,
      y: 0,
    });
    rail.setPointerCapture = vi.fn();
    rail.hasPointerCapture = () => false;

    fireEvent.pointerDown(rail, { button: 0, clientX: 600, pointerId: 1 });
    fireEvent.pointerMove(rail, { clientX: 680, pointerId: 1 });
    fireEvent.pointerUp(rail, { clientX: 680, pointerId: 1 });

    expect(split?.style.getPropertyValue("--main-pane-intent")).toBe("680px");
    expect(
      screen.getByRole("button", { name: "Toggle side pane" }).getAttribute("aria-pressed"),
    ).toBe("true");
  });

  it("keeps side-pane layout and component state while the pane is hidden", () => {
    renderOpenPaneSplit();
    const sidePane = document.querySelector<HTMLElement>('[data-slot="side-pane"]');
    const width = sidePane?.style.width;
    fireEvent.click(screen.getByRole("button", { name: "Count 0" }));

    fireEvent.click(screen.getByRole("button", { name: "Toggle side pane" }));

    expect(width).not.toBe("");
    expect(sidePane?.style.width).toBe(width);
    expect(screen.getByRole("button", { name: "Count 1", hidden: true })).toBeDefined();

    fireEvent.click(screen.getByRole("button", { name: "Toggle side pane" }));
    expect(screen.getByRole("button", { name: "Count 1" })).toBeDefined();
  });
});
