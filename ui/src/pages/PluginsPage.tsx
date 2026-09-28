import { useEffect, useState } from "react";
import {
  Alert,
  Box,
  Card,
  CardContent,
  Chip,
  Stack,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
  Typography,
} from "@mui/material";
import { fetchJson } from "../api";
import type { PluginsResponse } from "../types";

function formatAverage(avgUs: number, count: number) {
  if (count === 0) {
    return "—";
  }
  const rounded = Math.round(avgUs);
  const label = rounded >= 1000 ? `${(rounded / 1000).toFixed(2)} ms` : `${rounded} µs`;
  return `${label} (${count})`;
}

export default function PluginsPage() {
  const [plugins, setPlugins] = useState<PluginsResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchJson<PluginsResponse>("/api/plugins")
      .then(setPlugins)
      .catch((err) => setError(String(err)));
  }, []);

  return (
    <Stack spacing={3}>
      <Box>
        <Typography variant="overline" color="primary.light">
          Plugins
        </Typography>
        <Typography variant="h4" sx={{ mt: 0.5 }}>
          Active modules
        </Typography>
        <Typography color="text.secondary" sx={{ mt: 1 }}>
          Content checkers are loaded once at startup and stay in memory for the life
          of the process. Timing averages update as messages are processed.
        </Typography>
      </Box>

      {error && <Alert severity="error">{error}</Alert>}

      <Card>
        <CardContent>
          <Typography variant="h6" sx={{ mb: 2 }}>
            Content checkers
          </Typography>
          {!plugins?.checkers.length ? (
            <Typography color="text.secondary">No checkers loaded.</Typography>
          ) : (
            <Table size="small">
              <TableHead>
                <TableRow>
                  <TableCell>Name</TableCell>
                  <TableCell>Kind</TableCell>
                  <TableCell>Loaded</TableCell>
                  <TableCell>Avg (all)</TableCell>
                  <TableCell>Avg (blocked)</TableCell>
                  <TableCell>Avg (passed)</TableCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {plugins.checkers.map((checker) => (
                  <TableRow key={checker.name}>
                    <TableCell sx={{ fontFamily: "IBM Plex Mono, monospace" }}>
                      {checker.name}
                    </TableCell>
                    <TableCell>
                      <Chip size="small" label={checker.kind} variant="outlined" />
                    </TableCell>
                    <TableCell>
                      <Chip
                        size="small"
                        label={checker.loaded ? "yes" : "no"}
                        color={checker.loaded ? "success" : "default"}
                        variant="outlined"
                      />
                    </TableCell>
                    <TableCell>
                      {formatAverage(checker.timing.all.avg_us, checker.timing.all.count)}
                    </TableCell>
                    <TableCell>
                      {formatAverage(
                        checker.timing.blocked.avg_us,
                        checker.timing.blocked.count
                      )}
                    </TableCell>
                    <TableCell>
                      {formatAverage(
                        checker.timing.passed.avg_us,
                        checker.timing.passed.count
                      )}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardContent>
          <Typography variant="h6" sx={{ mb: 2 }}>
            Event plugins
          </Typography>
          {!plugins?.event_plugins.length ? (
            <Typography color="text.secondary">No event plugins loaded.</Typography>
          ) : (
            <Table size="small">
              <TableHead>
                <TableRow>
                  <TableCell>Name</TableCell>
                  <TableCell>Role</TableCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {plugins.event_plugins.map((plugin) => (
                  <TableRow key={plugin.name}>
                    <TableCell sx={{ fontFamily: "IBM Plex Mono, monospace" }}>
                      {plugin.name}
                    </TableCell>
                    <TableCell>
                      <Chip size="small" label="event" variant="outlined" />
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
    </Stack>
  );
}
