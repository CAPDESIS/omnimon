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

  it("renderiza status de sincronizacion", async () => {
    mockInvoke.mockImplementation(async (command, payload) => {
      if (command === "get_cloud_key") return "saved-key";
      if (command === "save_cloud_key") return undefined;
      throw new Error(`Unexpected command: ${String(command)} ${JSON.stringify(payload)}`);
    });

    render(CloudSync);

    const input = await screen.findByLabelText("API Key:");
    expect(input).toHaveValue("saved-key");

    await fireEvent.click(screen.getByText("Sync now"));

    expect(screen.getByText("Sync is not implemented yet.")).toBeInTheDocument();
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
