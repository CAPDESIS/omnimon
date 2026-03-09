import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";

import SmartAlerts from "../SmartAlerts.svelte";

type SmartAlert = {
  id: string;
  problem: string;
  explanation: string;
  processPid?: number;
  processName?: string;
  timestamp: number;
};

const { mockSmartAlerts, mockDismissSmartAlert, mockKillSingle } = vi.hoisted(() => {
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  return {
    mockSmartAlerts: writable<SmartAlert[]>([]),
    mockDismissSmartAlert: vi.fn(),
    mockKillSingle: vi.fn(async () => true),
  };
});

vi.mock("../../stores/alerts", () => ({
  smartAlerts: mockSmartAlerts,
  dismissSmartAlert: mockDismissSmartAlert,
}));

vi.mock("../../stores/processes", () => ({
  killSingle: mockKillSingle,
}));

describe("SmartAlerts", () => {
  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    mockSmartAlerts.set([]);
    mockDismissSmartAlert.mockClear();
    mockKillSingle.mockClear();
  });

  it("renderiza lista de alertas", () => {
    mockSmartAlerts.set([
      {
        id: "smart-1",
        problem: "High CPU usage",
        explanation: "Chrome is keeping many active tabs awake.",
        processPid: 123,
        processName: "Chrome",
        timestamp: Date.now(),
      },
    ]);

    render(SmartAlerts);

    expect(screen.getByText("Health Report")).toBeInTheDocument();
    expect(screen.getByText("High CPU usage")).toBeInTheDocument();
    expect(screen.getByText(/keeping many active tabs/)).toBeInTheDocument();
  });

  it("permite dismiss de alerta", async () => {
    mockSmartAlerts.set([
      {
        id: "smart-2",
        problem: "Disk pressure",
        explanation: "A background task is writing too much data.",
        timestamp: Date.now(),
      },
    ]);

    render(SmartAlerts);

    await fireEvent.click(screen.getByRole("button", { name: "Dismiss" }));

    expect(mockDismissSmartAlert).toHaveBeenCalledWith("smart-2");
  });

  it("muestra estado vacio", () => {
    render(SmartAlerts);

    expect(document.querySelector(".smart-alert-card")).not.toBeInTheDocument();
  });
});
