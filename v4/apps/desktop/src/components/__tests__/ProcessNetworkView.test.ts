import { cleanup, render, screen } from "@testing-library/svelte";
import ProcessNetworkView from "../ProcessNetworkView.svelte";

const { mockProcessSummary } = vi.hoisted(() => ({
  mockProcessSummary: [
    {
      name: "Chrome",
      connectionsCount: 3,
      totalUp: 2_048,
      totalDown: 4_096,
      topDest: "example.com",
    },
    {
      name: "Firefox",
      connectionsCount: 1,
      totalUp: 512,
      totalDown: 256,
      topDest: "mozilla.org",
    },
  ],
}));

vi.mock("../../stores/network.svelte", () => ({
  getPerProcessSummary: () => mockProcessSummary,
}));

describe("ProcessNetworkView", () => {
  afterEach(() => {
    cleanup();
  });

  it("renders per-process summary rows", () => {
    render(ProcessNetworkView);

    expect(screen.getByText("Network usage by process")).toBeInTheDocument();
    expect(screen.getByText("Chrome")).toBeInTheDocument();
    expect(screen.getByText("example.com")).toBeInTheDocument();
    expect(screen.getByText("2.0 KB/s")).toBeInTheDocument();
    expect(screen.getByText("4.0 KB/s")).toBeInTheDocument();
  });

  it("shows chart placeholder", () => {
    render(ProcessNetworkView);
    expect(screen.getByText(/bandwidth distribution/i)).toBeInTheDocument();
  });
});
