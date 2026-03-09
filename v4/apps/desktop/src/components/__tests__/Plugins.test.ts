import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";

import Plugins from "../Plugins.svelte";
import type { PluginDescriptor } from "../../lib/types";

const { mockListPlugins, mockInstallPlugin, mockRemovePlugin, mockSetPluginEnabled } = vi.hoisted(() => ({
  mockListPlugins: vi.fn<() => Promise<PluginDescriptor[]>>(),
  mockInstallPlugin: vi.fn<(name: string, source: string) => Promise<void>>(),
  mockRemovePlugin: vi.fn<(id: string) => Promise<void>>(),
  mockSetPluginEnabled: vi.fn<(id: string, enabled: boolean) => Promise<void>>(),
}));

vi.mock("../../lib/ipc", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../lib/ipc")>();
  return {
    ...actual,
    ipcListPlugins: mockListPlugins,
    ipcInstallPlugin: mockInstallPlugin,
    ipcRemovePlugin: mockRemovePlugin,
    ipcSetPluginEnabled: mockSetPluginEnabled,
  };
});

function makePlugin(overrides: Partial<PluginDescriptor> = {}): PluginDescriptor {
  return {
    id: "docker-monitor",
    name: "Docker Monitor",
    file_name: "docker.lua",
    enabled: true,
    description: "Reports Docker metrics",
    version: "1.0.0",
    status: "ok",
    last_error: null,
    last_run_ms: Date.now() - 2_000,
    last_duration_ms: 32,
    metrics: [
      {
        name: "docker.containers.running",
        label: "Running containers",
        kind: "gauge",
        value: 3,
        unit: "count",
        tags: { source: "demo" },
      },
    ],
    ...overrides,
  };
}

describe("Plugins", () => {
  beforeEach(() => {
    mockListPlugins.mockReset();
    mockInstallPlugin.mockReset();
    mockRemovePlugin.mockReset();
    mockSetPluginEnabled.mockReset();
    mockListPlugins.mockResolvedValue([]);
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it("renderiza sin errores en estado vacio", async () => {
    render(Plugins, { props: { onclose: vi.fn() } });

    await waitFor(() => {
      expect(screen.getByRole("dialog", { name: "Plugins" })).toBeInTheDocument();
      expect(screen.getByText("No plugins installed")).toBeInTheDocument();
      expect(screen.getByText("Load a collector")).toBeInTheDocument();
    });
  });

  it("refleja props y cierra desde boton o backdrop", async () => {
    const onclose = vi.fn();
    render(Plugins, { props: { onclose } });

    await screen.findByRole("dialog", { name: "Plugins" });
    await fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(onclose).toHaveBeenCalledTimes(1);

    const backdrop = document.querySelector(".plugins-backdrop") as HTMLElement;
    await fireEvent.click(backdrop);
    expect(onclose).toHaveBeenCalledTimes(2);
  });

  it("muestra loading mientras carga plugins", () => {
    mockListPlugins.mockReturnValueOnce(new Promise(() => {}));

    render(Plugins, { props: { onclose: vi.fn() } });

    expect(screen.getByText("Loading...")).toBeInTheDocument();
  });

  it("muestra resumen y metricas cuando hay plugins cargados", async () => {
    mockListPlugins.mockResolvedValueOnce([makePlugin()]);

    render(Plugins, { props: { onclose: vi.fn() } });

    await waitFor(() => {
      expect(screen.getByText("Docker Monitor")).toBeInTheDocument();
      expect(screen.getByText("Reports Docker metrics")).toBeInTheDocument();
      expect(screen.getByText("Installed: 1")).toBeInTheDocument();
      expect(screen.getByText("Active: 1")).toBeInTheDocument();
      expect(screen.getByText("Live metrics: 1")).toBeInTheDocument();
      expect(screen.getByText("Running containers")).toBeInTheDocument();
      expect(screen.getByText("3 count")).toBeInTheDocument();
      expect(screen.getByText("source:demo")).toBeInTheDocument();
    });
  });

  it("permite habilitar y deshabilitar plugins", async () => {
    mockListPlugins
      .mockResolvedValueOnce([makePlugin({ enabled: true })])
      .mockResolvedValueOnce([makePlugin({ enabled: false, status: "idle" })]);

    render(Plugins, { props: { onclose: vi.fn() } });

    const toggleButton = await screen.findByRole("button", { name: "Disable" });
    await fireEvent.click(toggleButton);

    await waitFor(() => {
      expect(mockSetPluginEnabled).toHaveBeenCalledWith("docker-monitor", false);
      expect(screen.getByRole("button", { name: "Enable" })).toBeInTheDocument();
      expect(screen.getByText("Disabled")).toBeInTheDocument();
    });
  });

  it("muestra error cuando togglePlugin falla", async () => {
    mockListPlugins.mockResolvedValueOnce([makePlugin({ enabled: true })]);
    mockSetPluginEnabled.mockRejectedValueOnce(new Error("toggle failed"));

    render(Plugins, { props: { onclose: vi.fn() } });

    await fireEvent.click(await screen.findByRole("button", { name: "Disable" }));

    await waitFor(() => {
      expect(mockSetPluginEnabled).toHaveBeenCalledWith("docker-monitor", false);
      expect(screen.getByText("toggle failed")).toBeInTheDocument();
    });
  });

  it("permite eliminar plugins", async () => {
    mockListPlugins
      .mockResolvedValueOnce([makePlugin()])
      .mockResolvedValueOnce([]);

    render(Plugins, { props: { onclose: vi.fn() } });

    await fireEvent.click(await screen.findByRole("button", { name: "Remove" }));

    await waitFor(() => {
      expect(mockRemovePlugin).toHaveBeenCalledWith("docker-monitor");
      expect(screen.getByText("No plugins installed")).toBeInTheDocument();
    });
  });

  it("muestra error cuando removePlugin falla", async () => {
    mockListPlugins.mockResolvedValueOnce([makePlugin()]);
    mockRemovePlugin.mockRejectedValueOnce(new Error("remove failed"));

    render(Plugins, { props: { onclose: vi.fn() } });

    await fireEvent.click(await screen.findByRole("button", { name: "Remove" }));

    await waitFor(() => {
      expect(mockRemovePlugin).toHaveBeenCalledWith("docker-monitor");
      expect(screen.getByText("remove failed")).toBeInTheDocument();
    });
  });

  it("muestra indicador de refreshing mientras recarga plugins", async () => {
    vi.useFakeTimers();
    let resolveRefresh!: (value: PluginDescriptor[]) => void;
    const refreshDeferred = new Promise<PluginDescriptor[]>((resolve) => {
      resolveRefresh = resolve;
    });

    mockListPlugins
      .mockResolvedValueOnce([makePlugin()])
      .mockReturnValueOnce(refreshDeferred);

    render(Plugins, { props: { onclose: vi.fn() } });

    await screen.findByText("Docker Monitor");

    await vi.advanceTimersByTimeAsync(4000);

    await waitFor(() => {
      expect(screen.getByText("Refreshing...")).toBeInTheDocument();
    });

    resolveRefresh([makePlugin({ name: "Docker Monitor reloaded" })]);

    await waitFor(() => {
      expect(screen.queryByText("Refreshing...")).not.toBeInTheDocument();
      expect(screen.getByText("Docker Monitor reloaded")).toBeInTheDocument();
    });

    vi.useRealTimers();
  });

  it("sube un plugin y muestra estado success", async () => {
    const file = new File(["return {}"], "collector.lua", { type: "text/plain" });
    mockListPlugins
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([makePlugin({ file_name: "collector.lua", name: "Collector" })]);

    render(Plugins, { props: { onclose: vi.fn() } });

    const input = document.querySelector("input[type='file']") as HTMLInputElement;
    await fireEvent.change(input, { target: { files: [file] } });

    await waitFor(() => {
      expect(mockInstallPlugin).toHaveBeenCalledWith("collector.lua", "return {}");
      expect(screen.getByText("Loaded collector.lua")).toBeInTheDocument();
      expect(screen.getByText("Collector")).toBeInTheDocument();
    });
  });

  it("ignora upload vacio", async () => {
    render(Plugins, { props: { onclose: vi.fn() } });

    const input = document.querySelector("input[type='file']") as HTMLInputElement;
    await fireEvent.change(input, { target: { files: [] } });

    expect(mockInstallPlugin).not.toHaveBeenCalled();
  });

  it("muestra errores de carga y subida fallida", async () => {
    mockListPlugins.mockRejectedValueOnce(new Error("list failed"));

    render(Plugins, { props: { onclose: vi.fn() } });
    expect(await screen.findByText("list failed")).toBeInTheDocument();

    cleanup();
    const file = new File(["bad"], "broken.lua", { type: "text/plain" });
    mockListPlugins.mockResolvedValueOnce([]);
    mockInstallPlugin.mockRejectedValueOnce(new Error("invalid lua"));

    render(Plugins, { props: { onclose: vi.fn() } });
    const input = document.querySelector("input[type='file']") as HTMLInputElement;
    await fireEvent.change(input, { target: { files: [file] } });

    expect(await screen.findByText("invalid lua")).toBeInTheDocument();
  });

  it("muestra estados edge: sin descripcion, sin version, sin metricas y ultimo error", async () => {
    mockListPlugins.mockResolvedValueOnce([
      makePlugin({
        description: null,
        version: null,
        enabled: false,
        status: "error",
        last_error: "collector failed",
        last_run_ms: null,
        metrics: [],
      }),
    ]);

    render(Plugins, { props: { onclose: vi.fn() } });

    await waitFor(() => {
      expect(screen.getByText("No plugin description provided.")).toBeInTheDocument();
      expect(screen.getByText("No version")).toBeInTheDocument();
      expect(screen.getByText(/Last run: Never/)).toBeInTheDocument();
      expect(screen.getByText("This plugin has not emitted metrics yet.")).toBeInTheDocument();
      expect(screen.getByText("collector failed")).toBeInTheDocument();
    });
  });

  it("no renderiza tags cuando metric.tags esta vacio", async () => {
    mockListPlugins.mockResolvedValueOnce([
      makePlugin({
        metrics: [
          {
            name: "docker.containers.running",
            label: "Running containers",
            kind: "gauge",
            value: 3,
            unit: "count",
            tags: {},
          },
        ],
      }),
    ]);

    render(Plugins, { props: { onclose: vi.fn() } });

    await screen.findByText("Running containers");

    expect(document.querySelector(".metric-tags")).toBeNull();
  });

  it("muestra fallbacks cuando un plugin no tiene descripcion ni version", async () => {
    mockListPlugins.mockResolvedValueOnce([
      makePlugin({
        description: null,
        version: null,
      }),
    ]);

    render(Plugins, { props: { onclose: vi.fn() } });

    await waitFor(() => {
      expect(screen.getByText("No plugin description provided.")).toBeInTheDocument();
      expect(screen.getByText("No version")).toBeInTheDocument();
    });
  });
});
