<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import { fadeConfig, scaleConfig } from "../lib/transitions";
  import { onMount } from "svelte";
  import { t } from "../lib/i18n";
  import { focusFirstFocusable, trapFocus } from "../lib/focusTrap";
  import { APP_VERSION } from "../lib/constants";
  import Button from "./Button.svelte";

  interface Props {
    onclose: () => void;
  }

  let { onclose }: Props = $props();
  let modalEl: HTMLDivElement | undefined = $state();

  function closeWhenBackdropMatches(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      onclose();
    }
  }

  function stopMouseEventPropagation(event: MouseEvent) {
    event.stopPropagation();
  }

  function closeOnEscape(event: KeyboardEvent) {
    if (event.key === "Escape") {
      onclose();
      return;
    }
    trapFocus(event, modalEl);
  }

  onMount(() => {
    requestAnimationFrame(() => focusFirstFocusable(modalEl));
  });
</script>

<div class="backdrop" onmousedown={closeWhenBackdropMatches} role="presentation" transition:fade={fadeConfig}>
  <div class="modal" bind:this={modalEl} onmousedown={stopMouseEventPropagation} onkeydown={closeOnEscape} role="dialog" aria-modal="true" aria-labelledby="help-center-title" tabindex="-1" transition:scale={scaleConfig}>
    <div class="header">
      <div>
        <div class="eyebrow">{t("helpCenter.eyebrow")}</div>
        <h2 id="help-center-title">{t("helpCenter.title")}</h2>
      </div>
      <Button variant="ghost" size="icon" class="close-button" onclick={onclose} aria-label={t("common.close")}>×</Button>
    </div>

    <div class="body">
      <section class="section about-section">
        <div class="about-card">
          <div>
            <div class="card-title">{t("helpCenter.aboutTitle")}</div>
            <p>{t("helpCenter.aboutBody")}</p>
          </div>
          <div class="about-meta">
            <span class="about-version">OmniMon {APP_VERSION}</span>
            <span>{t("helpCenter.createdBy")}</span>
          </div>
          <Button
            href="https://github.com/chochy2001/omnimon"
            target="_blank"
            rel="noopener noreferrer"
            variant="secondary"
          >
            {t("helpCenter.moreInfo")}
          </Button>
        </div>
      </section>

      <section class="section">
        <h3>{t("helpCenter.aiTitle")}</h3>
        <div class="card-grid">
          <article class="card">
            <div class="card-title">{t("helpCenter.aiActionsTitle")}</div>
            <p>{t("helpCenter.aiActionsBody")}</p>
          </article>
          <article class="card">
            <div class="card-title">{t("helpCenter.aiConfigTitle")}</div>
            <p>{t("helpCenter.aiConfigBody")}</p>
          </article>
        </div>
      </section>

      <section class="section">
        <h3>{t("helpCenter.alertRulesTitle")}</h3>
        <p>{t("helpCenter.alertRulesBody")}</p>
      </section>

      <section class="section">
        <h3>{t("helpCenter.networkTitle")}</h3>
        <div class="bullet-list">
          <div><strong>{t("network.map")}</strong> - {t("helpCenter.networkMapBody")}</div>
          <div><strong>{t("network.connections")}</strong> - {t("helpCenter.networkConnectionsBody")}</div>
          <div><strong>{t("network.traffic")}</strong> - {t("helpCenter.networkTrafficBody")}</div>
        </div>
      </section>

      <section class="section">
        <h3>{t("helpCenter.profilesTitle")}</h3>
        <div class="bullet-list">
          <div><strong>{t("toolbar.general")}</strong> - {t("toolbar.generalDesc")}</div>
          <div><strong>{t("toolbar.developer")}</strong> - {t("toolbar.developerDesc")}</div>
          <div><strong>{t("toolbar.gaming")}</strong> - {t("toolbar.gamingDesc")}</div>
          <div><strong>{t("toolbar.batterySaver")}</strong> - {t("toolbar.batteryDesc")}</div>
        </div>
      </section>

      <section class="section">
        <h3>{t("helpCenter.controlsTitle")}</h3>
        <div class="bullet-list">
          <div><strong>{t("toolbar.securityReport")}</strong> - {t("helpCenter.securityReportBody")}</div>
          <div><strong>{t("toolbar.showDashboard")}</strong> - {t("helpCenter.dashboardBody")}</div>
          <div><strong>{t("toolbar.automations")}</strong> - {t("helpCenter.automationsBody")}</div>
          <div><strong>{t("toolbar.aiSettings")}</strong> - {t("helpCenter.settingsBody")}</div>
          <div><strong>{t("helpCenter.fontTitle")}</strong> - {t("helpCenter.fontBody")}</div>
        </div>
      </section>

      <section class="section faq">
        <h3>{t("helpCenter.faqTitle")}</h3>
        <div class="faq-item">
          <div class="faq-q">{t("helpCenter.faqQ1")}</div>
          <div class="faq-a">{t("helpCenter.faqA1")}</div>
        </div>
        <div class="faq-item">
          <div class="faq-q">{t("helpCenter.faqQ2")}</div>
          <div class="faq-a">{t("helpCenter.faqA2")}</div>
        </div>
        <div class="faq-item">
          <div class="faq-q">{t("helpCenter.faqQ3")}</div>
          <div class="faq-a">{t("helpCenter.faqA3")}</div>
        </div>
      </section>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.7);
  }

  .modal {
    width: min(920px, calc(100vw - 32px));
    max-height: calc(100vh - 48px);
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: 14px;
    background: linear-gradient(180deg, var(--bg-surface, var(--bg-alt)) 0%, var(--bg-alt) 100%);
    box-shadow: var(--shadow-lg, 0 12px 40px rgba(0,0,0,0.45));
  }

  .header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 18px 20px 12px;
    border-bottom: 1px solid var(--border);
  }

  .eyebrow {
    font-size: calc(var(--base-font-size, 12px) * 0.75);
    font-weight: 700;
    letter-spacing: 0.5px;
    text-transform: uppercase;
    color: var(--accent);
    margin-bottom: 4px;
  }

  h2, h3, p {
    margin: 0;
  }

  h2 {
    font-size: calc(var(--base-font-size, 12px) * 1.5);
  }

  h3 {
    font-size: calc(var(--base-font-size, 12px) * 1.05);
    color: var(--fg);
  }

  :global(.close-button) {
    flex-shrink: 0;
  }

  .about-section {
    margin-bottom: 4px;
  }

  .about-card {
    display: grid;
    gap: 14px;
    padding: 18px;
    border: 1px solid color-mix(in srgb, var(--accent) 22%, var(--border));
    border-radius: 16px;
    background: var(--bg-surface, var(--bg-alt));
  }

  .about-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 10px 16px;
    color: var(--fg-dim);
  }

  .about-version {
    color: var(--accent);
    font-weight: 800;
    letter-spacing: 0.04em;
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding: 18px 20px 22px;
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .card-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .card {
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--bg-alt);
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    line-height: 1.55;
  }

  .card-title,
  .faq-q {
    font-weight: 700;
    color: var(--fg);
  }

  .bullet-list,
  .faq {
    display: flex;
    flex-direction: column;
    gap: 8px;
    line-height: 1.55;
  }

  .faq-item {
    border-left: 2px solid var(--accent);
    padding-left: 10px;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .faq-a,
  p,
  .bullet-list div {
    color: var(--fg-dim);
  }

  @media (max-width: 760px) {
    .about-card {
      padding: 16px;
    }

    .card-grid {
      grid-template-columns: 1fr;
    }

    .header,
    .body {
      padding-left: 14px;
      padding-right: 14px;
    }
  }
</style>
