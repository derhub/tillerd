import { useEffect, useState } from "react";
import type { Route } from "./+types/_index";

export const meta: Route.MetaFunction = () => [
  { title: "Dashboard | a-thing" },
  { name: "description", content: "SDK agent dashboard" },
];

interface Status {
  server: string;
  sessions: number;
}

export default function Dashboard() {
  const [status, setStatus] = useState<Status | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchStatus = async () => {
      try {
        const res = await fetch("http://localhost:3000/api/status");
        if (!res.ok) throw new Error("Failed to fetch status");
        const data = await res.json();
        setStatus(data);
        setError(null);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Unknown error");
        setStatus(null);
      } finally {
        setLoading(false);
      }
    };

    fetchStatus();
    const interval = setInterval(fetchStatus, 5000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div style={{ padding: "2rem", maxWidth: "1200px", margin: "0 auto" }}>
      <h1>Dashboard</h1>
      <p>SDK agent dashboard</p>
      <div
        style={{
          marginTop: "2rem",
          padding: "1rem",
          backgroundColor: "#f3f4f6",
          borderRadius: "0.5rem",
        }}
      >
        <h2>Status</h2>
        {loading && <p>Loading...</p>}
        {error && <p style={{ color: "red" }}>Error: {error}</p>}
        {status && (
          <div>
            <p>
              Server: <strong>{status.server}</strong>
            </p>
            <p>
              Sessions: <strong>{status.sessions}</strong>
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
