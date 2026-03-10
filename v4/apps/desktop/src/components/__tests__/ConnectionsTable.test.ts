import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import ConnectionsTable from "../ConnectionsTable.svelte";

import { locale } from "../../lib/i18n";

const { mockState, mockConnections } = vi.hoisted(() => ({
  mockState: {
    filter: {
      protocol: "",
      process: "",
      host: "",
      hideLocalhost: false,
      onlyEstablished: false,
      minSpeed: 0,
    },
  },
  mockConnections: [
    {
      process_id: 1,
      process_name: "Chrome",
      protocol: "TCP",
      local_address: "127.0.0.1",
      local_port: 51000,
      remote_address: "8.8.8.8",
      remote_port: 443,
      remote_hostname: "dns.google",
      state: "ESTABLISHED",
      bytes_up: 4_096,
      bytes_down: 8_192,
      bytes_per_sec_up: 2_048,
      bytes_per_sec_down: 1_024,
    },
    {
      process_id: 2,
      process_name: "Firefox",
      protocol: "UDP",
      local_address: "127.0.0.1",
      local_port: 52000,
      remote_address: "1.1.1.1",
      remote_port: 53,
      remote_hostname: "one.one.one.one",
      state: "LISTEN",
      bytes_up: 2_048,
      bytes_down: 1_024,
      bytes_per_sec_up: 512,
      bytes_per_sec_down: 256,
    },
  ],
}));

vi.mock("../../stores/network.svelte", () => ({
  getNetworkState: () => mockState,
  getFilteredConnections: () => mockConnections,
}));

describe("ConnectionsTable", () => {
  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    locale.set("es");
    mockState.filter = {
      protocol: "",
      process: "",
      host: "",
      hideLocalhost: false,
      onlyEstablished: false,
      minSpeed: 0,
    };
  });

  it("renders rows and filter controls", () => {
    render(ConnectionsTable);

    expect(screen.getByDisplayValue("Todos los protocolos")).toBeInTheDocument();
    expect(screen.getByPlaceholderText("Filtrar por proceso...")).toBeInTheDocument();
    expect(screen.getByText(/Chrome/)).toBeInTheDocument();
    expect(screen.getByText(/Firefox/)).toBeInTheDocument();
  });

  it("updates bound filter inputs", async () => {
    render(ConnectionsTable);

    await fireEvent.input(screen.getByPlaceholderText("Filtrar por proceso..."), {
      target: { value: "fire" },
    });
    await fireEvent.click(screen.getByLabelText("Ocultar localhost"));

    expect(mockState.filter.process).toBe("fire");
    expect(mockState.filter.hideLocalhost).toBe(true);
  });

  it("shows expanded row details when clicking a connection", async () => {
    render(ConnectionsTable);

    await fireEvent.click(screen.getByText(/Chrome/));

    expect(screen.getByText("Total subida:")).toBeInTheDocument();
    expect(screen.getByText("4.00 KB")).toBeInTheDocument();
    expect(screen.getByText("8.00 KB")).toBeInTheDocument();
  });
});
