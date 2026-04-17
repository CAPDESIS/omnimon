import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import ZombieKiller from "../ZombieKiller.svelte";

type ZombieCandidate = {
  pid: number;
  name: string;
  execName: string;
  exePath: string | null;
  cpuPct: number;
  memoryBytes: number;
  ageSecs: number;
  reason: "cpu_sustained" | "ram_sustained" | "cpu_and_ram_sustained";
  startTime: number;
};

type ZombieKillerConfig = {
  enabled: boolean;
  cpuThresholdPct: number;
  ramThresholdBytes: number;
  minUptimeSecs: number;
  sustainedSecs: number;
  autoKill: boolean;
  neverKill: string[];
};

const mockInvoke = vi.mocked(invoke);
const mockListen = vi.mocked(listen);

function baseConfig(): ZombieKillerConfig {
  return {
    enabled: true,
    cpuThresholdPct: 50,
    ramThresholdBytes: 0,
    minUptimeSecs: 7 * 24 * 60 * 60,
    sustainedSecs: 3600,
    autoKill: false,
    neverKill: [],
  };
}

function makeCandidate(overrides: Partial<ZombieCandidate> = {}): ZombieCandidate {
  return {
    pid: 1337,
    name: "adobe",
    execName: "adobe",
    exePath: null,
    cpuPct: 107.7,
    memoryBytes: 50_000_000,
    ageSecs: 30 * 24 * 60 * 60,
    reason: "cpu_sustained",
    startTime: 1_700_000_000,
    ...overrides,
  };
}

describe("ZombieKiller", () => {
  let config: ZombieKillerConfig;
  let zombies: ZombieCandidate[];

  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    config = baseConfig();
    zombies = [];

    mockInvoke.mockReset();
    mockInvoke.mockImplementation(async (command, payload) => {
      if (command === "get_zombie_killer_config") return { ...config };
      if (command === "list_zombie_candidates") return [...zombies];
      if (command === "set_zombie_killer_config") {
        config = { ...(payload as { config: ZombieKillerConfig }).config };
        return undefined;
      }
      if (command === "kill_zombie") {
        const pid = (payload as { pid: number }).pid;
        zombies = zombies.filter((z) => z.pid !== pid);
        return { pid, processName: "x", killed: true };
      }
      if (command === "kill_all_zombies") {
        const killed = zombies.map((z) => ({
          pid: z.pid,
          processName: z.name,
          killed: true,
        }));
        zombies = [];
        return killed;
      }
      throw new Error(`Unexpected command: ${String(command)}`);
    });

    mockListen.mockReset();
    mockListen.mockImplementation(async () => () => {});
  });

  it("carga configuración y lista al montar", async () => {
    render(ZombieKiller, { props: { onclose: vi.fn() } });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("get_zombie_killer_config");
      expect(mockInvoke).toHaveBeenCalledWith("list_zombie_candidates");
      expect(screen.getByText(/Detected processes \(0\)/)).toBeInTheDocument();
    });

    expect(screen.getByText("Zombie Killer")).toBeInTheDocument();
  });

  it("muestra empty state cuando no hay zombies", async () => {
    render(ZombieKiller, { props: { onclose: vi.fn() } });

    await waitFor(() => {
      expect(screen.getByText("No zombies")).toBeInTheDocument();
    });
  });

  it("renderiza un candidato y permite matarlo", async () => {
    zombies = [makeCandidate({ pid: 5377, name: "AdobeIPCBroker" })];

    render(ZombieKiller, { props: { onclose: vi.fn() } });

    await waitFor(() => {
      expect(screen.getByText("AdobeIPCBroker")).toBeInTheDocument();
      expect(screen.getByText("PID 5377")).toBeInTheDocument();
      expect(screen.getByText("Sustained high CPU")).toBeInTheDocument();
    });

    await fireEvent.click(screen.getByRole("button", { name: /^Kill$/ }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("kill_zombie", { pid: 5377 });
      expect(screen.queryByText("AdobeIPCBroker")).not.toBeInTheDocument();
    });
  });

  it("botón 'Matar todos' despacha kill_all_zombies", async () => {
    zombies = [
      makeCandidate({ pid: 100, name: "proc-a" }),
      makeCandidate({ pid: 200, name: "proc-b" }),
    ];

    render(ZombieKiller, { props: { onclose: vi.fn() } });

    await waitFor(() => {
      expect(screen.getByText(/Detected processes \(2\)/)).toBeInTheDocument();
    });

    await fireEvent.click(screen.getByRole("button", { name: /Kill all/ }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("kill_all_zombies");
      expect(screen.queryByText("proc-a")).not.toBeInTheDocument();
      expect(screen.queryByText("proc-b")).not.toBeInTheDocument();
    });
  });

  it("guarda configuración con clamps aplicados", async () => {
    render(ZombieKiller, { props: { onclose: vi.fn() } });

    await waitFor(() => {
      expect(screen.getByText("Configuration")).toBeInTheDocument();
    });

    await fireEvent.click(screen.getByRole("button", { name: /Save configuration/ }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "set_zombie_killer_config",
        expect.objectContaining({
          config: expect.objectContaining({
            enabled: true,
            autoKill: false,
            cpuThresholdPct: 50,
            minUptimeSecs: 7 * 24 * 60 * 60,
            sustainedSecs: 3600,
          }),
        }),
      );
    });
  });

  it("se suscribe al evento zombie-killer-update", async () => {
    render(ZombieKiller, { props: { onclose: vi.fn() } });

    await waitFor(() => {
      expect(mockListen).toHaveBeenCalledWith("zombie-killer-update", expect.any(Function));
    });
  });

  it("agrega y quita entradas de la blocklist 'nunca matar'", async () => {
    render(ZombieKiller, { props: { onclose: vi.fn() } });

    await waitFor(() => {
      expect(screen.getByText(/Never kill/)).toBeInTheDocument();
    });

    const input = screen.getByPlaceholderText(/AdobePremierePro/) as HTMLInputElement;
    await fireEvent.input(input, { target: { value: "FinalCutPro" } });
    await fireEvent.click(screen.getByRole("button", { name: /^Add$/ }));

    await waitFor(() => {
      expect(screen.getByText("FinalCutPro")).toBeInTheDocument();
    });

    await fireEvent.click(screen.getByRole("button", { name: /Remove FinalCutPro/ }));

    await waitFor(() => {
      expect(screen.queryByText("FinalCutPro")).not.toBeInTheDocument();
    });
  });
});
