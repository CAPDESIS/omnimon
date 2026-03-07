import { render, screen, fireEvent } from "@testing-library/svelte";
import NetworkMap from "../NetworkMap.svelte";
import { writable } from "svelte/store";
import type { NetworkConnection } from "../../lib/types";

const { mockNetworkConnections } = vi.hoisted(() => {
  const { writable } = require("svelte/store") as typeof import("svelte/store");
  return {
    mockNetworkConnections: writable<NetworkConnection[]>([]),
  };
});

vi.mock("../../stores/security", () => ({
  networkConnections: mockNetworkConnections,
}));

function makeConn(overrides?: Partial<NetworkConnection>): NetworkConnection {
  return {
    pid: 1,
    process_name: "Chrome",
    remote_addr: "example.com",
    remote_port: 443,
    protocol: "tcp",
    direction: "outbound",
    bytes_sent: 0,
    bytes_recv: 0,
    state: "ESTABLISHED",
    ...overrides,
  };
}

describe("NetworkMap", () => {
  beforeEach(() => {
    mockNetworkConnections.set([]);
  });

  it("renders nothing when no connections", () => {
    render(NetworkMap);
    expect(screen.queryByText("Network Map")).not.toBeInTheDocument();
  });

  it("shows toggle button when connections exist", () => {
    mockNetworkConnections.set([makeConn()]);
    render(NetworkMap);
    expect(screen.getByText("Network Map")).toBeInTheDocument();
  });

  it("displays connection count and process count", () => {
    mockNetworkConnections.set([
      makeConn({ process_name: "Chrome", remote_addr: "google.com" }),
      makeConn({ process_name: "Chrome", remote_addr: "github.com" }),
      makeConn({ process_name: "Firefox", pid: 2, remote_addr: "mozilla.org" }),
    ]);
    render(NetworkMap);
    expect(screen.getByText("3 connections / 2 processes")).toBeInTheDocument();
  });

  it("starts collapsed by default", () => {
    mockNetworkConnections.set([makeConn()]);
    render(NetworkMap);
    expect(screen.queryByText("example.com")).not.toBeInTheDocument();
  });

  it("expands on toggle click and shows process names and domains", async () => {
    mockNetworkConnections.set([
      makeConn({ process_name: "Chrome", remote_addr: "google.com" }),
    ]);
    render(NetworkMap);
    const toggle = screen.getByText("Network Map").closest("button")!;
    await fireEvent.click(toggle);
    expect(screen.getByText("Chrome")).toBeInTheDocument();
    expect(screen.getByText("google.com")).toBeInTheDocument();
  });

  it("toggle button is always visible when connections exist", async () => {
    mockNetworkConnections.set([makeConn()]);
    render(NetworkMap);
    const toggle = screen.getByText("Network Map").closest("button")!;
    await fireEvent.click(toggle);
    expect(screen.getByText("Chrome")).toBeInTheDocument();
    // Toggle button remains available after second click
    await fireEvent.click(toggle);
    expect(screen.getByText("Network Map")).toBeInTheDocument();
  });

  it("groups connections by process", async () => {
    mockNetworkConnections.set([
      makeConn({ process_name: "Chrome", remote_addr: "google.com" }),
      makeConn({ process_name: "Chrome", remote_addr: "youtube.com" }),
      makeConn({ process_name: "Firefox", pid: 2, remote_addr: "mozilla.org" }),
    ]);
    render(NetworkMap);
    await fireEvent.click(screen.getByText("Network Map").closest("button")!);
    const procNames = screen.getAllByText(/^(Chrome|Firefox)$/);
    expect(procNames[0].textContent).toBe("Chrome");
  });

  it("shows connection count per process", async () => {
    mockNetworkConnections.set([
      makeConn({ process_name: "Chrome", remote_addr: "google.com" }),
      makeConn({ process_name: "Chrome", remote_addr: "youtube.com" }),
    ]);
    render(NetworkMap);
    await fireEvent.click(screen.getByText("Network Map").closest("button")!);
    expect(screen.getByText("2")).toBeInTheDocument();
  });

  it("shows +N when more than 5 domains", async () => {
    const conns = [];
    for (let i = 0; i < 7; i++) {
      conns.push(makeConn({
        process_name: "Chrome",
        remote_addr: `domain${i}.com`,
      }));
    }
    mockNetworkConnections.set(conns);
    render(NetworkMap);
    await fireEvent.click(screen.getByText("Network Map").closest("button")!);
    expect(screen.getByText("+2")).toBeInTheDocument();
  });

  it("deduplicates same domain:port connections", async () => {
    mockNetworkConnections.set([
      makeConn({ process_name: "Chrome", remote_addr: "google.com", remote_port: 443 }),
      makeConn({ process_name: "Chrome", remote_addr: "google.com", remote_port: 443 }),
    ]);
    render(NetworkMap);
    await fireEvent.click(screen.getByText("Network Map").closest("button")!);
    const chips = screen.getAllByText("google.com");
    expect(chips).toHaveLength(1);
  });

  it("renders canvas element when expanded", async () => {
    mockNetworkConnections.set([makeConn()]);
    render(NetworkMap);
    await fireEvent.click(screen.getByText("Network Map").closest("button")!);
    const canvas = document.querySelector("canvas");
    expect(canvas).toBeInTheDocument();
  });
});
