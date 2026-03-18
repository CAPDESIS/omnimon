import { describe, it, expect, beforeEach, vi } from "vitest";
import { render, fireEvent, screen } from "@testing-library/svelte";
import ProfileSettings from "../ProfileSettings.svelte";
import {
  displayName,
  profilePreset,
  dashboardLayout,
  refreshInterval,
  favoriteProcesses,
  notificationLevel,
} from "../../stores/preferences";
import { get } from "svelte/store";

describe("ProfileSettings", () => {
  beforeEach(() => {
    displayName.set("Test User");
    profilePreset.set("balanced");
    dashboardLayout.set("standard");
    refreshInterval.set(500);
    favoriteProcesses.set(["chrome"]);
    notificationLevel.set("all");
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
    render(ProfileSettings);
    const resetBtn = screen.getByText(/Reset to Defaults/i);
    await fireEvent.click(resetBtn);

    expect(get(displayName)).toBe("");
    expect(get(profilePreset)).toBe("balanced");
    expect(get(dashboardLayout)).toBe("standard");
    expect(get(refreshInterval)).toBe(500);
    expect(get(favoriteProcesses)).toEqual([]);
    expect(get(notificationLevel)).toBe("all");
  });
});
