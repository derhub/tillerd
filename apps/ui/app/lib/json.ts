// Objects, arrays, null, and undefined collapse to `fallback`; a malformed payload
// cannot leak "[object Object]" through `String(someUnknown)`.
export function scalarString(value: unknown, fallback = ""): string {
  return typeof value === "string" || typeof value === "number" || typeof value === "boolean"
    ? String(value)
    : fallback;
}
