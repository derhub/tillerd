import React from "react";
import ReactDOM from "react-dom/client";

import { THEME_PAINT_SCRIPT } from "~/lib/settings/theme";
import { AppRouter } from "~/router";

import "./app.css";

// Apply the cached theme before first paint; avoids flash on load.
const s = document.createElement("script");
s.textContent = THEME_PAINT_SCRIPT;
document.head.appendChild(s);

const root = document.getElementById("root");
if (!root) throw new Error("Missing #root element");

ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <AppRouter />
  </React.StrictMode>,
);
