import { fireEvent, render, screen } from "@testing-library/svelte";
import ProfilePanel from "../ProfilePanel.svelte";

const { mockAiProfile, mockUserMode } = vi.hoisted(() => {
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  return {
    mockAiProfile: writable("general"),
    mockUserMode: writable("pro"),
  };
});

vi.mock("../../stores/processes", () => ({
  aiProfile: mockAiProfile,
}));

vi.mock("../../stores/preferences", () => ({
  userMode: mockUserMode,
}));

describe("ProfilePanel", () => {
  beforeEach(() => {
    mockAiProfile.set("general");
    mockUserMode.set("pro");
  });

  it("renders profile options", () => {
    render(ProfilePanel);
    expect(screen.getAllByText("General").length).toBeGreaterThan(0);
    expect(screen.getByText("Developer")).toBeInTheDocument();
    expect(screen.getByText("Gaming")).toBeInTheDocument();
    expect(screen.getByText("Battery Saver")).toBeInTheDocument();
  });

  it("updates selected profile on click", async () => {
    render(ProfilePanel);
    await fireEvent.click(screen.getByText("Gaming"));
    let value = "";
    mockAiProfile.subscribe((v) => { value = v; })();
    expect(value).toBe("gaming");
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
