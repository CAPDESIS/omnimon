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

    expect(screen.getByText("Total Upload")).toBeInTheDocument();
    expect(screen.getByText("2.0 KB/s ↑")).toBeInTheDocument();
    expect(screen.getByText("4.0 KB/s ↓")).toBeInTheDocument();
    expect(screen.getByText("Conexiones activas")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(mockInitNetworkListener).toHaveBeenCalled();
    expect(screen.getByPlaceholderText("Filtrar por proceso...")).toBeInTheDocument();
  });

  it("switches to process tab", async () => {
    render(NetworkDashboard);

    await fireEvent.click(screen.getByText("Vista por Proceso"));

    expect(screen.getByText("Uso de red por proceso")).toBeInTheDocument();
    expect(screen.queryByPlaceholderText("Filtrar por proceso...")).not.toBeInTheDocument();
  });
});
