export type Badge = {
  set_id: string;
  id: string;
};

export type CheckerTiming = {
  name: string;
  duration_us: number;
  action: string;
};

export type MessageTimings = {
  total_duration_us: number;
  security_duration_us: number;
  checker_duration_us: number;
  checkers: CheckerTiming[];
};

export type ChatMessage = {
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
  timings?: MessageTimings | null;
};

export type StatusResponse = {
  authenticated: boolean;
  channel: string;
  token_loaded: boolean;
  pending_oauth_sessions: number;
  ws_running: boolean;
};

export type ChannelResponse = {
  ok: boolean;
  channel: string;
  error: string | null;
};

export type TimingSnapshot = {
  count: number;
  avg_us: number;
};

export type PluginTimingSnapshot = {
  all: TimingSnapshot;
  blocked: TimingSnapshot;
  passed: TimingSnapshot;
};

export type CheckerPluginInfo = {
  name: string;
  kind: string;
  loaded: boolean;
  timing: PluginTimingSnapshot;
};

export type PluginInfo = {
  name: string;
};

export type PluginsResponse = {
  checkers: CheckerPluginInfo[];
  event_plugins: PluginInfo[];
};

export type CheckerStepDetail = {
  name: string;
  kind: string;
  action: string;
  duration_us: number;
  averages: PluginTimingSnapshot;
  detail: Record<string, unknown> | null;
};

export type SecurityStepDetail = {
  name: string;
  action: string;
  duration_us: number;
  flags: string[];
  normalized_message: string | null;
};

export type DebugMessageResponse = {
  emitted_to_sse: boolean;
  dropped: boolean;
  verdict_action: string;
  total_duration_us: number;
  checker_duration_us: number;
  security: SecurityStepDetail;
  checkers: CheckerStepDetail[];
  event_plugins_run: string[];
  message: ChatMessage;
};
