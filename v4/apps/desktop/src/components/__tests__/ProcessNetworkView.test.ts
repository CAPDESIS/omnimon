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

  it("renders real donut chart with shares, no placeholder", () => {
    const { container } = render(ProcessNetworkView);
    expect(screen.getByTestId("process-donut-chart")).toBeInTheDocument();
    expect(container.querySelectorAll("[data-testid='process-donut-chart'] circle")).toHaveLength(3); // track + 2 segments
    expect(screen.getByText(/Chrome 88\.9%/)).toBeInTheDocument();
    expect(screen.queryByText(/Pie\/Donut chart/)).not.toBeInTheDocument();
  });

  it("shows empty state when no process has network activity", () => {
    mockState.summary = [];
    render(ProcessNetworkView);
    expect(screen.getByText("No processes with network activity")).toBeInTheDocument();
  });
});
