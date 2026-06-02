import { useParams } from "react-router";
import { TerminalPane } from "~/components/TerminalPane";

export default function SessionPage() {
  const { id } = useParams();
  return <TerminalPane sessionId={id ?? null} />;
}
