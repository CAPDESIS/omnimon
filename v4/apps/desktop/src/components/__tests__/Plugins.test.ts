import { cleanup, render, screen, waitFor } from "@testing-library/svelte";

import Plugins from "../Plugins.svelte";
import type { PluginDescriptor } from "../../lib/types";

const { mockListPlugins, mockInstallPlugin, mockRemovePlugin, mockSetPluginEnabled } = vi.hoisted(() => ({
  mockListPlugins: vi.fn<() => Promise<PluginDescriptor[]>>(),
  mockInstallPlugin: vi.fn(),
  mockRemovePlugin: vi.fn(),
  mockSetPluginEnabled: vi.fn(),
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

describe("Plugins", () => {
  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    mockListPlugins.mockReset();
    mockInstallPlugin.mockReset();
    mockRemovePlugin.mockReset();
    mockSetPluginEnabled.mockReset();
    mockListPlugins.mockResolvedValue([]);
  });

  it("renderiza sin errores", async () => {
    render(Plugins, { props: { onclose: vi.fn() } });

    await waitFor(() => {
      expect(screen.getByRole("dialog", { name: "Plugins" })).toBeInTheDocument();
      expect(screen.getByText("No plugins installed")).toBeInTheDocument();
    });
  });
});
