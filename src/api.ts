import { invoke } from "@tauri-apps/api/core";

export type Layout = {
  hasNotch: boolean;
  notchWidth: number;
  collapsedWidth: number;
  collapsedHeight: number;
  expandedWidth: number;
  expandedHeight: number;
  platform: string;
};

export type MediaState = {
  available: boolean;
  playing: boolean;
  title: string;
  artist: string;
  album: string;
  player: string;
  art: string | null;
  position: number | null;
  duration: number | null;
};

export type Shortcut = { id: string; name: string; icon: string; kind: string; target: string };
export type ShelfItem = { path: string; name: string; addedAt: number; isDir: boolean; exists: boolean };
export type Data = { shortcuts: Shortcut[]; shelf: ShelfItem[] };

export type Totals = { input: number; output: number; cacheRead: number; cacheWrite: number; total: number };
export type ProviderReport = {
  id: string;
  name: string;
  found: boolean;
  dirs: string[];
  today: Totals;
  week: Totals;
  days: { date: string; total: number }[];
  lastActivity: string | null;
};
export type Report = { providers: ProviderReport[]; generatedAt: string };
export type AgentSource = "claude" | "codex";

export const api = {
  layout: () => invoke<Layout>("get_layout"),
  setExpanded: (expanded: boolean) => invoke<void>("set_expanded", { expanded }),
  media: () => invoke<MediaState>("media_state"),
  mediaControl: (action: "play_pause" | "next" | "previous") => invoke<void>("media_control", { action }),
  aiUsage: () => invoke<Report>("ai_usage"),
  data: () => invoke<Data>("get_data"),
  saveShortcuts: (shortcuts: Shortcut[]) => invoke<Data>("save_shortcuts", { shortcuts }),
  launch: (id: string) => invoke<void>("launch_shortcut", { id }),
  shelfAdd: (paths: string[]) => invoke<Data>("shelf_add", { paths }),
  shelfRemove: (path: string | null) => invoke<Data>("shelf_remove", { path }),
  openPath: (path: string, reveal: boolean) => invoke<void>("open_path", { path, reveal }),
  dragIcon: () => invoke<string | null>("drag_icon"),
  notificationsInit: () => invoke<boolean>("notifications_init"),
  setNotificationsEnabled: (on: boolean) => invoke<void>("set_notifications_enabled", { on }),
  agentHooksStatus: () => invoke<Record<AgentSource, boolean>>("agent_hooks_status"),
  setAgentHooks: (source: AgentSource, on: boolean) => invoke<void>("set_agent_hooks", { source, on }),
};
