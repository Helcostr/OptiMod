import { Link } from "@tanstack/solid-router";
import MessageTable from "../components/MessageTable";

const API_BASE = "http://localhost:3000";

export default function Messages() {
  return (
    <main class="shell">
      <section class="panel">
        <p class="eyebrow">Secure Twitch Chat Monitor</p>
        <Link to="/" class="primary-button">
          ← Back
        </Link>
      </section>

      <section class="panel">
        <h2>Message History</h2>
        <p class="lead">
          Browse stored chat messages from PostgreSQL. Click a column header to
          sort. Flagged rows are highlighted in red.
        </p>
        <MessageTable apiBase={API_BASE} limit={50} />
      </section>
    </main>
  );
}