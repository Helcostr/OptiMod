import { useState } from "react";
import {
  Alert,
  Box,
  Button,
  Card,
  CardContent,
  Checkbox,
  FormControlLabel,
  Stack,
  TextField,
  Typography,
} from "@mui/material";
import PlayArrowIcon from "@mui/icons-material/PlayArrow";
import ActionChip from "../components/ActionChip";
import TimingStats from "../components/TimingStats";
import { fetchJson } from "../api";
import type { DebugMessageResponse } from "../types";

function formatDuration(us: number) {
  if (us >= 1000) {
    return `${(us / 1000).toFixed(2)} ms`;
  }
  return `${us} µs`;
}

export default function DebugPage() {
  const [message, setMessage] = useState("");
  const [userName, setUserName] = useState("debug_user");
  const [injectSse, setInjectSse] = useState(true);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<DebugMessageResponse | null>(null);

  const runDebug = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!message.trim()) return;

    setLoading(true);
    setError(null);
    try {
      const data = await fetchJson<DebugMessageResponse>("/api/debug/message", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          message,
          user_name: userName,
          inject_sse: injectSse,
        }),
      });
      setResult(data);
    } catch (err) {
      setError(String(err));
      setResult(null);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Stack spacing={3}>
      <Box>
        <Typography variant="overline" color="primary.light">
          Debug
        </Typography>
        <Typography variant="h4" sx={{ mt: 0.5 }}>
          Inject a synthetic chat message
        </Typography>
        <Typography color="text.secondary" sx={{ mt: 1 }}>
          Run the same security and checker pipeline used for Twitch chat. Optionally
          emit the result to the live SSE stream.
        </Typography>
      </Box>

      <Card>
        <CardContent>
          <Box component="form" onSubmit={runDebug}>
            <Stack spacing={2}>
              <TextField
                label="Message text"
                multiline
                minRows={4}
                value={message}
                onChange={(e) => setMessage(e.target.value)}
                placeholder="Paste a suspicious chat message here..."
                fullWidth
              />
              <TextField
                label="Display name"
                value={userName}
                onChange={(e) => setUserName(e.target.value)}
                fullWidth
              />
              <FormControlLabel
                control={
                  <Checkbox
                    checked={injectSse}
                    onChange={(e) => setInjectSse(e.target.checked)}
                  />
                }
                label="Emit to live SSE stream if not dropped"
              />
              <Button
                type="submit"
                variant="contained"
                startIcon={<PlayArrowIcon />}
                disabled={loading || !message.trim()}
                sx={{ alignSelf: "flex-start" }}
              >
                {loading ? "Running..." : "Run pipeline"}
              </Button>
            </Stack>
          </Box>
        </CardContent>
      </Card>

      {error && <Alert severity="error">{error}</Alert>}

      {result && (
        <Stack spacing={2}>
          <Alert severity={result.emitted_to_sse ? "success" : "warning"}>
            {result.emitted_to_sse
              ? "Message emitted to live SSE stream."
              : result.dropped
                ? "Message was dropped by OPTIMOD_DROP_BLOCKED."
                : "Pipeline ran without emitting to SSE."}
          </Alert>

          <Card>
            <CardContent>
              <Stack direction="row" spacing={1} alignItems="center" sx={{ mb: 2 }}>
                <Typography variant="h6">Pipeline verdict</Typography>
                <ActionChip action={result.verdict_action} />
              </Stack>
              <Typography color="text.secondary" sx={{ mb: 1 }}>
                Total pipeline: <strong>{formatDuration(result.total_duration_us)}</strong>
                {" • "}
                Checkers: <strong>{formatDuration(result.checker_duration_us)}</strong>
              </Typography>
              <Typography color="text.secondary">
                Event plugins run: {result.event_plugins_run.join(", ")}
              </Typography>
            </CardContent>
          </Card>

          <Card>
            <CardContent>
              <Stack direction="row" spacing={1} alignItems="center" sx={{ mb: 1 }}>
                <Typography variant="h6">{result.security.name}</Typography>
                <ActionChip action={result.security.action} />
              </Stack>
              <TimingStats durationUs={result.security.duration_us} />
              <Typography color="text.secondary" sx={{ mt: 1 }}>
                Flags: {result.security.flags.join(", ") || "none"}
              </Typography>
              {result.security.normalized_message && (
                <Typography sx={{ fontFamily: "IBM Plex Mono, monospace", mt: 1 }}>
                  Normalized: {result.security.normalized_message}
                </Typography>
              )}
            </CardContent>
          </Card>

          {result.checkers.map((checker) => (
            <Card key={checker.name}>
              <CardContent>
                <Stack direction="row" spacing={1} alignItems="center" sx={{ mb: 1 }}>
                  <Typography variant="h6">{checker.name}</Typography>
                  <ActionChip action={checker.action} />
                  <Typography variant="caption" color="text.secondary">
                    {checker.kind} • loaded
                  </Typography>
                </Stack>
                <TimingStats
                  durationUs={checker.duration_us}
                  averages={checker.averages}
                />
                {checker.detail ? (
                  <Box
                    component="pre"
                    sx={{
                      m: 0,
                      mt: 2,
                      p: 2,
                      bgcolor: "#10131b",
                      borderRadius: 2,
                      overflow: "auto",
                      fontFamily: "IBM Plex Mono, monospace",
                      fontSize: "0.85rem",
                    }}
                  >
                    {JSON.stringify(checker.detail, null, 2)}
                  </Box>
                ) : (
                  <Typography color="text.secondary" sx={{ mt: 2 }}>
                    No detailed output available for this checker.
                  </Typography>
                )}
              </CardContent>
            </Card>
          ))}

          <Card>
            <CardContent>
              <Typography variant="h6" sx={{ mb: 1 }}>
                Resulting message payload
              </Typography>
              <Box
                component="pre"
                sx={{
                  m: 0,
                  p: 2,
                  bgcolor: "#10131b",
                  borderRadius: 2,
                  overflow: "auto",
                  fontFamily: "IBM Plex Mono, monospace",
                  fontSize: "0.85rem",
                }}
              >
                {JSON.stringify(result.message, null, 2)}
              </Box>
            </CardContent>
          </Card>
        </Stack>
      )}
    </Stack>
  );
}
