import { createEffect, createSignal, onCleanup } from "solid-js";
import "./App.css";

type Badge = {
  set_id: string;
  id: string;
};

type ChatMessage = {
  message_id: string;
  timestamp_ms: number;
  channel_id: string;
  channel_login: string;
  user_id: string;
  user_login: string;
  user_name: string;
  badges: Badge[];
  color: string;
  raw_message: string;
  normalized_message?: string | null;
  security_flags: string[];
};

type StatusResponse = {
  authenticated: boolean;
  channel: string;
  token_loaded: boolean;
  pending_oauth_sessions: number;
  ws_running: boolean;
};

type ChannelResponse = {
  ok: boolean;
  channel: string;
  error: string | null;
};

type FlaggedMessagesResponse = {
  messages: ChatMessage[];
};

function formatTime(ts: number) {
  const d = new Date(ts);
  return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function getBadgeDisplay(badges: Badge[] | undefined) {
  if (!badges || badges.length === 0) return null;
  return badges.map((b) => `${b.set_id}:${b.id}`).join(" ");
}

const API_BASE = "http://localhost:3000";

function App() {
  const backendUrl = `${API_BASE}/auth/login`;
  const [status, setStatus] = createSignal("Checking backend status...");
  const [channel, setChannel] = createSignal("Not connected");
  const [messages, setMessages] = createSignal<ChatMessage[]>([]);
  const [authenticated, setAuthenticated] = createSignal(false);
  const [wsRunning, setWsRunning] = createSignal(false);
  const [channelInput, setChannelInput] = createSignal("");
  const [switching, setSwitching] = createSignal(false);
  const [switchError, setSwitchError] = createSignal<string | null>(null);
  const [flaggedMessages, setFlaggedMessages] = createSignal<ChatMessage[]>([]);
  const [loadingFlagged, setLoadingFlagged] = createSignal(false);

  createEffect(() => {
    let closed = false;
    let retries = 0;
    let source: EventSource | undefined;
    let reconnectTimer: number | undefined;

    const connect = () => {
      source = new EventSource(`${API_BASE}/events`);

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
        setMessages((current) => [payload, ...current].slice(0, 200));
        setStatus(`Received ${messages().length} chat events`);
      });
    };

    fetch(`${API_BASE}/api/status`)
      .then((response) => response.json())
      .then((data: StatusResponse) => {
        setAuthenticated(data.authenticated);
        setChannel(data.channel);
        setWsRunning(data.ws_running);
        setChannelInput(data.channel);
        setStatus(data.authenticated ? "Authenticated" : "Not logged in yet");
      })
      .catch(() => setStatus("Backend unavailable"));

    connect();

    onCleanup(() => {
      closed = true;
      source?.close();
      if (reconnectTimer) window.clearTimeout(reconnectTimer);
    });
  });

  const fetchFlaggedMessages = async () => {
    setLoadingFlagged(true);
    try {
      const res = await fetch(`${API_BASE}/api/flagged`);
      const data = (await res.json()) as FlaggedMessagesResponse;
      setFlaggedMessages(data.messages);
    } catch (err) {
      console.error("Failed to fetch flagged messages:", err);
    } finally {
      setLoadingFlagged(false);
    }
  };

  const switchChannel = async (e: Event) => {
    e.preventDefault();
    const target = channelInput().trim();
    if (!target) return;
    setSwitching(true);
    setSwitchError(null);
    try {
      const res = await fetch(`${API_BASE}/api/channel`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ channel: target }),
      });
      const data = (await res.json()) as ChannelResponse;
      if (data.ok) {
        setChannel(data.channel);
        setChannelInput(data.channel);
        setMessages([]);
        setStatus(`Switched to #${data.channel}; reconnecting listener`);
      } else {
        setSwitchError(data.error ?? "unknown error");
      }
    } catch (err) {
      setSwitchError(String(err));
    } finally {
      setSwitching(false);
    }
  };

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
          Channel: <strong class="channel-name">#{channel()}</strong>
        </p>
        <p class="status">
          Auth: {authenticated() ? "connected" : "not connected"} • WS:
          {wsRunning() ? " running" : " idle"}
        </p>
      </section>

      <section class="panel">
        <h2>Switch Channel</h2>
        <p class="lead">
          Point the listener at a different broadcaster. The old WebSocket is
          aborted and a fresh EventSub subscription is created for the new
          channel.
        </p>
        <form class="channel-form" onSubmit={switchChannel}>
          <input
            class="channel-input"
            type="text"
            placeholder="twitchpresents"
            value={channelInput()}
            disabled={switching() || !authenticated()}
            onInput={(e) => setChannelInput(e.currentTarget.value)}
          />
          <button
            class="primary-button"
            type="submit"
            disabled={switching() || !authenticated()}
          >
            {switching() ? "Switching..." : "Switch"}
          </button>
        </form>
        {switchError() && (
          <p class="status error">{switchError()}</p>
        )}
        {!authenticated() && (
          <p class="status">Log in with Twitch first to enable switching.</p>
        )}
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

      <section class="panel">
        <h2>Flagged Messages History</h2>
        <p class="lead">
                  View all messages with security flags for moderation and debugging.
                  Messages persist across server restarts via PostgreSQL.
                </p>
        <button
          class="primary-button"
          onClick={fetchFlaggedMessages}
          disabled={loadingFlagged()}
        >
          {loadingFlagged() ? "Loading..." : "Refresh Flagged Messages"}
        </button>
        <div class="event-list">
          {flaggedMessages().length === 0 ? (
            <p class="empty">No flagged messages found. Click above to load.</p>
          ) : (
            flaggedMessages().map((message) => (
              <article class="event-card flagged">
                <header>
                  <strong style={{ color: message.color || "#91f5b8" }}>{message.user_name}</strong>
                  <span>@{message.user_login}</span>
                  <span class="timestamp">{formatTime(message.timestamp_ms)}</span>
                </header>
                <p>{message.normalized_message ?? message.raw_message}</p>
                {getBadgeDisplay(message.badges) && (
                  <small class="badges">{getBadgeDisplay(message.badges)}</small>
                )}
                <small class="flags" style={{ color: "#ff6b6b" }}>
                  {message.security_flags.join(", ")}
                </small>
              </article>
            ))
          )}
        </div>
      </section>

      <section class="panel chat-panel">
        <h2>Live Events ({messages().length})</h2>
        <div class="event-list">
          {messages().length === 0 ? (
            <p class="empty">Waiting for chat messages...</p>
          ) : (
            messages().map((message) => (
              <article class="event-card">
                <header>
                  <strong style={{ color: message.color || "#91f5b8" }}>{message.user_name}</strong>
                  <span>@{message.user_login}</span>
                  <span class="timestamp">{formatTime(message.timestamp_ms)}</span>
                </header>
                <p>{message.normalized_message ?? message.raw_message}</p>
                {getBadgeDisplay(message.badges) && (
                  <small class="badges">{getBadgeDisplay(message.badges)}</small>
                )}
                <small class="flags">
                  {message.security_flags.length > 0
                    ? message.security_flags.join(", ")
                    : "no flags"}
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