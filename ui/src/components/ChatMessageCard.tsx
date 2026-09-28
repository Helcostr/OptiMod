import { Box, Card, CardContent, Chip, Stack, Typography } from "@mui/material";
import type { ChatMessage } from "../types";

type Props = {
  message: ChatMessage;
};

function formatTime(ts: number) {
  return new Date(ts).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

export default function ChatMessageCard({ message }: Props) {
  const flagged = message.security_flags.length > 0;

  return (
    <Card
      variant="outlined"
      sx={{
        bgcolor: flagged ? "rgba(255, 123, 100, 0.08)" : "background.paper",
        borderColor: flagged ? "error.main" : "divider",
      }}
    >
      <CardContent>
        <Stack direction="row" justifyContent="space-between" spacing={2} sx={{ mb: 1 }}>
          <Typography sx={{ color: message.color || "success.main", fontWeight: 700 }}>
            {message.user_name}
          </Typography>
          <Typography variant="caption" color="text.secondary">
            {formatTime(message.timestamp_ms)}
          </Typography>
        </Stack>
        <Typography sx={{ mb: 1.5, whiteSpace: "pre-wrap" }}>
          {message.normalized_message ?? message.raw_message}
        </Typography>
        <Stack direction="row" spacing={1} flexWrap="wrap" useFlexGap>
          {message.security_flags.length === 0 ? (
            <Chip size="small" label="no flags" variant="outlined" />
          ) : (
            message.security_flags.map((flag) => (
              <Chip key={flag} size="small" label={flag} color="warning" variant="outlined" />
            ))
          )}
        </Stack>
        <Box sx={{ mt: 1 }}>
          <Typography variant="caption" color="text.secondary">
            #{message.channel_login} • {message.message_id}
          </Typography>
        </Box>
      </CardContent>
    </Card>
  );
}
