import { createEffect, createSignal, onCleanup } from "solid-js";
import "./App.css";

type ChatMessage = {
  message_id: string;
  timestamp_ms: number;
  channel_id: string;
  channel_login: string;
  user_id: string;
  user_login: string;
  user_name: string;
  raw_message: string;
  normalized_message?: string | null;
  security_flags: string[];
};

function App() {
  const backendUrl = "http://localhost:3000/auth/login";
  const [status, setStatus] = createSignal("Checking backend status...");
  const [messages, setMessages] = createSignal<ChatMessage[]>([]);
  const [authenticated, setAuthenticated] = createSignal(false);

  createEffect(() => {
    let closed = false;
    let retries = 0;
    let source: EventSource | undefined;
    let reconnectTimer: number | undefined;

    const connect = () => {
      source = new EventSource("http://localhost:3000/events");

      source.onopen = () => {
        retries = 0;
        setStatus("Connected to SSE");
      };
      source.onerror = () => {
        if (closed) return;
        retries += 1;
        const delay = Math.min(1000 * retries, 5000);
        setStatus(`SSE disconnected; reconnecting in ${delay}ms`);
        source?.close();
        reconnectTimer = window.setTimeout(connect, delay);
      };
      source.addEventListener("chat-message", (event) => {
        const payload = JSON.parse((event as MessageEvent).data) as ChatMessage;
        setMessages((current) => [payload, ...current].slice(0, 25));
        setStatus(`Received ${messages().length + 1} chat events`);
      });
    };

    fetch("http://localhost:3000/api/status")
      .then((response) => response.json())
      .then((data) => {
        setAuthenticated(Boolean(data.token_loaded));
        setStatus(data.token_loaded ? "Authenticated" : "Not logged in yet");
      })
      .catch(() => setStatus("Backend unavailable"));

    connect();

    onCleanup(() => {
      closed = true;
      source?.close();
      if (reconnectTimer) window.clearTimeout(reconnectTimer);
    });
  });

  return (
    <main class="shell">
      <section class="panel">
        <p class="eyebrow">Secure Twitch Chat Monitor</p>
        <h1>Login with Twitch</h1>
        <p class="lead">
          Authorize the app in your browser, then the backend will start the
          EventSub chat listener.
        </p>
        <a class="primary-button" href={backendUrl}>
          Login with Twitch
        </a>
        <p class="status">Status: {status()}</p>
        <p class="status">
          Auth: {authenticated() ? "connected" : "not connected"}
        </p>
      </section>

      <section class="panel">
        <h2>Pipeline</h2>
        <ol>
          <li>OAuth authorization code flow</li>
          <li>In-memory token storage</li>
          <li>EventSub WebSocket subscription</li>
          <li>Security normalization and confusable detection</li>
          <li>SSE fan-out for UI/plugins</li>
        </ol>
      </section>

      <section class="panel chat-panel">
        <h2>Live Events</h2>
        <div class="event-list">
          {messages().length === 0 ? (
            <p class="empty">Waiting for chat messages...</p>
          ) : (
            messages().map((message) => (
              <article class="event-card">
                <header>
                  <strong>{message.user_name}</strong>
                  <span>@{message.user_login}</span>
                </header>
                <p>{message.normalized_message ?? message.raw_message}</p>
                <small>
                  {message.security_flags.join(", ") || "no security flags"}
                </small>
              </article>
            ))
          )}
        </div>
      </section>
    </main>
  );
}

export default App;
