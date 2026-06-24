// Only place inline dynamic import() is allowed -- ast-grep rule `no-inline-dynamic-import` forbids it inside hooks/components. Xterm and the diff renderer are heavy and load on demand; the web build has no Tauri.

export const lazyXterm = () => import("@xterm/xterm");
export const lazyFitAddon = () => import("@xterm/addon-fit");
export const lazyDiffs = () => import("@pierre/diffs");
export const lazyDiffsReact = () => import("@pierre/diffs/react");
