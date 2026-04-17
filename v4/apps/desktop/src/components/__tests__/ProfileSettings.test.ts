import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, fireEvent, screen, waitFor } from "@testing-library/svelte";
import { invoke } from "@tauri-apps/api/core";
import ProfileSettings from "../ProfileSettings.svelte";
import {
  displayName,
  profilePreset,
  dashboardLayout,
  refreshInterval,
  favoriteProcesses,
  notificationLevel,
  aiPrivacyMode,
  aiDailyLimit,
} from "../../stores/preferences";
import { get } from "svelte/store";

const mockedInvoke = vi.mocked(invoke);

describe("ProfileSettings", () => {
  beforeEach(() => {
    displayName.set("Test User");
    profilePreset.set("balanced");
    dashboardLayout.set("standard");
    refreshInterval.set(500);
    favoriteProcesses.set(["chrome"]);
    notificationLevel.set("all");
    aiPrivacyMode.set(false);
    aiDailyLimit.set(null);
    mockedInvoke.mockReset();
    mockedInvoke.mockResolvedValue([0, 200]);
  });

  it("renders all controls", () => {
    render(ProfileSettings);
    expect(screen.getByLabelText(/Display Name/i)).toBeDefined();
    expect(screen.getByText(/Balanced/i)).toBeDefined();
    expect(screen.getByLabelText(/Layout/i)).toBeDefined();
    expect(screen.getByLabelText(/Refresh Interval/i)).toBeDefined();
    expect(screen.getByLabelText(/Notifications/i)).toBeDefined();
    expect(screen.getByLabelText(/Favorite Processes/i)).toBeDefined();
    expect(screen.getByText("chrome")).toBeDefined();
  });

  it("changes preset and verifies options update", async () => {
    render(ProfileSettings);
    const minimalBtn = screen.getByText("Minimal");
    await fireEvent.click(minimalBtn);

    expect(get(profilePreset)).toBe("minimal");
    expect(get(dashboardLayout)).toBe("compact");
    expect(get(refreshInterval)).toBe(5000);
    expect(get(notificationLevel)).toBe("off");
  });

  it("adds and removes favorite processes", async () => {
    render(ProfileSettings);
    const input = screen.getByLabelText(/Favorite Processes/i);
    await fireEvent.input(input, { target: { value: "firefox" } });
    const addBtn = screen.getByLabelText("Add");
    await fireEvent.click(addBtn);

    expect(get(favoriteProcesses)).toContain("firefox");
    expect(screen.getByText("firefox")).toBeDefined();

    const removeBtns = screen.getAllByLabelText("Remove favorite process");
    await fireEvent.click(removeBtns[1]); // Remove firefox

    expect(get(favoriteProcesses)).not.toContain("firefox");
  });

  it("reset to defaults works", async () => {
    aiPrivacyMode.set(true);
    aiDailyLimit.set(50);
    render(ProfileSettings);
    const resetBtn = screen.getByText(/Reset to Defaults/i);
    await fireEvent.click(resetBtn);

    expect(get(displayName)).toBe("");
    expect(get(profilePreset)).toBe("balanced");
    expect(get(dashboardLayout)).toBe("standard");
    expect(get(refreshInterval)).toBe(500);
    expect(get(favoriteProcesses)).toEqual([]);
    expect(get(notificationLevel)).toBe("all");
    expect(get(aiPrivacyMode)).toBe(false);
    expect(get(aiDailyLimit)).toBeNull();
  });

  it("toggles AI privacy mode", async () => {
    render(ProfileSettings);
    const toggle = document.getElementById("aiPrivacyMode") as HTMLInputElement;
    expect(toggle).toBeDefined();
    expect(toggle.checked).toBe(false);

    await fireEvent.click(toggle);
    expect(get(aiPrivacyMode)).toBe(true);

    await fireEvent.click(toggle);
    expect(get(aiPrivacyMode)).toBe(false);
  });

  it("accepts a valid AI daily limit and clearing it to null", async () => {
    render(ProfileSettings);
    const input = document.getElementById("aiDailyLimit") as HTMLInputElement;
    expect(input).toBeDefined();

    await fireEvent.input(input, { target: { value: "150" } });
    expect(get(aiDailyLimit)).toBe(150);

    await fireEvent.input(input, { target: { value: "0" } });
    expect(get(aiDailyLimit)).toBe(0);

    await fireEvent.input(input, { target: { value: "  " } });
    expect(get(aiDailyLimit)).toBeNull();
  });

  it("ignores out-of-range or non-integer AI daily limit inputs", async () => {
    aiDailyLimit.set(42);
    render(ProfileSettings);
    const input = document.getElementById("aiDailyLimit") as HTMLInputElement;

    await fireEvent.input(input, { target: { value: "-5" } });
    expect(get(aiDailyLimit)).toBe(42);

    await fireEvent.input(input, { target: { value: "999999" } });
    expect(get(aiDailyLimit)).toBe(42);

    await fireEvent.input(input, { target: { value: "3.14" } });
    expect(get(aiDailyLimit)).toBe(42);
  });

  it("invokes get_ai_daily_usage on mount and on refresh click", async () => {
    mockedInvoke.mockResolvedValueOnce([3, 200]);
    render(ProfileSettings);
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("get_ai_daily_usage");
    });

    mockedInvoke.mockResolvedValueOnce([7, 200]);
    const refreshBtn = screen.getByRole("button", {
      name: /refresh/i,
    });
    await fireEvent.click(refreshBtn);
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledTimes(2);
    });
  });

  it("tolerates get_ai_daily_usage rejection without throwing", async () => {
    mockedInvoke.mockRejectedValueOnce(new Error("rate_limited"));
    // Should render without throwing even if the IPC fails.
    expect(() => render(ProfileSettings)).not.toThrow();
    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("get_ai_daily_usage");
    });
  });
});
