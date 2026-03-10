import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";

import SmartAlerts from "../SmartAlerts.svelte";

type SmartAlert = {
  id: string;
  problem: string;
  explanation: string;
  processPid?: number;
  processName?: string;
  timestamp: number;
  updateCount?: number;
};

const { mockSmartAlerts, mockDismissSmartAlert, mockDismissAllSmartAlerts, mockKillSingle } = vi.hoisted(() => {
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  return {
    mockSmartAlerts: writable<SmartAlert[]>([]),
    mockDismissSmartAlert: vi.fn(),
    mockDismissAllSmartAlerts: vi.fn(),
    mockKillSingle: vi.fn(async () => true),
  };
});

vi.mock("../../stores/alerts", () => ({
  smartAlerts: mockSmartAlerts,
  dismissSmartAlert: mockDismissSmartAlert,
  dismissAllSmartAlerts: mockDismissAllSmartAlerts,
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
    mockDismissAllSmartAlerts.mockClear();
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

  it("muestra badge de actualizacion y permite cerrar todas cuando hay varias alertas", async () => {
    mockSmartAlerts.set([
      {
        id: "smart-1",
        problem: "CPU spike",
        explanation: "One app is busy.",
        timestamp: Date.now(),
        updateCount: 3,
      },
      {
        id: "smart-2",
        problem: "Disk pressure",
        explanation: "A sync task is active.",
        timestamp: Date.now(),
      },
    ]);

    render(SmartAlerts);

    expect(screen.getByText("Actualizada 3x")).toBeInTheDocument();

    await fireEvent.click(screen.getByRole("button", { name: /cerrar todas/i }));

    expect(mockDismissAllSmartAlerts).toHaveBeenCalledOnce();
  });

  it("limita la vista a las ultimas cinco alertas y muestra contador oculto", () => {
    mockSmartAlerts.set(
      Array.from({ length: 7 }, (_, index) => ({
        id: `smart-${index + 1}`,
        problem: `Problem ${index + 1}`,
        explanation: `Explanation ${index + 1}`,
        timestamp: index + 1,
      })),
    );

    render(SmartAlerts);

    expect(screen.getByText(/\+2 alertas m.s/i)).toBeInTheDocument();
    expect(screen.queryByText("Problem 1")).not.toBeInTheDocument();
    expect(screen.queryByText("Problem 2")).not.toBeInTheDocument();
    expect(screen.getByText("Problem 7")).toBeInTheDocument();
  });

  it("ejecuta force quit y luego descarta la alerta cuando hay pid", async () => {
    mockSmartAlerts.set([
      {
        id: "smart-3",
        problem: "Busy process",
        explanation: "This app is consuming resources.",
        processPid: 321,
        timestamp: Date.now(),
      },
    ]);

    render(SmartAlerts);

    await fireEvent.click(screen.getByRole("button", { name: "Force Quit" }));

    expect(mockKillSingle).toHaveBeenCalledWith(321);
    expect(mockDismissSmartAlert).toHaveBeenCalledWith("smart-3");
  });

  it("no muestra boton de force quit cuando falta el pid", () => {
    mockSmartAlerts.set([
      {
        id: "smart-4",
        problem: "Background activity",
        explanation: "No PID available.",
        timestamp: Date.now(),
      },
    ]);

    render(SmartAlerts);

    expect(screen.queryByRole("button", { name: "Force Quit" })).not.toBeInTheDocument();
  });

  it("muestra estado vacio", () => {
    render(SmartAlerts);

    expect(document.querySelector(".smart-alert-card")).not.toBeInTheDocument();
  });
});
