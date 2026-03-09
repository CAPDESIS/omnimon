import { fireEvent, render, screen } from "@testing-library/svelte";
import ProfilePanel from "../ProfilePanel.svelte";

const { mockAiProfile, mockUserMode, mockProfilePresets, mockActiveProfilePreset, mockApplyProfilePresetById, mockSyncAiProfileToPreset } = vi.hoisted(() => {
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  return {
    mockAiProfile: writable("general"),
    mockUserMode: writable("pro"),
    mockProfilePresets: writable([
      { id: "general", label: "General", idleThreshold: 1, pollIntervalMs: 2000, automationIntervalSecs: 5, aiProfile: "general" },
      { id: "developer", label: "Developer", idleThreshold: 0.6, pollIntervalMs: 1500, automationIntervalSecs: 3, aiProfile: "developer" },
      { id: "gaming", label: "Gaming", idleThreshold: 0.4, pollIntervalMs: 1000, automationIntervalSecs: 2, aiProfile: "gaming" },
      { id: "battery", label: "Battery Saver", idleThreshold: 2.0, pollIntervalMs: 4000, automationIntervalSecs: 10, aiProfile: "battery" },
    ]),
    mockActiveProfilePreset: writable("general"),
    mockApplyProfilePresetById: vi.fn(() => true),
    mockSyncAiProfileToPreset: vi.fn(),
  };
});

vi.mock("../../stores/processes", () => ({
  aiProfile: mockAiProfile,
}));

vi.mock("../../stores/preferences", () => ({
  userMode: mockUserMode,
  profilePresets: mockProfilePresets,
  activeProfilePreset: mockActiveProfilePreset,
  applyProfilePresetById: mockApplyProfilePresetById,
  syncAiProfileToPreset: mockSyncAiProfileToPreset,
}));

describe("ProfilePanel", () => {
  beforeEach(() => {
    mockAiProfile.set("general");
    mockUserMode.set("pro");
    mockApplyProfilePresetById.mockClear();
    mockSyncAiProfileToPreset.mockClear();
  });

  it("renders profile options", () => {
    render(ProfilePanel);
    expect(screen.getAllByText("General").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Developer").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Gaming").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Battery Saver").length).toBeGreaterThan(0);
  });

  it("updates selected profile on click", async () => {
    render(ProfilePanel);
    await fireEvent.click(screen.getAllByText("Gaming")[0]);
    let value = "";
    mockAiProfile.subscribe((v) => { value = v; })();
    expect(value).toBe("gaming");
    expect(mockSyncAiProfileToPreset).toHaveBeenCalledWith("gaming");
  });

  it("updates user mode on click", async () => {
    render(ProfilePanel);
    await fireEvent.click(screen.getByText("Basic Mode"));
    let value = "";
    mockUserMode.subscribe((v) => { value = v; })();
    expect(value).toBe("basic");
  });

  it("shows the active workspace copy", () => {
    render(ProfilePanel);
    expect(screen.getByText(/Active workspace/i)).toBeInTheDocument();
    expect(screen.getByText(/This preference is saved on this device/i)).toBeInTheDocument();
  });
});
