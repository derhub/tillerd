import React from "react";

export type SessionContextValue = {
  sessionId: string | null;
  status: string;
  setStatus: (s: string) => void;
};

export const SessionContext = React.createContext<SessionContextValue>({
  sessionId: null,
  status: "",
  setStatus: () => {},
});
