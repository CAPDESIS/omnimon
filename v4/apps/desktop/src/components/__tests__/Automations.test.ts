import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { invoke } from "@tauri-apps/api/core";

import Automations from "../Automations.svelte";

type AutomationRule = {
  id: string;
  process_pattern: string;
  metric: string;
  threshold: number;
  duration_secs: number;
  action: string;
};

const mockInvoke = vi.mocked(invoke);

describe("Automations", () => {
  let rules: AutomationRule[];

  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    rules = [
      {
        id: "rule-1",
        process_pattern: "Chrome",
        metric: "cpu",
        threshold: 80,
        duration_secs: 60,
        action: "alert",
      },
      {
        id: "rule-2",
        process_pattern: "node",
        metric: "ram",
        threshold: 1024,
        duration_secs: 120,
        action: "kill",
      },
    ];

    mockInvoke.mockReset();
    mockInvoke.mockImplementation(async (command, payload) => {
      if (command === "get_automation_rules") {
        return [...rules];
      }

      if (command === "add_automation_rule") {
        const nextRule = (payload as { rule: AutomationRule }).rule;
        rules = [...rules, nextRule];
        return undefined;
      }

      if (command === "remove_automation_rule") {
        const id = (payload as { id: string }).id;
        rules = rules.filter((rule) => rule.id !== id);
        return undefined;
      }

      throw new Error(`Unexpected command: ${String(command)}`);
    });
  });

  it("renderiza lista de reglas", async () => {
    render(Automations, { props: { onclose: vi.fn() } });

    await waitFor(() => {
      expect(screen.getByText(/Chrome > 80 cpu for 60s -> alert/)).toBeInTheDocument();
      expect(screen.getByText(/node > 1024 ram for 120s -> kill/)).toBeInTheDocument();
    });
  });

  it("permite activar o desactivar una regla existente", async () => {
    render(Automations, { props: { onclose: vi.fn() } });

    await waitFor(() => {
      expect(screen.getByText(/Chrome > 80 cpu for 60s -> alert/)).toBeInTheDocument();
    });

    await fireEvent.click(screen.getAllByText("Delete")[0]);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("remove_automation_rule", { id: "rule-1" });
      expect(screen.queryByText(/Chrome > 80 cpu for 60s -> alert/)).not.toBeInTheDocument();
      expect(screen.getByText(/node > 1024 ram for 120s -> kill/)).toBeInTheDocument();
    });
  });

  it("muestra estado vacio cuando no hay reglas", async () => {
    rules = [];

    const { container } = render(Automations, { props: { onclose: vi.fn() } });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("get_automation_rules");
    });

    expect(screen.getByText("Automations Engine")).toBeInTheDocument();
    expect(container.querySelectorAll(".rule-item")).toHaveLength(0);
  });

  it("bloquea agregar una regla con patrón vacío y muestra error", async () => {
    rules = [];
    const { container } = render(Automations, { props: { onclose: vi.fn() } });

    // Wait for the builder form to be mounted (not the loading spinner).
    await waitFor(() => {
      expect(container.querySelector(".builder")).not.toBeNull();
    });

    const addBtn = container.querySelector(
      '.builder button[data-variant="primary"]',
    ) as HTMLButtonElement;
    expect(addBtn).toBeTruthy();
    await fireEvent.click(addBtn);

    await waitFor(() => {
      expect(screen.getByRole("alert")).toBeInTheDocument();
    });
    // add_automation_rule must not have been invoked.
    expect(
      mockInvoke.mock.calls.find((call) => call[0] === "add_automation_rule"),
    ).toBeUndefined();
  });

  it("muestra error cuando loadRules falla", async () => {
    mockInvoke.mockReset();
    mockInvoke.mockImplementation(async (command) => {
      if (command === "get_automation_rules") {
        throw new Error("backend unreachable");
      }
      return undefined;
    });

    render(Automations, { props: { onclose: vi.fn() } });

    await waitFor(() => {
      expect(screen.getByRole("alert")).toBeInTheDocument();
    });
    expect(screen.getByRole("alert").textContent).toMatch(/backend unreachable/);
  });

  it("muestra error cuando removeRule falla", async () => {
    mockInvoke.mockReset();
    let callCount = 0;
    mockInvoke.mockImplementation(async (command) => {
      if (command === "get_automation_rules") {
        callCount += 1;
        return callCount === 1 ? [...rules] : rules;
      }
      if (command === "remove_automation_rule") {
        throw new Error("remove boom");
      }
      return undefined;
    });

    render(Automations, { props: { onclose: vi.fn() } });

    await waitFor(() => {
      expect(screen.getByText(/Chrome > 80 cpu/)).toBeInTheDocument();
    });

    await fireEvent.click(screen.getAllByText("Delete")[0]);

    await waitFor(() => {
      expect(screen.getByRole("alert").textContent).toMatch(/remove boom/);
    });
    // Rule should still be visible because the remove failed.
    expect(screen.getByText(/Chrome > 80 cpu/)).toBeInTheDocument();
  });
});
