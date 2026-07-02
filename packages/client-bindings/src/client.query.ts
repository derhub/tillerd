import {
  queryOptions,
  infiniteQueryOptions,
  mutationOptions,
  type QueryKey,
} from "@tanstack/react-query";

import { getQueryClient } from "./query-client";
import { whenReady, ensureResult } from "./readiness";
import { commands, events } from "./tauri_bindings.gen";

type Commands = typeof commands;
export type CommandKey = keyof Commands;

type Args<K extends CommandKey> =
  Parameters<Commands[K]> extends [infer A, ...unknown[]] ? A : void;

// Distributive (naked R): maps the {ok}|{error} union member-wise. A direct union test yields never.
type OkData<R> = R extends { status: "ok"; data: infer T } ? T : never;
type Result<K extends CommandKey> = OkData<Awaited<ReturnType<Commands[K]>>>;

// Raw __TAURI_INVOKE utilities unwrap to never; excluded so query()/command() cannot mis-route them.
type TypedKey = { [K in CommandKey]: [Result<K>] extends [never] ? never : K }[CommandKey];

function call<K extends CommandKey>(key: K, args?: Args<K>): Promise<Result<K>> {
  const run = commands[key] as (
    a?: unknown,
  ) => Promise<{ status: "ok"; data: unknown } | { status: "error"; error: unknown }>;
  return run(args).then(ensureResult) as Promise<Result<K>>;
}

// [prefix, cache key], longest prefix first (commandCenter before command, settings before setting).
const ENTITY: ReadonlyArray<readonly [string, string]> = [
  ["commandCenter", "commandCenter"],
  ["launchTemplate", "launchTemplates"],
  ["notifications", "notifications"],
  ["notification", "notifications"],
  ["orchestrator", "orchestrator"],
  ["serviceHealth", "serviceHealth"],
  ["keybinding", "keybindings"],
  ["workspace", "workspaces"],
  ["log", "logs"],
  ["settings", "settings"],
  ["setting", "settings"],
  ["template", "templates"],
  ["registry", "registry"],
  ["session", "sessions"],
  ["surface", "surfaces"],
  ["command", "commands"],
  ["profile", "profiles"],
  ["project", "projects"],
  ["config", "config"],
  ["daemon", "daemon"],
  ["window", "windows"],
  ["theme", "themes"],
];

const CROSS: Partial<Record<CommandKey, QueryKey[]>> = {
  projectArchive: [["projects"], ["sessions"]],
  projectDelete: [["projects"], ["sessions"]],
  projectMove: [["projects"], ["sessions"]],
};

const PARSED = new Map<CommandKey, readonly [string, string]>();
function parse(key: CommandKey): readonly [string, string] {
  const cached = PARSED.get(key);
  if (cached) return cached;
  const hit = ENTITY.find(([prefix]) => key.startsWith(prefix));
  if (!hit) {
    throw new Error(
      `client.query: unclassified command "${String(key)}" -- add a prefix to ENTITY`,
    );
  }
  const rest = key.slice(hit[0].length);
  const parsed = [hit[1], rest ? rest.charAt(0).toLowerCase() + rest.slice(1) : "self"] as const;
  PARSED.set(key, parsed);
  return parsed;
}

export function entityKey(key: CommandKey): string {
  return parse(key)[0];
}

// whenReady() yields false only on a ready -> not-ready transition, then a fresh pending promise
// replaces it; re-await until ready so the data type stays a clean Result<K>.
async function gated<K extends CommandKey>(key: K, args?: Args<K>): Promise<Result<K>> {
  while (!(await whenReady())) {
    /* await the next readiness promise */
  }
  return call(key, args);
}

function makeQuery<K extends TypedKey>(key: K, args?: Args<K>) {
  const [entity, verb] = parse(key);
  return queryOptions({
    queryKey: [entity, verb, args ?? null] as QueryKey,
    queryFn: () => gated(key, args),
  });
}

function makeInfinite<K extends TypedKey>(key: K, args?: Args<K>, pageSize = 50) {
  const [entity, verb] = parse(key);
  return infiniteQueryOptions({
    queryKey: [entity, verb, "infinite", args ?? null] as QueryKey,
    initialPageParam: 0,
    queryFn: ({ pageParam }) =>
      gated(key, { ...(args as object), limit: pageSize, offset: pageParam } as Args<K>),
    getNextPageParam: (last: unknown, pages: unknown[]) =>
      Array.isArray(last) && last.length === pageSize ? pages.flat().length : undefined,
  });
}

export const query = Object.assign(makeQuery, { infinite: makeInfinite });

type Snap<R> = Array<[QueryKey, R[] | undefined]>;
type Updater<K extends CommandKey, R> = (rows: R[], args: Args<K>) => R[];

async function applyOptimistic<R, A>(
  listKey: QueryKey,
  updater: (rows: R[], args: A) => R[],
  args: A,
): Promise<Snap<R>> {
  const qc = getQueryClient();
  await qc.cancelQueries({ queryKey: listKey });
  const snap = qc.getQueriesData<R[]>({ queryKey: listKey });
  for (const [k, rows] of snap) {
    if (rows) qc.setQueryData(k, updater(rows, args));
  }
  return snap;
}

function restore<R>(snap: Snap<R> | undefined): void {
  if (!snap) return;
  const qc = getQueryClient();
  for (const [k, rows] of snap) qc.setQueryData(k, rows);
}

export function command<K extends TypedKey, R = unknown>(
  key: K,
  opts?: { optimistic?: Updater<K, R> },
) {
  const invalidates = CROSS[key] ?? [[entityKey(key)]];
  const listKey: QueryKey = [entityKey(key)];
  const optimistic = opts?.optimistic;
  return mutationOptions<Result<K>, Error, Args<K>, Snap<R> | undefined>({
    mutationFn: (args) => call(key, args),
    meta: { invalidates },
    onMutate: optimistic ? (args) => applyOptimistic(listKey, optimistic, args) : undefined,
    onError: optimistic ? (_e, _args, ctx) => restore(ctx) : undefined,
  });
}

// Fire a command outside a React hook (bootstrap providers, callbacks). Ungated, like command()'s
// mutationFn: writes do not wait on readiness. The caller owns invalidation (fire-and-forget writes).
export function runCommand<K extends TypedKey>(key: K, args?: Args<K>): Promise<Result<K>> {
  return call(key, args);
}

// Maps ordered ids to one <entity>Reorder(id, sortOrder) per row. Not optimistic: lists are
// infinite-backed ({pages}, not a flat array), so it refetches rather than patch an unknown shape.
export function reorder<K extends TypedKey>(key: K) {
  const listKey: QueryKey = [entityKey(key)];
  return mutationOptions<void, Error, string[]>({
    mutationFn: async (orderedIds) => {
      await Promise.all(orderedIds.map((id, sortOrder) => call(key, { id, sortOrder } as Args<K>)));
    },
    meta: { invalidates: [listKey] },
  });
}

export function subscribe<K extends keyof typeof events>(key: K) {
  return events[key];
}
