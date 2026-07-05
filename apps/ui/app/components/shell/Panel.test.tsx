import { cleanup, fireEvent, render, screen } from "@testing-library/react";
/// <reference lib="dom" />
import { afterEach, describe, expect, mock, test } from "bun:test";

import { TooltipProvider } from "~/components/ui/tooltip";

import { Panel } from "./Panel";

afterEach(cleanup);

// Minimal dataTransfer stub -- happy-dom's DataTransfer is incomplete, but fireEvent accepts a
// plain object with the properties xterm/panel drag handlers actually read (types/getData/setData).
function stubDataTransfer(initial?: Record<string, string>) {
  const store = new Map(Object.entries(initial ?? {}));
  return {
    dropEffect: "none",
    effectAllowed: "uninitialized",
    get types() {
      return [...store.keys()];
    },
    setData: (type: string, value: string) => store.set(type, value),
    getData: (type: string) => store.get(type) ?? "",
  };
}

function renderLeaf(props: {
  draggable?: boolean;
  onDragStart?: (e: React.DragEvent) => void;
  isDropTarget?: boolean;
  isClosing?: boolean;
  onDrop?: (e: React.DragEvent) => void;
  onDragOver?: (e: React.DragEvent) => void;
}) {
  return render(
    <TooltipProvider>
      <Panel.Provider
        id="leaf-1"
        title="Session · Terminal"
        actions={{ split: () => {}, close: () => {} }}
      >
        <Panel.Frame
          isClosing={props.isClosing}
          isDropTarget={props.isDropTarget}
          onDragOver={props.onDragOver}
          onDrop={props.onDrop}
        >
          <Panel.Header draggable={props.draggable} onDragStart={props.onDragStart}>
            <Panel.Title />
            <Panel.Toolbar>
              <Panel.Toolbar.Button icon={<span />} label="Split right" onClick={() => {}} />
              <Panel.CloseButton canClose />
            </Panel.Toolbar>
          </Panel.Header>
          <Panel.Content>content</Panel.Content>
        </Panel.Frame>
      </Panel.Provider>
    </TooltipProvider>,
  );
}

describe("Panel toolbar tooltips", () => {
  test("every icon-only toolbar button carries an accessible name naming the action", () => {
    renderLeaf({});
    expect(screen.getByRole("button", { name: "Split right" })).not.toBeNull();
    expect(screen.getByRole("button", { name: "Close panel" })).not.toBeNull();
  });

  test("the close control is hidden when the leaf cannot be closed", () => {
    render(
      <TooltipProvider>
        <Panel.Provider id="leaf-1" title="Empty" actions={{ split: () => {}, close: () => {} }}>
          <Panel.Frame>
            <Panel.Header>
              <Panel.Toolbar>
                <Panel.CloseButton canClose={false} />
              </Panel.Toolbar>
            </Panel.Header>
            <Panel.Content>content</Panel.Content>
          </Panel.Frame>
        </Panel.Provider>
      </TooltipProvider>,
    );
    expect(screen.queryByRole("button", { name: "Close panel" })).toBeNull();
  });
});

describe("Panel lifecycle motion", () => {
  test("a closing leaf renders opacity-0 and pointer-events-none", () => {
    renderLeaf({ isClosing: true });
    const frame = document.querySelector("[data-panel-id='leaf-1']");
    expect(frame?.className).toContain("opacity-0");
    expect(frame?.className).toContain("pointer-events-none");
    expect(frame?.getAttribute("data-state")).toBe("closing");
  });

  test("a live (non-closing) leaf is not marked pointer-events-none", () => {
    renderLeaf({});
    const frame = document.querySelector("[data-panel-id='leaf-1']");
    expect(frame?.className).not.toContain("pointer-events-none");
  });
});

describe("Panel drag/drop swap wiring", () => {
  test("a draggable header calls onDragStart with the drag dataTransfer", () => {
    const spy = mock((e: React.DragEvent) => {
      e.dataTransfer.setData("application/x-tillerd-panel-leaf", "leaf-1");
    });
    renderLeaf({ draggable: true, onDragStart: spy });
    const header = document.querySelector("[draggable='true']") as HTMLElement;
    expect(header).not.toBeNull();
    fireEvent.dragStart(header, { dataTransfer: stubDataTransfer() });
    expect(spy).toHaveBeenCalledTimes(1);
  });

  test("a non-draggable header renders draggable=false", () => {
    renderLeaf({ draggable: false });
    expect(document.querySelector("[draggable='true']")).toBeNull();
  });

  test("dragover on a drop target calls onDragOver so the caller can highlight it", () => {
    const spy = mock((e: React.DragEvent) => e.preventDefault());
    renderLeaf({ onDragOver: spy });
    const frame = document.querySelector("[data-panel-id='leaf-1']") as HTMLElement;
    fireEvent.dragOver(frame, {
      dataTransfer: stubDataTransfer({ "application/x-tillerd-panel-leaf": "leaf-2" }),
    });
    expect(spy).toHaveBeenCalledTimes(1);
  });

  test("isDropTarget renders the highlight ring and its e2e anchor", () => {
    renderLeaf({ isDropTarget: true });
    const frame = document.querySelector("[data-panel-id='leaf-1']");
    expect(frame?.className).toContain("ring-primary");
    expect(frame?.getAttribute("data-testid")).toBe("panel-drop-target-active");
  });

  test("dropping calls onDrop with the source leaf id readable from dataTransfer", () => {
    const spy = mock((e: React.DragEvent) => {
      expect(e.dataTransfer.getData("application/x-tillerd-panel-leaf")).toBe("leaf-2");
    });
    renderLeaf({ onDrop: spy });
    const frame = document.querySelector("[data-panel-id='leaf-1']") as HTMLElement;
    fireEvent.drop(frame, {
      dataTransfer: stubDataTransfer({ "application/x-tillerd-panel-leaf": "leaf-2" }),
    });
    expect(spy).toHaveBeenCalledTimes(1);
  });
});
