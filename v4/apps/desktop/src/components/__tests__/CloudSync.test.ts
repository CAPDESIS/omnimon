import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { invoke } from "@tauri-apps/api/core";

import CloudSync from "../CloudSync.svelte";

const mockInvoke = vi.mocked(invoke);

describe("CloudSync", () => {
  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("sincroniza con exito via sync_cloud", async () => {
    mockInvoke.mockImplementation(async (command, payload) => {
      if (command === "get_cloud_key") return "saved-key";
      if (command === "save_cloud_key") return undefined;
      if (command === "sync_cloud") return { uploaded_bytes: 123, sha256: "a".repeat(64) };
      throw new Error(`Unexpected command: ${String(command)} ${JSON.stringify(payload)}`);
    });

    render(CloudSync);

    const input = await screen.findByLabelText("API Key:");
    expect(input).toHaveValue("saved-key");

    await fireEvent.click(screen.getByText("Sync now"));

    expect(await screen.findByText("Sync completed (123 bytes uploaded).")).toBeInTheDocument();
    expect(mockInvoke).toHaveBeenCalledWith("sync_cloud");
  });

  it("muestra error limpio sin clave al sincronizar", async () => {
    mockInvoke.mockImplementation(async (command) => {
      if (command === "get_cloud_key") return "saved-key";
      if (command === "sync_cloud") throw "no cloud key configured";
      return undefined;
    });

    render(CloudSync);
    await screen.findByLabelText("API Key:");
    await fireEvent.click(screen.getByText("Sync now"));

    expect(await screen.findByText("Error: no cloud key configured")).toBeInTheDocument();
  });

  it("maneja estado offline", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("offline"));

    render(CloudSync);

    await waitFor(() => {
      const input = screen.getByLabelText("API Key:");
      expect(input).toHaveValue("");
      expect(screen.getByText("Save key")).toBeDisabled();
    });
  });
});
