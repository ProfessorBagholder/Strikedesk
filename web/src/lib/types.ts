export type Tone = "gold" | "cyan" | "purple" | "orange" | "green" | "pink" | "red";

export interface Badge {
  id: string;
  label: string;
  tone: Tone;
  passed: boolean;
  detail: string;
  order: number;
}

export interface Check {
  id: string;
  label: string;
  tone: Tone;
  passed: boolean;
  detail: string;
}

export interface StrikePart {
  id: string;
  label: string;
  points: number;
  max: number;
}

export interface Metrics {
  dist_ema10_pct: number | null;
  dist_year_high_pct: number | null;
  prior_move_pct: number | null;
  stage_advance_pct: number | null;
  sma10: number | null;
  sma20: number | null;
  sma50: number | null;
  sma150: number | null;
  sma200: number | null;
  ema10: number | null;
  ema20: number | null;
}

export interface ScanRow {
  symbol: string;
  name: string | null;
  exchange: string | null;
  market_cap: number | null;
  last: number;
  change_pct: number;
  month_pct: number | null;
  adr_pct: number | null;
  avg_dollar_volume: number | null;
  rs: number;
  strike: number;
  strike_tone: Tone;
  as_of: string;
  tv_url: string;
  badges: Badge[];
  checks: Check[];
  strike_parts: StrikePart[];
  metrics: Metrics;
}

export interface Preset {
  id: string;
  name: string;
  require: string[];
  min_strike: number;
}

export interface ScanReport {
  desk: string;
  orders_enabled: boolean;
  source: string;
  benchmark: string;
  as_of: string | null;
  preset: Preset;
  presets: Preset[];
  count: number;
  notice: string;
  rows: ScanRow[];
  errors: { symbol: string; message: string }[];
}

export interface RuleDoc {
  id: string;
  label: string;
  tone: Tone;
  title: string;
  summary: string;
  body: string;
  approximation: boolean;
}

export interface StoredAlert {
  received_at: string;
  symbol: string;
  badge: string | null;
  strike: number | null;
  price: number | null;
  time: string | null;
  message: string | null;
}
