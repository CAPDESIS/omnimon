import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import NetworkDashboard from "../NetworkDashboard.svelte";

const { mockInitNetworkListener, mockState } = vi.hoisted(() => {
  const state = {
    snapshot: {
      timestamp: 1,
      total_bytes_per_sec_up: 2_048,
      total_bytes_per_sec_down: 4_096,
      active_connections: 3,
      processes_with_network: 2,
      connections: [],
    },
    history: [],
    filter: {
      protocol: "",
      process: "",
      host: "",
      hideLocalhost: false,
      onlyEstablished: false,
      minSpeed: 0,
    },
    isCapturing: true,
    error: null,
  };

  return {
    mockInitNetworkListener: vi.fn(async () => vi.fn()),
    mockState: state,
  };
});

vi.mock("../../stores/network.svelte", () => ({
  getNetworkState: () => mockState,
  getTotalUp: () => 2_048,
  getTotalDown: () => 4_096,
  getFilteredConnections: () => [],
  getPerProcessSummary: () => [],
  initNetworkListener: mockInitNetworkListener,
}));

describe("NetworkDashboard", () => {
  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    mockState.snapshot = {
      timestamp: 1,
      total_bytes_per_sec_up: 2_048,
      total_bytes_per_sec_down: 4_096,
      active_connections: 3,
      processes_with_network: 2,
      connections: [],
    };
    mockInitNetworkListener.mockClear();
  });

  it("renders summary metrics and initializes listener", () => {
    render(NetworkDashboard);

    expect(screen.getByText("Total upload")).toBeInTheDocument();
    expect(screen.getByText("2.0 KB/s ↑")).toBeInTheDocument();
    expect(screen.getByText("4.0 KB/s ↓")).toBeInTheDocument();
    expect(screen.getByText("Active connections")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(mockInitNetworkListener).toHaveBeenCalled();
    expect(screen.getByPlaceholderText("Filter by process...")).toBeInTheDocument();
  });

  it("renders real SVG history chart from store history", () => {
    (mockState as { history: unknown[] }).history = [1, 2, 3].map((i) => ({
      timestamp: i, total_bytes_per_sec_up: i * 1024, total_bytes_per_sec_down: i * 2048,
      active_connections: 1, processes_with_network: 1, connections: [],
    }));
    const { container } = render(NetworkDashboard);
    expect(screen.getByTestId("network-history-chart")).toBeInTheDocument();
    expect(container.querySelectorAll("[data-testid='network-history-chart'] path")).toHaveLength(4);
    mockState.history = [];
  });

  it("switches to process tab", async () => {
    render(NetworkDashboard);

    await fireEvent.click(screen.getByText("Process view"));

    expect(screen.getByText("Network usage by process")).toBeInTheDocument();
    expect(screen.queryByPlaceholderText("Filter by process...")).not.toBeInTheDocument();
  });

  it("drops the listener if unmounted before init resolves", async () => {
    const unlisten = vi.fn();
    let resolveListener: ((fn: typeof unlisten) => void) | undefined;
    mockInitNetworkListener.mockImplementationOnce(
      () =>
        new Promise<typeof unlisten>((resolve) => {
          resolveListener = resolve;
        }),
    );

    const { unmount } = render(NetworkDashboard);
    unmount();
    expect(resolveListener).toBeDefined();
    resolveListener!(unlisten);
    await Promise.resolve();
    await Promise.resolve();

    expect(unlisten).toHaveBeenCalled();
  });
});
