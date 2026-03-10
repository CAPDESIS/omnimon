import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { get, writable } from "svelte/store";
import { beforeEach, describe, expect, it } from "vitest";

import { locale } from "../../lib/i18n";

const mockNetworkAlertRules = writable([
  {
    id: "rule-1",
    name: "Puerto sospechoso",
    enabled: true,
    condition: { kind: "unusual_port", suspicious_ports: [4444, 6667] },
    severity: "warning" as const,
    cooldown_seconds: 30,
    notify_ai: false,
  },
]);

vi.mock("../../stores/preferences", () => ({
  networkAlertRules: mockNetworkAlertRules,
}));

describe("NetworkAlertConfig", () => {
  beforeEach(async () => {
    locale.set("es");
    mockNetworkAlertRules.set([
      {
        id: "rule-1",
        name: "Puerto sospechoso",
        enabled: true,
        condition: { kind: "unusual_port", suspicious_ports: [4444, 6667] },
        severity: "warning",
        cooldown_seconds: 30,
        notify_ai: false,
      },
    ]);
  });

  afterEach(() => {
    cleanup();
  });

  it("renders existing rules", async () => {
    const { default: NetworkAlertConfig } = await import("../NetworkAlertConfig.svelte");
    render(NetworkAlertConfig);

    expect(screen.getByText("Reglas configurables")).toBeInTheDocument();
    expect(screen.getByText("Puerto sospechoso")).toBeInTheDocument();
    expect(screen.getByText("Puertos: 4444, 6667")).toBeInTheDocument();
  });

  it("creates a new rule from the modal", async () => {
    const { default: NetworkAlertConfig } = await import("../NetworkAlertConfig.svelte");
    render(NetworkAlertConfig);

    await fireEvent.click(screen.getByText("Agregar regla"));
    await fireEvent.input(screen.getByPlaceholderText("Mi alerta de red"), {
      target: { value: "Subida alta" },
    });
    await fireEvent.click(screen.getByText("Guardar regla"));

    const rules = get(mockNetworkAlertRules);
    expect(rules).toHaveLength(2);
    expect(rules[1].name).toBe("Subida alta");
    expect(rules[1].condition.kind).toBe("high_bandwidth");
  });

  it("edits and removes rules", async () => {
    const { default: NetworkAlertConfig } = await import("../NetworkAlertConfig.svelte");
    render(NetworkAlertConfig);

    await fireEvent.click(screen.getByText("Editar"));
    await fireEvent.input(screen.getByPlaceholderText("Mi alerta de red"), {
      target: { value: "Puerto editado" },
    });
    await fireEvent.click(screen.getByText("Guardar regla"));
    expect(get(mockNetworkAlertRules)[0].name).toBe("Puerto editado");

    await fireEvent.click(screen.getByText("Eliminar"));
    expect(get(mockNetworkAlertRules)).toHaveLength(0);
  });
});
