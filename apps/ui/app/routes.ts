import { type RouteConfig, index, route, layout } from "@react-router/dev/routes";

export default [
  layout("routes/_shell.tsx", [
    index("routes/_shell._index.tsx"),
    route("session/:id", "routes/_shell.session.$id.tsx"),
  ]),
] satisfies RouteConfig;
