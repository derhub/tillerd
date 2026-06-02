import { createContext } from "react";

export type SessionContextValue = {
  sessionId: string | null;
  status: string;
  setStatus: (s: string) => void;
};

export const SessionContext = createContext<SessionContextValue>({
  sessionId: null,
  status: "",
  setStatus: () => {},
});
