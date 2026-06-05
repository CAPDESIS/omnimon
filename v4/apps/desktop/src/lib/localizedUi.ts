import { t } from "./i18n";

import type { DynamicAlert, NetworkAlert, ToolResult } from "./types";

export function formatLocalizedLabel(value: string): string {
  const translated = t(value);
  return translated === value ? value : translated;
}

function parseProcessUptime(uptime: string): { value: string; unit: string } | null {
  const match = uptime.trim().match(/^(\d+)([smhd])$/i);
  if (!match) return null;
  return { value: match[1], unit: match[2].toLowerCase() };
}

export function formatProcessState(state: string | null | undefined): string {
  if (!state) return "—";
  switch (state.toUpperCase()) {
    case "R":
      return t("processStates.running");
    case "S":
      return t("processStates.idle");
    default:
      return state;
  }
}

export function formatProcessUptime(uptime: string | null | undefined): string {
  if (!uptime) return "—";
  const parsed = parseProcessUptime(uptime);
  if (!parsed) return uptime;
  const unitKey = {
    s: "commonUnits.secondsShort",
    m: "commonUnits.minutesShort",
    h: "commonUnits.hoursShort",
    d: "commonUnits.daysShort",
  }[parsed.unit];
  return unitKey ? `${parsed.value}${t(unitKey)}` : uptime;
}

export function formatCaptureBackendLabel(backend: string | null | undefined): string {
  if (!backend) return t("network.captureBackendUnknown");
  const normalized = backend.toLowerCase();
  if (normalized === "ebpf") return t("network.captureBackends.ebpf");
  if (normalized.includes("libpcap") || normalized.includes("packet filter") || normalized === "pcap") {
    return t("network.captureBackends.libpcap");
  }
  if (normalized === "windivert") return t("network.captureBackends.windivert");
  if (normalized === "unsupported") return t("network.captureBackends.unsupported");
  if (normalized === "unknown") return t("network.captureBackends.unknown");
  return backend;
}

export function formatPluginStatus(status: string, enabled: boolean): string {
  if (!enabled) return t("plugins.disabled");
  switch (status) {
    case "ok":
      return t("plugins.status.ok");
    case "error":
      return t("plugins.status.error");
    case "idle":
      return t("plugins.status.idle");
    case "disabled":
      return t("plugins.disabled");
    default:
      return status;
  }
}

export function formatPluginMetricKind(kind: string): string {
  switch (kind) {
    case "gauge":
      return t("plugins.metricKinds.gauge");
    case "counter":
      return t("plugins.metricKinds.counter");
    default:
      return kind;
  }
}

export function formatProtocolLabel(protocol: string): string {
  switch (protocol.toUpperCase()) {
    case "TCP":
      return t("networkConnections.protocols.tcp");
    case "UDP":
      return t("networkConnections.protocols.udp");
    default:
      return protocol;
  }
}

export function formatDirectionLabel(direction: string): string {
  switch (direction.toLowerCase()) {
    case "inbound":
      return t("network.inbound");
    case "outbound":
      return t("network.outbound");
    default:
      return direction;
  }
}

export function formatConnectionState(state: string): string {
  const key = `connectionStates.${state.toLowerCase()}`;
  const translated = t(key);
  return translated === key ? state : translated;
}

export function localizeBackendError(message: string): string {
  const trimmed = message.trim();
  const invalidPidMatch = trimmed.match(/^(Invalid PID|PID inválido):\s*(.+)$/i);
  const toolInvalidPidMatch = trimmed.match(/^tool_invalid_pid:(.+)$/i);
  const toolProcessNotFoundMatch = trimmed.match(/^tool_process_not_found:(.+)$/i);
  const toolNoProcessesMatchedMatch = trimmed.match(/^tool_no_processes_matched:(.+)$/i);
  const toolUnknownMatch = trimmed.match(/^tool_unknown:(.+)$/i);

  if (/rate limited/i.test(trimmed)) return t("common.rateLimited");
  if (/^error_no_api_key:/i.test(trimmed) || /No API key found/i.test(trimmed)) return t("settings.apiKeyMissing");
  if (trimmed === "error_api_key_empty" || /API key cannot be empty/i.test(trimmed)) return t("settings.apiKeyEmpty");
  if (/Failed to save API key to OS keyring/i.test(trimmed)) return t("settings.apiKeyKeyringError");
  if (/payload exceeds/i.test(trimmed)) return t("errors.aiRulesPayloadTooLarge");
  if (trimmed === "error_payload_too_large" || /network alert rules payload too large/i.test(trimmed)) return t("errors.networkRulesPayloadTooLarge");
  if (/^error_invalid_json:/i.test(trimmed) || /invalid network alert rules JSON/i.test(trimmed)) return t("errors.invalidNetworkRulesJson");
  if (/No network snapshot available yet/i.test(trimmed)) return t("errors.networkSnapshotUnavailable");
  if (/^error_batch_limit:/i.test(trimmed) || /batch limited to/i.test(trimmed)) return t("errors.killBatchLimit");
  if (/process not found/i.test(trimmed)) return t("errors.processNotFound");
  if (invalidPidMatch) return t("aiChat.invalidPid", { pid: invalidPidMatch[2] });
  if (/invalid pid|pid inválido/i.test(trimmed)) return t("errors.invalidPid");
  if (/refusing to kill protected process/i.test(trimmed)) return t("errors.protectedProcess");
  if (/plugin data dir unavailable/i.test(trimmed)) return t("plugins.errors.dataDirUnavailable");
  if (/plugins must use the \.lua extension/i.test(trimmed)) return t("plugins.errors.invalidExtension");
  if (/invalid plugin file name/i.test(trimmed)) return t("plugins.errors.invalidFileName");
  if (/plugin file name must contain ASCII letters or numbers/i.test(trimmed)) return t("plugins.errors.invalidAsciiName");
  if (/plugin returned too many metrics/i.test(trimmed)) return t("plugins.errors.tooManyMetrics");
  if (/plugin metric values must be finite numbers/i.test(trimmed)) return t("plugins.errors.metricFinite");
  if (/metric name cannot be empty/i.test(trimmed)) return t("plugins.errors.metricNameEmpty");
  if (/unsupported kind/i.test(trimmed)) return t("plugins.errors.metricKindUnsupported");
  if (/plugin execution exceeded the time budget/i.test(trimmed)) return t("plugins.errors.timeBudgetExceeded");
  if (/plugin must export a collect\(ctx\) function/i.test(trimmed)) return t("plugins.errors.collectMissing");
  if (/plugin source cannot be empty/i.test(trimmed)) return t("plugins.errors.sourceEmpty");
  if (/plugin source exceeds/i.test(trimmed)) return t("plugins.errors.sourceTooLarge");
  if (/plugin registry is full/i.test(trimmed)) return t("plugins.errors.registryFull");
  if (/plugin .* was not found/i.test(trimmed)) return t("plugins.errors.notFound");
  if (trimmed === "prompt_injection_blocked") return t("aiChat.blockedPrompt");
  if (/^error_unknown_browser:/i.test(trimmed) || /Unknown browser/i.test(trimmed)) return t("errors.unknownBrowser");
  if (trimmed === "error_invalid_tab_id") return t("errors.invalidTabId");
  if (trimmed === "error_cdp_not_localhost") return t("errors.cdpNotLocalhost");
  if (trimmed === "error_firefox_not_supported") return t("errors.firefoxNotSupported");
  if (/^error_tab_id_/i.test(trimmed) || /^error_tab_url_/i.test(trimmed)) return t("errors.invalidTabInput");
  if (/Unknown AI provider/i.test(trimmed)) return t("errors.unknownAiProvider");
  if (/Ollama is not running/i.test(trimmed)) return t("aiChat.errorApi");
  if (/Invalid API key/i.test(trimmed)) return t("settings.apiKeyFailed");
  if (toolInvalidPidMatch) return t("aiChat.invalidPid", { pid: toolInvalidPidMatch[1] });
  if (toolProcessNotFoundMatch) return t("aiChat.processNotFound", { pid: toolProcessNotFoundMatch[1] });
  if (trimmed === "tool_no_process_name") return t("aiChat.noProcessNameProvided");
  if (toolNoProcessesMatchedMatch) return t("aiChat.noProcessesMatched", { name: toolNoProcessesMatchedMatch[1] });
  if (trimmed === "tool_close_tabs_missing_pattern") return t("aiChat.noPatternProvided");
  if (trimmed === "tool_automation_rule_missing_fields") return t("automations.ruleMissingFields");
  if (trimmed === "tool_automation_rule_id_missing") return t("automations.ruleIdInvalid");
  if (trimmed === "tool_automation_rule_not_found") return t("automations.ruleNotFound");
  if (/^tool_automation_rule_add_failed:/i.test(trimmed)) return t("automations.ruleAddFailed");
  if (/^tool_automation_rule_remove_failed:/i.test(trimmed)) return t("automations.ruleRemoveFailed");
  if (trimmed === "tool_process_details_not_found") return t("aiChat.processDetailsNotFound");
  if (trimmed === "tool_process_details_ready") return t("aiChat.processDetailsLoaded", { name: t("common.untitled"), pid: "?" });
  if (trimmed === "tool_network_details_none") return t("aiChat.noActiveConnections");
  if (trimmed === "tool_network_details_found") return t("aiChat.networkConnectionsFound", { count: 0 });
  if (trimmed === "tool_security_scan_completed") return t("aiChat.securityScanCompleted", { count: 0 });
  if (trimmed === "tool_process_explanation_unavailable") return t("aiChat.processExplanationUnavailable");
  if (trimmed === "tool_process_explanation_ready") return t("aiChat.processExplanationLoaded", { name: t("common.untitled") });
  if (trimmed === "tool_system_summary_ready") return t("aiChat.systemSummaryTitle");
  if (trimmed === "automation_rule_added") return t("automations.ruleAdded");
  if (trimmed === "automation_rule_args_invalid") return t("automations.ruleArgsInvalid");
  if (trimmed === "automation_rule_removed") return t("automations.ruleRemoved");
  if (trimmed === "automation_rule_id_invalid") return t("automations.ruleIdInvalid");
  if (toolUnknownMatch) return t("aiChat.unknownTool");
  if (/Unknown tool:/i.test(trimmed)) return t("aiChat.unknownTool");

  return trimmed;
}

export function formatDynamicAlertMessage(alert: DynamicAlert): string {
  return t("alerts.dynamicRuleTriggered", {
    process: alert.process_name,
    pid: alert.pid,
    rule: formatLocalizedLabel(alert.rule_name),
  });
}

export function formatDynamicAlertTitle(alert: DynamicAlert): string {
  return t("alerts.dynamicRuleTitle", { rule: formatLocalizedLabel(alert.rule_name) });
}

export function formatNetworkAlertMessage(alert: NetworkAlert): string {
  switch (alert.condition_kind) {
    case "high_bandwidth":
      return t("networkAlertMessages.highBandwidth", {
        process: alert.process_name ?? t("alerts.system"),
        bandwidth: alert.bandwidth_mbps?.toFixed(2) ?? "0.00",
      });
    case "new_external_connection":
      return t("networkAlertMessages.newExternalConnection", {
        destination: alert.destination ?? t("networkDetail.unknownRegion"),
      });
    case "unusual_port":
      return t("networkAlertMessages.unusualPort", {
        destination: alert.destination ?? t("networkDetail.unknownRegion"),
      });
    case "process_network_spike":
      return t("networkAlertMessages.processSpike", {
        process: alert.process_name ?? t("alerts.system"),
      });
    case "connection_count_exceeded":
      return t("networkAlertMessages.connectionCountExceeded", {
        process: alert.process_name ?? t("alerts.system"),
        count: alert.connection_count ?? 0,
      });
    case "suspicious_destination":
      return t("networkAlertMessages.suspiciousDestination", {
        destination: alert.destination ?? t("networkDetail.unknownRegion"),
      });
    default:
      return alert.message;
  }
}

export function formatNetworkAlertDetails(alert: NetworkAlert): string {
  const segments = [
    alert.bandwidth_mbps != null
      ? t("networkAlertMessages.bandwidthDetail", { value: alert.bandwidth_mbps.toFixed(2) })
      : null,
    alert.connection_count != null
      ? t("networkAlertMessages.connectionsDetail", { count: alert.connection_count })
      : null,
    alert.destination,
  ].filter(Boolean);

  return segments.join(" - ");
}

export function formatToolResultDetails(result: ToolResult): string {
  if (!result.success) {
    return localizeBackendError(result.details);
  }

  if (result.tool === "run_security_scan" && result.payload) {
    const findings = Array.isArray(result.payload.findings) ? result.payload.findings.length : 0;
    return t("aiChat.securityScanCompleted", { count: findings });
  }

  if (result.tool === "get_network_details" && result.payload) {
    const connections = Array.isArray(result.payload.connections) ? result.payload.connections.length : 0;
    return connections > 0
      ? t("aiChat.networkConnectionsFound", { count: connections })
      : t("aiChat.noActiveConnections");
  }

  if (result.tool === "get_process_details" && result.payload) {
    return t("aiChat.processDetailsLoaded", {
      name: String(result.payload.name ?? t("common.untitled")),
      pid: String(result.payload.pid ?? "?"),
    });
  }

  if (result.tool === "explain_process" && result.payload) {
    return t("aiChat.processExplanationLoaded", {
      name: String(result.payload.name ?? t("common.untitled")),
    });
  }

  if (result.tool === "get_system_summary") {
    return t("aiChat.systemSummaryTitle");
  }

  if (result.tool === "explain_process" && !result.success) {
    return t("aiChat.processExplanationUnavailable");
  }

  return localizeBackendError(result.details);
}
