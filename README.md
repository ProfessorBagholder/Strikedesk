# Strikedesk

Local-first Market-Wizard badge screener. A dark desk ranks symbols with colored pills and a 0–100 strike circle, in the spirit of a Qullamaggie-style screen. Badges decide **what to chart**. Strikedesk never places a broker order.

Steve Jacobs publishes screens in that visual language. The exact private thresholds behind those pills are not a public spec, so every rule below is the one this engine actually runs. Where a pill name is borrowed and the original cutoff is unpublished, the rule is marked as a Strikedesk approximation.

## Run locally

From the repo root, two processes:

```bash
cargo run -p strikedesk-api
```

```bash
cd web && npm install && npm run dev
```

Open http://127.0.0.1:5173. The API listens on http://127.0.0.1:8790. The dev server proxies `/api`.

Fixtures are the default and need no network. Switch the desk to **Yahoo** for cached daily bars of the same symbol list.

```bash
cargo test --workspace
cd web && npm run check && npm run build
```

After `npm run build`, `cargo run -p strikedesk-api` also serves the built UI at http://127.0.0.1:8790.

Optional environment variables are listed in `.env.example`. Do not commit a real `.env`.

## What it will not do

There is no order route, no broker client, and no auto-buy. `GET /api/health` returns `orders_enabled: false`. TradingView alerts posted to `/api/tv/webhook` are stored in memory and appended to `data/alerts.jsonl`. They are not forwarded anywhere.

## Layout

| Path | Role |
| --- | --- |
| `crates/strikedesk-core` | Pure badge engine. No files, no network, no HTTP. This is the crate to depend on from Bagholder. |
| `crates/strikedesk-data` | `MarketData` adapters: deterministic fixtures, plus Yahoo daily bars with a disk cache. |
| `crates/strikedesk-screener` | Config, presets, and the ranked scan. |
| `crates/strikedesk-api` | Axum process. Scan, rules, Pine text, webhook inbox. |
| `web` | SvelteKit desk. |
| `tradingview/` | Pine v5 companions. |
| `config/screener.toml` | Host, source, cache, presets. Omit `[badges]` to keep the defaults below. |
| `fixtures/universe.toml` | Demo symbols and synthetic recipes. Not live quotes. |

## Badge rules

Defaults live in `BadgeParams` and are echoed by `GET /api/rules` for whatever config is loaded. Fractions below are the defaults.

Moving averages are simple unless named EMA. ADR is the 20-session mean of `(high − low) / close`. A trading week in SBW is a block of 5 sessions aligned to the last bar, not a calendar week.

**KQ — Qullamaggie leader (approximation).** All of: close > SMA(50); SMA(10) > SMA(20); ADR(20) ≥ 3.5%; 20-session average dollar volume ≥ $5M; prior move ≥ 30%. Prior move is the last close divided by the lowest low in the 63 sessions that end 10 sessions ago. The close must also be within 5% of EMA(10) or EMA(20) and at or above EMA(20).

**MM — Minervini stack (approximation).** close > SMA(50) > SMA(150) > SMA(200), SMA(50) also above SMA(200), SMA(200) higher than 21 sessions ago, close ≥ 1.30 × the 252-session low, and close ≥ 0.75 × the 252-session high. RS is not hidden inside MM.

**ON — On the highs (approximation).** Close ≥ 98% of the highest high of the last 10 sessions, and the close is at least halfway from that day's low to its high.

**DB — Darvas-style dry box (approximation).** Last 10 sessions range ≤ 12% of the close, close is at least 70% of the way up that box, average volume of the last 5 sessions ≤ 0.85 × the 15 sessions before them, close > SMA(20).

**SB4 — 4-session thrust (approximation).** Close is up at least 4% versus 4 sessions ago, close > SMA(10), and at least one of those 4 sessions has volume ≥ 1.5 × the prior average or range ≥ 1.2 × the prior ADR.

**SBW — Weekly surf (approximation).** Close is 0–6% above the average of the last 10 weekly closes, that average is higher than the one ending 4 weeks earlier, and the close is no more than 15% below the highest weekly high of 26 weeks.

**SB9 — 9-session flag (approximation).** Last 9 sessions range ≤ 8% of the close, the close at the start of the flag is up at least 15% versus 15 sessions before the flag, the flag has not given back more than 2%, close > SMA(20).

**TML — Ten-day MA lift (approximation).** EMA(10) is higher than 5 sessions ago, EMA(10) > EMA(20), close is 0–3% above EMA(10), and the lowest low of the last 3 sessions is at or below EMA(10) × 1.01.

**97C — 97 close (approximation).** `(close − low) / (high − low) ≥ 0.97`. A zero-range bar passes only if it closes at the high. This is an intraday close, not “within 3% of the 52-week high.”

**52W — Recent 52-week high.** At least 100 sessions of history. The highest high of the last 252 sessions (or all of them, if shorter) printed fewer than 10 sessions ago. Ties use the most recent print.

**ER-n — Earnings-style reaction (approximation).** In the last 15 sessions that still have 20 sessions of history, the most recent bar that closes up, trades ≥ 2 × the prior 20-session average volume, and either gaps ≥ 4% or ranges ≥ 2 × the prior ADR. `n` is sessions ago (`ER-0` is the last bar). This is price and volume, not a confirmed earnings date.

**2A / 2B / 2C — Stage 2 phase (approximation).** Stage 2 means close > SMA(150) and SMA(150) is higher than 21 sessions ago. Advance = close / SMA(150) − 1. **2A** if advance < 15%, **2B** if advance < 40%, otherwise **2C**. Only one phase passes.

**RS — square, 1–99 (approximation).** Weighted return: 20% × 21 sessions, 40% × 63, 30% × 126, 10% × 252. Missing windows are dropped and the rest renormalized. With a benchmark loaded, each window is stock return minus benchmark return on the same dates; otherwise the raw stock return is used. The scan ranks those scores. RS = 1 + percentile × 98, ties share a mid-rank, and a one-name universe is 50. The square is always drawn. It is “hot” at ≥ 80, which is what the RS filter requires.

**Strike — circle, 0–100 (approximation).** Rounded sum of seven parts, clamped to 0–100:

| Part | Max | How |
| --- | --- | --- |
| Moving-average stack | 20 | 8 if close > SMA(50), 6 if SMA(50) > SMA(150), 6 if SMA(150) > a rising SMA(200) |
| Relative strength | 20 | RS / 99 × 20 |
| Location | 15 | Full within 8% of the 63-session high, 10 within 15%, 5 within 25%. Subtract 8 if the close is more than 12% above EMA(10) |
| Tightness | 15 | Last 10 sessions: 15 if range ≤ 8%, 10 if ≤ 12%, 5 if ≤ 18%. Capped at 8 when the 63-session return is under 15% |
| Close and volume | 10 | 5 for a ≥97% close (3 for ≥70%) plus 5 for volume ≥ 1.2 × the 20-session average (2 for ≥ 0.8 ×) |
| ADR band | 10 | 10 for 3.5–8%, 6 for 2.5–3.5% or 8–12%, 3 for 1.5–2.5% or 12–18% |
| Liquidity | 10 | 10 if 20-session average dollar volume ≥ $20M, 7 if ≥ $5M, 4 if ≥ $1M |

The circle is green at ≥ 70, gold at ≥ 50, and red below 50.

**Late chase — subtracted, then clamped (approximation).** The books buy the break of a base, or a tight retest of that pivot. They do not buy a close that has already run through the shelf. A base is 10 sessions whose high–low range is at most 15% of the high and whose close-to-close drift is at most 6%. The pivot is that shelf’s high. The active pivot is the most recent such shelf in the last 63 sessions whose next close clears it. A later thrust does not mint a new pivot unless those prior sessions are themselves a base.

Three docks, summed and capped at 36:

| Dock | When it is zero | Otherwise |
| --- | --- | --- |
| Distance | Close is within 5% of the pivot | 8 points per extra 10% above that band, cap 20 |
| Age | The break printed within the last 2 sessions, or the close is back inside 5% (a retest) | 3 points per session after those 2, cap 16 |
| ATR | Close is within 1 ATR(14) of EMA(10), or the break is still fresh: ≤ 2 sessions and within 8% of the pivot | 4 points per extra ATR above EMA(10), cap 12 |

If no shelf break is found, distance and age are skipped and an EMA(10) dock is used instead: 8 points per 10% beyond 5% above EMA(10), cap 16, still inside the 36-point cap. A QMCO-like path — shelf, then a multi-session run into an extended close with a long upper wick — takes this dock. A name that is still within 5% of a pivot that broke in the last 2 sessions does not.

## TradingView

- **Open in TradingView** on a row uses `https://www.tradingview.com/chart/?symbol=EXCHANGE:SYMBOL&interval=D`.
- **Copy Pine** copies `tradingview/strikedesk_badges.pine`. The strike study is `tradingview/strikedesk_strike.pine`. Both are served at `/api/tv/pine/badges` and `/api/tv/pine/strike`.
- Point a TradingView alert at `http://127.0.0.1:8790/api/tv/webhook` with a JSON message. If `STRIKEDESK_WEBHOOK_TOKEN` is set, send it as `X-Strikedesk-Token`. The Pine scripts already emit JSON. Keep this listener on localhost.

The Pine RS figure is a single-symbol excess-return proxy. The square on the desk is the cross-sectional percentile. Both files say so.

## Data

`fixtures` synthesizes every symbol in `fixtures/universe.toml` through a fixed calendar ending 2026-09-25. Use it for the demo and for tests.

`yahoo` calls the public chart endpoint (`/v8/finance/chart/{symbol}?interval=1d&range=2y`) and, when it can, the quote endpoint for market cap. Responses are cached under `data/cache` for `cache_ttl_hours` (default 12). A failed refresh falls back to a stale cache file. One bad symbol does not drop the rest of the scan.

## Config

`config/screener.toml` selects the bind address, the default source, the fixture file, the cache, the benchmark (`SPY`), and presets. Preset `require` is an AND filter on passing badges. `min_strike` drops lower scores before the desk sorts by strike, then RS, then symbol.

An empty `[universe].symbols` list scans every fixture except the benchmark. Yahoo uses that same list.

To change a threshold, add a `[badges]` table. Missing keys keep `BadgeParams` defaults. `cargo test -p strikedesk-screener` checks that the shipped file still matches those defaults when the table is absent.

## Tests

`cargo test -p strikedesk-core` covers KQ, MM, 97C, DB, SB4, SB9, SBW, TML, ON, 52W, ER-n, 2A versus 2C, RS ranking, and the strike range. The screener test scans the fixture universe and requires every badge family to appear. The API test checks health, the ranked scan, the webhook, and that `POST /api/orders` is not a route.
