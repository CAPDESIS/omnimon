import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/svelte";

import ContextAiChat from "../ContextAiChat.svelte";

const { mockAnalyzeContext, mockAiProviderConfig, mockRenderMarkdown } = vi.hoisted(() => ({
  mockAnalyzeContext: vi.fn<(context: string, provider: string, model: string) => Promise<string>>(),
  mockAiProviderConfig: (() => {
    const { writable } = require("svelte/store") as typeof import("svelte/store");
    return writable({ provider: "openrouter", model: "test-model" });
  })(),
  mockRenderMarkdown: vi.fn((value: string) => `<p>${value}</p>`),
}));

vi.mock("../../lib/ipc", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../lib/ipc")>();
  return {
    ...actual,
    ipcAnalyzeContext: mockAnalyzeContext,
  };
});

vi.mock("../../stores/preferences", () => ({
  aiProviderConfig: mockAiProviderConfig,
}));

vi.mock("../../lib/markdown", () => ({
  renderMarkdown: mockRenderMarkdown,
}));

vi.mock("../../lib/chatUtils", () => ({
  scrollToBottom: vi.fn(),
  resizeInput: vi.fn(),
}));

describe("ContextAiChat", () => {
  const baseProps = {
    title: "CPU Assistant",
    placeholder: "Ask about CPU",
    emptyState: "No messages yet",
    buildContext: (question: string) => `CTX:${question}`,
    helpTooltip: "Helpful context",
    sendLabel: "Analyze",
    inputAriaLabel: "Question input",
    maxHeight: 260,
  };

  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    mockAiProviderConfig.set({ provider: "openrouter", model: "test-model" });
    mockAnalyzeContext.mockReset();
    mockAnalyzeContext.mockResolvedValue("**CPU spike** on renderer");
    mockRenderMarkdown.mockClear();
  });

  it("renderiza sin errores y refleja props", () => {
    render(ContextAiChat, { props: baseProps });

    expect(screen.getByRole("region", { name: "CPU Assistant" })).toBeInTheDocument();
    expect(screen.getByPlaceholderText("Ask about CPU")).toBeInTheDocument();
    expect(screen.getByLabelText("Question input")).toBeInTheDocument();
    expect(screen.getByText("No messages yet")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Analyze" })).toBeDisabled();
  });

  it("envia contexto al hacer click en el boton", async () => {
    render(ContextAiChat, { props: baseProps });

    const input = screen.getByLabelText("Question input");
    await fireEvent.input(input, { target: { value: "Which process is hot?" } });
    await fireEvent.click(screen.getByRole("button", { name: "Analyze" }));

    await waitFor(() => {
      expect(mockAnalyzeContext).toHaveBeenCalledWith(
        "CTX:Which process is hot?",
        "openrouter",
        "test-model",
      );
      expect(screen.getByText("Which process is hot?")).toBeInTheDocument();
      expect(screen.getByText("CPU spike", { exact: false })).toBeInTheDocument();
    });
  });

  it("envia al presionar Enter y no con Shift+Enter", async () => {
    render(ContextAiChat, { props: baseProps });

    const input = screen.getByLabelText("Question input");
    await fireEvent.input(input, { target: { value: "Explain this spike" } });
    await fireEvent.keyDown(input, { key: "Enter", shiftKey: true });
    expect(mockAnalyzeContext).not.toHaveBeenCalled();

    await fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() => {
      expect(mockAnalyzeContext).toHaveBeenCalledTimes(1);
    });
  });

  it("muestra estado loading mientras espera respuesta", async () => {
    let resolvePromise: (value: string) => void = () => {};
    mockAnalyzeContext.mockReturnValueOnce(
      new Promise<string>((resolve) => {
        resolvePromise = resolve;
      }),
    );

    render(ContextAiChat, { props: baseProps });

    const input = screen.getByLabelText("Question input");
    await fireEvent.input(input, { target: { value: "Long request" } });
    await fireEvent.click(screen.getByRole("button", { name: "Analyze" }));

    expect(screen.getByText("Thinking", { exact: false })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "..." })).toBeDisabled();

    resolvePromise("Done");

    await waitFor(() => {
      expect(screen.queryByText("Thinking", { exact: false })).not.toBeInTheDocument();
      expect(screen.getByText("Done", { exact: false })).toBeInTheDocument();
    });
  });

  it("muestra errores del backend", async () => {
    mockAnalyzeContext.mockRejectedValueOnce(new Error("context unavailable"));

    render(ContextAiChat, { props: baseProps });

    const input = screen.getByLabelText("Question input");
    await fireEvent.input(input, { target: { value: "Fail please" } });
    await fireEvent.click(screen.getByRole("button", { name: "Analyze" }));

    await waitFor(() => {
      expect(screen.getByText("context unavailable")).toBeInTheDocument();
      expect(screen.getByText("System")).toBeInTheDocument();
    });
  });

  it("permite limpiar la conversacion despues de una respuesta exitosa", async () => {
    render(ContextAiChat, { props: baseProps });

    const input = screen.getByLabelText("Question input");
    await fireEvent.input(input, { target: { value: "Need summary" } });
    await fireEvent.click(screen.getByRole("button", { name: "Analyze" }));

    await screen.findByText("Need summary");
    await fireEvent.click(screen.getByRole("button", { name: "Clear" }));

    await waitFor(() => {
      expect(screen.getByText("No messages yet")).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Clear" })).not.toBeInTheDocument();
    });
  });

  it("renderiza markdown en respuestas del asistente", async () => {
    render(ContextAiChat, { props: baseProps });

    const input = screen.getByLabelText("Question input");
    await fireEvent.input(input, { target: { value: "Format this" } });
    await fireEvent.click(screen.getByRole("button", { name: "Analyze" }));

    await waitFor(() => {
      expect(mockRenderMarkdown).toHaveBeenCalledWith("**CPU spike** on renderer");
      expect(screen.getByText("CPU spike", { exact: false }).tagName).toBe("P");
    });
  });

  it("tolera respuestas vacias del backend", async () => {
    mockAnalyzeContext.mockResolvedValueOnce("");

    render(ContextAiChat, { props: baseProps });

    const input = screen.getByLabelText("Question input");
    await fireEvent.input(input, { target: { value: "Empty answer" } });
    await fireEvent.click(screen.getByRole("button", { name: "Analyze" }));

    await waitFor(() => {
      expect(screen.getByText("You")).toBeInTheDocument();
      expect(screen.getByText("AI")).toBeInTheDocument();
    });
  });
});
