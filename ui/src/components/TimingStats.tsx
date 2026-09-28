import { Stack, Typography } from "@mui/material";
import type { PluginTimingSnapshot } from "../types";

type Props = {
  label?: string;
  durationUs?: number;
  averages?: PluginTimingSnapshot;
};

function formatDuration(us: number) {
  if (us >= 1000) {
    return `${(us / 1000).toFixed(2)} ms`;
  }
  return `${us} µs`;
}

function formatAverage(avgUs: number, count: number) {
  if (count === 0) {
    return "—";
  }
  return `${formatDuration(Math.round(avgUs))} (${count})`;
}

export default function TimingStats({ label, durationUs, averages }: Props) {
  return (
    <Stack spacing={0.5}>
      {label && (
        <Typography variant="subtitle2" color="text.secondary">
          {label}
        </Typography>
      )}
      {durationUs !== undefined && (
        <Typography variant="body2">
          This message: <strong>{formatDuration(durationUs)}</strong>
        </Typography>
      )}
      {averages && (
        <Typography variant="body2" color="text.secondary">
          Avg all: {formatAverage(averages.all.avg_us, averages.all.count)} • blocked:{" "}
          {formatAverage(averages.blocked.avg_us, averages.blocked.count)} • passed:{" "}
          {formatAverage(averages.passed.avg_us, averages.passed.count)}
        </Typography>
      )}
    </Stack>
  );
}
