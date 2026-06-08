const origin = import.meta.env["VITE_SERVER_ORIGIN"] ?? "localhost:3000";

export const API_BASE = `http://${origin}`;
export const WS_BASE = `ws://${origin}`;
