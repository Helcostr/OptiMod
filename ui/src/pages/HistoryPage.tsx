import { useCallback, useEffect, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Card,
  CardContent,
  Chip,
  FormControlLabel,
  Stack,
  Switch,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
  Typography,
} from "@mui/material";
import RefreshIcon from "@mui/icons-material/Refresh";
import { fetchJson } from "../api";
import type { ChatMessage, MessageTimings } from "../types";

type MessagesResponse = {
  messages: ChatMessage[];
};

const PAGE_SIZE = 50;
const MESSAGE_PREVIEW_LEN = 100;

function formatTime(ts: number) {
  return new Date(ts).toLocaleString([], {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

function formatDuration(us: number) {
  if (us >= 1000) {
    return `${(us / 1000).toFixed(2)} ms`;
  }
  return `${us} µs`;
}

function truncateMessage(text: string) {
  if (text.length <= MESSAGE_PREVIEW_LEN) {
    return text;
  }
  return `${text.slice(0, MESSAGE_PREVIEW_LEN)}…`;
}

function formatTimings(timings: MessageTimings | null | undefined) {
  if (!timings) {
    return "—";
  }

  const checkerSummary = timings.checkers
    .map((checker) => `${checker.name}: ${formatDuration(checker.duration_us)}`)
    .join(" • ");

  return (
    <Stack spacing={0.5}>
      <Typography variant="body2">
        <strong>{formatDuration(timings.total_duration_us)}</strong> total
      </Typography>
      <Typography variant="caption" color="text.secondary">
        security {formatDuration(timings.security_duration_us)} • checkers{" "}
        {formatDuration(timings.checker_duration_us)}
      </Typography>
      {checkerSummary && (
        <Typography variant="caption" color="text.secondary">
          {checkerSummary}
        </Typography>
      )}
    </Stack>
  );
}

export default function HistoryPage() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [offset, setOffset] = useState(0);
  const [flaggedOnly, setFlaggedOnly] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(true);

  const loadPage = useCallback(
    async (pageOffset: number, replace: boolean) => {
      setLoading(true);
      setError(null);
      try {
        const params = new URLSearchParams({
          limit: String(PAGE_SIZE),
          offset: String(pageOffset),
          flagged_only: String(flaggedOnly),
        });
        const data = await fetchJson<MessagesResponse>(`/api/messages?${params}`);
        setHasMore(data.messages.length === PAGE_SIZE);
        setMessages((current) =>
          replace ? data.messages : [...current, ...data.messages]
        );
        setOffset(pageOffset);
      } catch (err) {
        setError(String(err));
      } finally {
        setLoading(false);
      }
    },
    [flaggedOnly]
  );

  useEffect(() => {
    loadPage(0, true);
  }, [loadPage]);

  const refresh = () => loadPage(0, true);
  const loadMore = () => loadPage(offset + PAGE_SIZE, false);

  return (
    <Stack spacing={3}>
      <Box>
        <Typography variant="overline" color="primary.light">
          History
        </Typography>
        <Typography variant="h4" sx={{ mt: 0.5 }}>
          Message history
        </Typography>
        <Typography color="text.secondary" sx={{ mt: 1 }}>
          All chat messages stored in PostgreSQL, including live Twitch events and
          debug injections. New messages include per-checker timings.
        </Typography>
      </Box>

      <Card>
        <CardContent>
          <Stack
            direction={{ xs: "column", sm: "row" }}
            spacing={2}
            alignItems={{ sm: "center" }}
            justifyContent="space-between"
          >
            <FormControlLabel
              control={
                <Switch
                  checked={flaggedOnly}
                  onChange={(e) => setFlaggedOnly(e.target.checked)}
                />
              }
              label="Flagged only"
            />
            <Button
              variant="outlined"
              startIcon={<RefreshIcon />}
              onClick={refresh}
              disabled={loading}
            >
              Refresh
            </Button>
          </Stack>
        </CardContent>
      </Card>

      {error && <Alert severity="error">{error}</Alert>}

      <Card>
        <CardContent sx={{ p: 0, "&:last-child": { pb: 0 } }}>
          {messages.length === 0 && !loading ? (
            <Box sx={{ p: 3 }}>
              <Typography color="text.secondary">No messages stored yet.</Typography>
            </Box>
          ) : (
            <Table size="small">
              <TableHead>
                <TableRow>
                  <TableCell>Time</TableCell>
                  <TableCell>User</TableCell>
                  <TableCell>Channel</TableCell>
                  <TableCell>Message</TableCell>
                  <TableCell>Timings</TableCell>
                  <TableCell>Flags</TableCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {messages.map((message) => {
                  const flagged = message.security_flags.length > 0;
                  const preview = truncateMessage(
                    message.normalized_message ?? message.raw_message
                  );
                  return (
                    <TableRow
                      key={message.message_id}
                      sx={{
                        bgcolor: flagged ? "rgba(255, 123, 100, 0.06)" : undefined,
                      }}
                    >
                      <TableCell sx={{ whiteSpace: "nowrap" }}>
                        {formatTime(message.timestamp_ms)}
                      </TableCell>
                      <TableCell>
                        <Typography
                          component="span"
                          sx={{ color: message.color || "text.primary", fontWeight: 600 }}
                        >
                          {message.user_name}
                        </Typography>
                        <Typography variant="caption" color="text.secondary" display="block">
                          @{message.user_login}
                        </Typography>
                      </TableCell>
                      <TableCell>#{message.channel_login}</TableCell>
                      <TableCell sx={{ maxWidth: 280 }}>
                        <Typography
                          sx={{
                            whiteSpace: "pre-wrap",
                            wordBreak: "break-word",
                          }}
                          title={message.normalized_message ?? message.raw_message}
                        >
                          {preview}
                        </Typography>
                      </TableCell>
                      <TableCell sx={{ minWidth: 180 }}>
                        {formatTimings(message.timings)}
                      </TableCell>
                      <TableCell>
                        <Stack direction="row" spacing={0.5} flexWrap="wrap" useFlexGap>
                          {message.security_flags.length === 0 ? (
                            <Chip size="small" label="none" variant="outlined" />
                          ) : (
                            message.security_flags.map((flag) => (
                              <Chip
                                key={flag}
                                size="small"
                                label={flag}
                                color="warning"
                                variant="outlined"
                              />
                            ))
                          )}
                        </Stack>
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {hasMore && messages.length > 0 && (
        <Button variant="outlined" onClick={loadMore} disabled={loading}>
          {loading ? "Loading..." : "Load more"}
        </Button>
      )}
    </Stack>
  );
}
