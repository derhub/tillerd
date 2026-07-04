// Best-effort renderer platform sniff for cosmetic branching only (native-control
// placement, key-hint glyphs) -- not a security or capability check.
export const isMac =
  typeof navigator !== "undefined" && /mac/i.test(navigator.platform || navigator.userAgent);
