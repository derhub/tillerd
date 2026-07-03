// Minimal `when` context system. A command's availability is a conjunction of
// context-key terms: a bare key must be truthy, a `!key` term must be falsy. An
// absent or empty expression is always available. Deliberately not a full
// expression grammar (no ||, ==, parens) -- the term type is forward-compatible
// so a richer form can replace it without changing call sites.

export type ContextValue = boolean | string;

export type ContextSnapshot = Readonly<Record<string, ContextValue | undefined>>;

// A term is a context-key name, optionally negated with a leading `!`.
export type WhenTerm = string;

export type WhenExpr = readonly WhenTerm[];

function isTruthy(value: ContextValue | undefined): boolean {
  return typeof value === "string" ? value.length > 0 : value === true;
}

export function evaluateWhen(expr: WhenExpr | undefined, ctx: ContextSnapshot): boolean {
  if (!expr || expr.length === 0) return true;
  for (const term of expr) {
    const negated = term.startsWith("!");
    const key = negated ? term.slice(1) : term;
    const truthy = isTruthy(ctx[key]);
    if (negated ? truthy : !truthy) return false;
  }
  return true;
}
