import { useState } from "react";
import type { Route } from "./+types/sessions";

export const meta: Route.MetaFunction = () => [
  { title: "Sessions | a-thing" },
  { name: "description", content: "Manage agent sessions" },
];

export default function Sessions() {
  const [sessions] = useState<Array<{ id: string; status: string }>>([]);

  return (
    <div style={{ padding: "2rem", maxWidth: "1200px", margin: "0 auto" }}>
      <h1>Sessions</h1>
      <p>Manage agent sessions</p>
      {sessions.length === 0 ? (
        <div
          style={{
            marginTop: "2rem",
            padding: "1rem",
            backgroundColor: "#f3f4f6",
            borderRadius: "0.5rem",
          }}
        >
          <p>No active sessions</p>
          <p style={{ fontSize: "0.875rem", color: "#666", marginTop: "0.5rem" }}>
            Start a new session from the server
          </p>
        </div>
      ) : (
        <div>
          {sessions.map((s) => (
            <div
              key={s.id}
              style={{ padding: "1rem", marginTop: "1rem", border: "1px solid #e5e7eb" }}
            >
              <p>
                {s.id} - {s.status}
              </p>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
