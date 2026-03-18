<script lang="ts">
  import { t } from "../lib/i18n";
  import { securityMap, severityColor } from "../stores/security";

  interface Props {
    pid: number;
  }

  let { pid }: Props = $props();

  let secInfo = $derived($securityMap.get(pid));

  let threatCount = $derived(secInfo?.threats.length ?? 0);
  let cveCount = $derived(secInfo?.cves.length ?? 0);

  let topSeverity = $derived.by(() => {
    if (!secInfo || secInfo.cves.length === 0) return null;
    const rank = (s: string | null) =>
      s === "critical" ? 4 : s === "high" ? 3 : s === "medium" ? 2 : 1;
    return secInfo.cves.reduce(
      (best, c) => (rank(c.severity) > rank(best) ? c.severity : best),
      secInfo.cves[0].severity,
    );
  });

  let threatTooltip = $derived(
    secInfo?.threats
      .map(
        (t) =>
          `${t.indicator} — ${t.mitre_techniques.map((m) => `${m.technique_id} ${m.name}`).join(", ")} (${(t.confidence * 100).toFixed(0)}%)`,
      )
      .join("\n") ?? "",
  );

  let cveTooltip = $derived(
    secInfo?.cves
      .map((c) => `${c.cve_id} [${c.severity ?? "?"}] ${c.summary ?? ""}`)
      .join("\n") ?? "",
  );
</script>

{#if secInfo}
  {#if threatCount > 0}
    <span
      class="sec-badge threat"
      title={threatTooltip}
      aria-label={t(threatCount > 1 ? "securityBadge.mitrePlural" : "securityBadge.mitreSingular", { count: threatCount })}
    >
      <svg class="sec-icon" viewBox="0 0 16 16" width="10" height="10" fill="currentColor">
        <path d="M8 1L1 14h14L8 1zm0 4v4m0 2v.01"/>
      </svg>
      MITRE:{threatCount}
    </span>
  {/if}
  {#if cveCount > 0}
    <span
      class="sec-badge cve"
      style="color: {severityColor(topSeverity)}; background: color-mix(in srgb, {severityColor(topSeverity)} 28%, var(--bg))"
      title={cveTooltip}
      aria-label={t(cveCount > 1 ? "securityBadge.cvePlural" : "securityBadge.cveSingular", { count: cveCount })}
    >
      <svg class="sec-icon" viewBox="0 0 16 16" width="10" height="10" fill="currentColor">
        <path d="M8 0a8 8 0 100 16A8 8 0 008 0zm0 3v6m0 2v1"/>
      </svg>
      CVE:{cveCount}
    </span>
  {/if}
{/if}

<style>
  .sec-badge {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 0 4px;
    border-radius: 3px;
    font-size: calc(var(--base-font-size, 12px) * 0.667);
    font-weight: 700;
    margin-left: 4px;
    vertical-align: middle;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    cursor: help;
    line-height: 1.6;
  }

  .sec-badge.threat {
    background: color-mix(in srgb, var(--danger) 28%, var(--bg));
    color: var(--danger);
  }

  .sec-icon {
    flex-shrink: 0;
    color: var(--danger);
  }
</style>
