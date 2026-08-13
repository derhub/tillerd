import type { MouseEventHandler, ReactNode, Ref } from "react";

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
/// <reference lib="dom" />
import { afterAll, afterEach, describe, expect, mock, test } from "bun:test";
import React from "react";

type Layout = Record<string, number>;
type GroupImperativeHandle = { setLayout: (layout: Layout) => void };
type GroupProps = {
  children?: ReactNode;
  defaultLayout?: Layout;
  groupRef?: Ref<GroupImperativeHandle | null>;
  onLayoutChanged?: (layout: Layout) => void;
};

let latestGroupProps: GroupProps | null = null;
let setLayoutCalls: Layout[] = [];
let latestHandleProps: {
  disableDoubleClick?: boolean;
  onDoubleClick?: MouseEventHandler<HTMLElement>;
} | null = null;

const ResizablePanelGroup = React.forwardRef<GroupImperativeHandle, GroupProps>(
  ({ children, ...props }, ref) => {
    latestGroupProps = props;
    const imperativeHandle = React.useMemo(
      () => ({
        setLayout: (layout: Layout) => setLayoutCalls.push(layout),
      }),
      [],
    );
    React.useImperativeHandle(ref, () => imperativeHandle, [imperativeHandle]);
    React.useImperativeHandle(props.groupRef, () => imperativeHandle, [imperativeHandle]);
    React.useEffect(() => {
      props.onLayoutChanged?.(props.defaultLayout ?? {});
    }, [props.defaultLayout, props.onLayoutChanged]);
    return <div data-testid="resizable-panel-group">{children}</div>;
  },
);

function ResizablePanel({ children }: { children?: ReactNode }) {
  return <div>{children}</div>;
}

function ResizableHandle(props: {
  disableDoubleClick?: boolean;
  onDoubleClick?: MouseEventHandler<HTMLElement>;
}) {
  latestHandleProps = props;
  return <button data-testid="resizable-handle" onDoubleClick={props.onDoubleClick} />;
}

void mock.module("~/components/ui/resizable", () => ({
  ResizablePanelGroup,
  ResizablePanel,
  ResizableHandle,
}));

const { PanelGroup } = await import("./PanelGroup");

afterEach(() => {
  cleanup();
  latestGroupProps = null;
  setLayoutCalls = [];
  latestHandleProps = null;
});
afterAll(() => mock.restore());

function renderSplit(sizes: number[], onSizesChange = mock(() => {})) {
  render(
    <PanelGroup.Provider
      id="group-1"
      displayMode="split"
      activeTabId={undefined}
      direction="horizontal"
      onSetActiveTab={() => {}}
    >
      <PanelGroup.Split
        childIds={["left-panel", "right-panel"]}
        sizes={sizes}
        onSizesChange={onSizesChange}
      >
        <PanelGroup.SplitItem panelId="left-panel" isLast={false}>
          left
        </PanelGroup.SplitItem>
        <PanelGroup.SplitItem panelId="right-panel" isLast>
          right
        </PanelGroup.SplitItem>
      </PanelGroup.Split>
    </PanelGroup.Provider>,
  );
  return onSizesChange;
}

describe("PanelGroup split geometry", () => {
  test("Stored sizes control rendering", () => {
    renderSplit([28, 72]);

    expect(latestGroupProps?.defaultLayout).toEqual({
      "panel:left-panel": 28,
      "panel:right-panel": 72,
    });
  });

  test("ignores the initial layout callback when it equals stored sizes", () => {
    const onSizesChange = renderSplit([28, 72]);

    expect(onSizesChange).not.toHaveBeenCalled();
  });

  test("Divider resize reports normalized sizes", () => {
    const onSizesChange = renderSplit([28, 72]);

    latestGroupProps?.onLayoutChanged?.({
      "panel:right-panel": 64,
      "panel:left-panel": 36,
    });

    expect(onSizesChange).toHaveBeenCalledTimes(1);
    expect(onSizesChange).toHaveBeenCalledWith([36, 64]);
  });

  test("Double-click resets", () => {
    const onSizesChange = renderSplit([28, 72]);

    fireEvent.doubleClick(screen.getByTestId("resizable-handle"));
    expect(latestHandleProps?.disableDoubleClick).toBe(true);

    expect(setLayoutCalls).toEqual([
      {
        "panel:left-panel": 50,
        "panel:right-panel": 50,
      },
    ]);
    expect(onSizesChange).toHaveBeenCalledTimes(1);
    expect(onSizesChange).toHaveBeenCalledWith([50, 50]);
  });
});
