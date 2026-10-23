import { cleanup, render, screen } from "@testing-library/svelte";
import ProcessNetworkView from "../ProcessNetworkView.svelte";

const { mockState } = vi.hoisted(() => ({
  mockState: {
    summary: [
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
    ] as Array<{
      name: string;
      connectionsCount: number;
      totalUp: number;
      totalDown: number;
      topDest: string;
    }>,
  },
}));

vi.mock("../../stores/network.svelte", () => ({
  getPerProcessSummary: () => mockState.summary,
}));

describe("ProcessNetworkView", () => {
  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    mockState.summary = [
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
    ];
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

  it("shows empty state when no process has network activity", () => {
    mockState.summary = [];
    render(ProcessNetworkView);
    expect(screen.getByText("No processes with network activity")).toBeInTheDocument();
  });
});
