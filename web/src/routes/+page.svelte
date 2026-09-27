<script lang="ts">
  import { marketCap, plainPct, price, signedPct } from "$lib/format";
  import type { RuleDoc, ScanReport, ScanRow, StoredAlert, StrikePart, Tone } from "$lib/types";

  const FILTERS = ["KQ", "MM", "ON", "DB", "SB4", "SBW", "SB9", "TML", "97C", "52W", "ER", "RS", "2A", "2B", "2C"];
  const TONE: Record<string, Tone> = {
    KQ: "gold", MM: "gold", "52W": "gold",
    ON: "cyan", DB: "cyan", SBW: "cyan", TML: "cyan",
    SB4: "orange", SB9: "orange",
    "97C": "purple",
    ER: "pink",
    RS: "green", "2A": "green", "2B": "green", "2C": "green"
  };

  let source = $state("fixtures");
  let preset = $state("qullamaggie");
  let required = $state<string[]>([]);
  let report = $state<ScanReport | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(true);
  let openSymbol = $state<string | null>(null);
  let drawer = $state<null | "rules" | "alerts" | "pine">(null);
  let rules = $state<RuleDoc[]>([]);
  let alerts = $state<StoredAlert[]>([]);
  let pineName = $state<"badges" | "strike">("badges");
  let pineText = $state("");
  let copyNote = $state("");
  let requestId = 0;

  const rows = $derived.by(() => {
    const all = report?.rows ?? [];
    return all.filter((row) =>
      required.every((code) => row.badges.some((badge) => badge.id === code && badge.passed))
    );
  });

  function toneClass(tone: string): string {
    return `t-${tone}`;
  }

  function partWidth(part: StrikePart): number {
    if (part.points < 0) {
      const cap = part.max > 0 ? part.max : 36;
      return Math.min(100, (Math.abs(part.points) / cap) * 100);
    }
    if (!part.max || part.max <= 0) return 0;
    return Math.min(100, Math.max(0, (part.points / part.max) * 100));
  }

  function partText(part: StrikePart): string {
    if (part.id === "chase") return part.points === 0 ? "0" : part.points.toFixed(0);
    return `${part.points.toFixed(0)}/${part.max.toFixed(0)}`;
  }

  function pivotText(row: ScanRow): string {
    const ext = row.metrics.pivot_extension_pct;
    const bars = row.metrics.bars_since_breakout;
    if (ext == null || bars == null) return "—";
    const day = bars === 1 ? "1 day" : `${bars} days`;
    return `${signedPct(ext)} · ${day}`;
  }

  function side(value: number): "up" | "down" | "flat" {
    if (value > 0.05) return "up";
    if (value < -0.05) return "down";
    return "flat";
  }

  function pills(row: ScanRow) {
    return row.badges
      .filter((badge) => badge.passed && badge.id !== "RS")
      .slice()
      .sort((a, b) => a.order - b.order);
  }

  function rsBadge(row: ScanRow) {
    return row.badges.find((badge) => badge.id === "RS");
  }

  async function load() {
    const id = ++requestId;
    const sourceNow = source;
    const presetNow = preset;
    loading = true;
    error = null;
    try {
      const response = await fetch(`/api/scan?source=${encodeURIComponent(sourceNow)}&preset=${encodeURIComponent(presetNow)}`);
      if (id !== requestId) return;
      if (!response.ok) {
        const body = await response.json().catch(() => ({}));
        throw new Error(body.error ?? `Scan failed (${response.status})`);
      }
      const body = (await response.json()) as ScanReport;
      if (body.orders_enabled) {
        throw new Error("Refusing a payload that claims orders are enabled.");
      }
      report = body;
    } catch (err) {
      if (id !== requestId) return;
      report = null;
      error = err instanceof Error ? err.message : "Scan failed";
    } finally {
      if (id === requestId) loading = false;
    }
  }

  async function copyPine() {
    const response = await fetch("/api/tv/pine/badges");
    const text = response.ok ? await response.text() : "";
    if (!text) {
      copyNote = "Could not load the Pine script.";
      return;
    }
    pineText = text;
    pineName = "badges";
    await copy(text, "Badge Pine copied");
  }

  async function openRules() {
    drawer = "rules";
    if (rules.length === 0) {
      const response = await fetch("/api/rules");
      rules = response.ok ? await response.json() : [];
    }
  }

  async function openAlerts() {
    drawer = "alerts";
    const response = await fetch("/api/tv/alerts");
    if (response.ok) {
      const body = await response.json();
      alerts = body.alerts ?? [];
    }
  }

  async function openPine(name: "badges" | "strike" = pineName) {
    pineName = name;
    drawer = "pine";
    const response = await fetch(`/api/tv/pine/${name}`);
    pineText = response.ok ? await response.text() : "Could not load the script.";
  }

  async function copy(text: string, note: string) {
    try {
      await navigator.clipboard.writeText(text);
      copyNote = note;
    } catch {
      copyNote = "Select the text and copy it manually.";
    }
  }

  async function sendSample() {
    const symbol = rows[0]?.symbol ?? "QMCO";
    const response = await fetch("/api/tv/webhook", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        symbol,
        badge: "KQ",
        strike: rows[0]?.strike ?? 80,
        price: rows[0]?.last ?? 0,
        time: new Date().toISOString(),
        message: "Sample alert from the desk. Not an order."
      })
    });
    if (!response.ok) {
      copyNote = "Sample alert was rejected.";
      return;
    }
    await openAlerts();
  }

  function toggle(code: string) {
    required = required.includes(code) ? required.filter((item) => item !== code) : [...required, code];
  }

  function toggleRow(symbol: string) {
    openSymbol = openSymbol === symbol ? null : symbol;
  }

  $effect(() => {
    source;
    preset;
    load();
  });
</script>

<main class="shell">
  <div class="topbar">
    <div class="brand">STRIKEDESK</div>
    <div class="seg" role="group" aria-label="Market data">
      <button type="button" aria-pressed={source === "fixtures"} onclick={() => (source = "fixtures")}>Fixtures</button>
      <button type="button" aria-pressed={source === "yahoo"} onclick={() => (source = "yahoo")}>Yahoo</button>
    </div>
    <select class="preset" aria-label="Screen preset" bind:value={preset}>
      {#each report?.presets ?? [{ id: "qullamaggie", name: "Qullamaggie", require: [], min_strike: 0 }] as item (item.id)}
        <option value={item.id}>{item.name}</option>
      {/each}
    </select>
    <div class="spacer"></div>
    <button class="ghost" type="button" onclick={openRules}>Rules</button>
    <button class="ghost" type="button" onclick={() => openPine("badges")}>Pine</button>
    <button class="ghost" type="button" onclick={openAlerts}>Alerts</button>
  </div>

  <section class="banner">
    <div class="count">{loading ? "…" : rows.length}</div>
    <div class="name">{report?.preset.name ?? "Qullamaggie"}</div>
  </section>

  <div class="filters" aria-label="Badge filters">
    {#each FILTERS as code (code)}
      <button
        class="chip {toneClass(TONE[code] ?? "green")}"
        type="button"
        aria-pressed={required.includes(code)}
        onclick={() => toggle(code)}
      >{code}</button>
    {/each}
  </div>

  <section class="board">
    {#if loading && !report}
      <p class="status">Loading the desk…</p>
    {:else if error}
      <p class="status">
        {error}. Start the API from the repo root with <code>cargo run -p strikedesk-api</code>.
      </p>
    {:else if rows.length === 0}
      <p class="status">No names carry every selected badge.</p>
    {:else}
      {#each rows as row (row.symbol)}
        <button class="row" type="button" aria-expanded={openSymbol === row.symbol} onclick={() => toggleRow(row.symbol)}>
          <div class="ident">
            <div class="line1">
              <span class="ticker {side(row.change_pct)}">{row.symbol}</span>
              <span class="chg {side(row.change_pct)}">{signedPct(row.change_pct)}</span>
            </div>
            <div class="line2">{marketCap(row.market_cap)} · {signedPct(row.month_pct)} · {plainPct(row.adr_pct)}</div>
          </div>
          <div class="pills">
            {#each pills(row) as badge (badge.id)}
              <span class="pill {toneClass(badge.tone)}">{badge.label}</span>
            {/each}
            {#if rsBadge(row)}
              <span class="rs {toneClass("green")} {rsBadge(row)?.passed ? "" : "cold"}" title="Relative strength">{rsBadge(row)?.label}</span>
            {/if}
            <span class="strike {toneClass(row.strike_tone)}" title="Strike zone">{row.strike}</span>
          </div>
        </button>
        {#if openSymbol === row.symbol}
          <div class="detail">
            <div class="metrics">
              <span>Last {price(row.last)}</span>
              <span>RS {row.rs}</span>
              <span>vs EMA10 {signedPct(row.metrics.dist_ema10_pct)}</span>
              <span>vs 252-high {signedPct(row.metrics.dist_year_high_pct)}</span>
              <span>Prior move {signedPct(row.metrics.prior_move_pct)}</span>
              <span>Stage advance {signedPct(row.metrics.stage_advance_pct)}</span>
              <span>vs pivot {pivotText(row)}</span>
            </div>
            <div class="actions">
              <a href={row.tv_url} target="_blank" rel="noreferrer">Open in TradingView</a>
              <button type="button" onclick={() => copy(row.symbol, `${row.symbol} copied`)}>Copy ticker</button>
              <button type="button" onclick={copyPine}>Copy Pine</button>
            </div>
            <div class="parts">
              {#each row.strike_parts as part (part.id)}
                <div class="part">
                  <span>{part.label}</span>
                  <div class="bar"><span class:dock={part.points < 0} style="width: {partWidth(part)}%"></span></div>
                  <span>{partText(part)}</span>
                </div>
              {/each}
            </div>
            <div class="checks">
              {#each row.checks as check (check.id)}
                <div class="check {check.passed ? "" : "fail"}">
                  <strong class={toneClass(check.tone)}>{check.label}</strong>
                  {check.detail}
                </div>
              {/each}
            </div>
          </div>
        {/if}
      {/each}
    {/if}
    <p class="notice">{report?.notice ?? "Badges choose what to chart. Strikedesk does not place broker orders."}</p>
    {#if report && report.errors.length > 0}
      <div class="errors">
        {#each report.errors as item (item.symbol)}
          <div>{item.symbol}: {item.message}</div>
        {/each}
      </div>
    {/if}
    {#if copyNote}<p class="notice">{copyNote}</p>{/if}
  </section>
</main>

{#if drawer}
  <div class="backdrop" role="presentation" onclick={(event) => { if (event.target === event.currentTarget) drawer = null; }}>
    <div class="drawer" role="dialog" aria-modal="true" aria-label={drawer}>
      {#if drawer === "rules"}
        <h2>Badge rules</h2>
        <p class="sub">These are the rules the engine runs. Approximations are marked. Nothing here is an order.</p>
        {#each rules as rule (rule.id)}
          <article class="rule">
            <h3><span class="pill {toneClass(rule.tone)}">{rule.label}</span> {rule.title}{#if rule.approximation}<span class="tag">approximation</span>{/if}</h3>
            <p>{rule.body}</p>
          </article>
        {/each}
      {:else if drawer === "alerts"}
        <h2>TradingView inbox</h2>
        <p class="sub">POST JSON to <code>/api/tv/webhook</code>. Alerts stay on this machine. The inbox never forwards them to a broker.</p>
        <div class="actions">
          <button class="primary" type="button" onclick={sendSample}>Send sample alert</button>
          <button type="button" onclick={openAlerts}>Refresh</button>
        </div>
        {#if alerts.length === 0}
          <p class="sub">No alerts stored in this process yet.</p>
        {:else}
          {#each alerts as alert (`${alert.received_at}-${alert.symbol}`)}
            <div class="alert-row">
              <strong>{alert.symbol}</strong>
              <span>{alert.badge ?? "—"} · strike {alert.strike ?? "—"} · {alert.price ?? "—"} · {alert.message ?? alert.received_at}</span>
            </div>
          {/each}
        {/if}
      {:else}
        <h2>Pine companion</h2>
        <p class="sub">Paste into TradingView’s Pine editor. The chart script is a companion, not the cross-sectional RS rank.</p>
        <div class="actions">
          <button type="button" aria-pressed={pineName === "badges"} onclick={() => openPine("badges")}>Badges</button>
          <button type="button" aria-pressed={pineName === "strike"} onclick={() => openPine("strike")}>Strike</button>
          <button class="primary" type="button" onclick={() => copy(pineText, "Pine copied")}>Copy Pine</button>
        </div>
        <textarea class="script" readonly spellcheck="false" value={pineText}></textarea>
      {/if}
      <div class="actions" style="margin-top: 12px">
        <button type="button" onclick={() => (drawer = null)}>Close</button>
      </div>
    </div>
  </div>
{/if}
