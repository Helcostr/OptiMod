import { useEffect, useState } from "react";
import {
  Alert,
  Box,
  Chip,
  Stack,
  Typography,
} from "@mui/material";
import ChatMessageCard from "../components/ChatMessageCard";
import type { ChatMessage } from "../types";

export default function LiveEventsPage() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [status, setStatus] = useState("Connecting to SSE...");

  useEffect(() => {
    let closed = false;
    let retries = 0;
    let source: EventSource | undefined;
    let reconnectTimer: number | undefined;

    const connect = () => {
      source = new EventSource("/events");

      source.onopen = () => {
        retries = 0;
        setStatus("Connected to live events");
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
        setStatus("Receiving live chat events");
      });
    };

    connect();

    return () => {
      closed = true;
      source?.close();
      if (reconnectTimer) window.clearTimeout(reconnectTimer);
    };
  }, []);

  return (
    <Stack spacing={3}>
      <Box>
        <Typography variant="overline" color="primary.light">
          Live Events
        </Typography>
        <Typography variant="h4" sx={{ mt: 0.5 }}>
          SSE chat stream
        </Typography>
        <Stack direction="row" spacing={1} alignItems="center" sx={{ mt: 1 }}>
          <Typography color="text.secondary">{status}</Typography>
          <Chip size="small" label={`${messages.length} events`} variant="outlined" />
        </Stack>
      </Box>

      {messages.length === 0 ? (
        <Alert severity="info">Waiting for chat messages...</Alert>
      ) : (
        <Stack spacing={2}>
          {messages.map((message) => (
            <ChatMessageCard key={message.message_id} message={message} />
          ))}
        </Stack>
      )}
    </Stack>
  );
}
