import { useEffect, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Card,
  CardContent,
  Stack,
  TextField,
  Typography,
} from "@mui/material";
import LoginIcon from "@mui/icons-material/Login";
import SyncIcon from "@mui/icons-material/Sync";
import { fetchJson } from "../api";
import type { ChannelResponse, StatusResponse } from "../types";

export default function StatusPage() {
  const [status, setStatus] = useState<StatusResponse | null>(null);
  const [channelInput, setChannelInput] = useState("");
  const [switching, setSwitching] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadStatus = async () => {
    try {
      const data = await fetchJson<StatusResponse>("/api/status");
      setStatus(data);
      setChannelInput(data.channel);
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  };

  useEffect(() => {
    loadStatus();
    const timer = window.setInterval(loadStatus, 5000);
    return () => window.clearInterval(timer);
  }, []);

  const switchChannel = async (event: React.FormEvent) => {
    event.preventDefault();
    const target = channelInput.trim();
    if (!target) return;

    setSwitching(true);
    setError(null);
    try {
      const data = await fetchJson<ChannelResponse>("/api/channel", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ channel: target }),
      });
      if (!data.ok) {
        setError(data.error ?? "Failed to switch channel");
      } else {
        setChannelInput(data.channel);
        await loadStatus();
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setSwitching(false);
    }
  };

  return (
    <Stack spacing={3}>
      <Box>
        <Typography variant="overline" color="primary.light">
          Status
        </Typography>
        <Typography variant="h4" sx={{ mt: 0.5 }}>
          Login & monitoring
        </Typography>
        <Typography color="text.secondary" sx={{ mt: 1 }}>
          Authorize with Twitch, then monitor a channel through EventSub.
        </Typography>
      </Box>

      {error && <Alert severity="error">{error}</Alert>}

      <Stack direction={{ xs: "column", md: "row" }} spacing={3}>
          <Card sx={{ flex: 1 }}>
            <CardContent>
              <Typography variant="h6" sx={{ mb: 2 }}>
                Authentication
              </Typography>
              <Stack spacing={1.5}>
                <Typography>
                  Auth:{" "}
                  <strong>{status?.authenticated ? "connected" : "not connected"}</strong>
                </Typography>
                <Typography>
                  Token loaded: <strong>{status?.token_loaded ? "yes" : "no"}</strong>
                </Typography>
                <Typography>
                  WebSocket listener:{" "}
                  <strong>{status?.ws_running ? "running" : "idle"}</strong>
                </Typography>
                <Button
                  variant="contained"
                  startIcon={<LoginIcon />}
                  href="/auth/login"
                  sx={{ alignSelf: "flex-start", mt: 1 }}
                >
                  Login with Twitch
                </Button>
              </Stack>
            </CardContent>
          </Card>

          <Card sx={{ flex: 1 }}>
            <CardContent>
              <Typography variant="h6" sx={{ mb: 2 }}>
                Monitored channel
              </Typography>
              <Typography sx={{ mb: 2 }}>
                Current channel: <strong>#{status?.channel ?? "..."}</strong>
              </Typography>
              <Box component="form" onSubmit={switchChannel}>
                <Stack direction={{ xs: "column", sm: "row" }} spacing={1.5}>
                  <TextField
                    fullWidth
                    label="Channel login"
                    value={channelInput}
                    disabled={switching || !status?.authenticated}
                    onChange={(e) => setChannelInput(e.target.value)}
                  />
                  <Button
                    type="submit"
                    variant="outlined"
                    startIcon={<SyncIcon />}
                    disabled={switching || !status?.authenticated}
                    sx={{ minWidth: 140 }}
                  >
                    {switching ? "Switching..." : "Switch"}
                  </Button>
                </Stack>
              </Box>
              {!status?.authenticated && (
                <Typography variant="body2" color="text.secondary" sx={{ mt: 2 }}>
                  Log in with Twitch before switching channels.
                </Typography>
              )}
            </CardContent>
          </Card>
      </Stack>
    </Stack>
  );
}
