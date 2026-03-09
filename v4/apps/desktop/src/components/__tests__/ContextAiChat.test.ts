import { cleanup, render, screen } from "@testing-library/svelte";

import ContextAiChat from "../ContextAiChat.svelte";

describe("ContextAiChat", () => {
  afterEach(() => {
    cleanup();
  });

  it("renderiza sin errores", () => {
    render(ContextAiChat, {
      props: {
        title: "CPU Assistant",
        placeholder: "Ask about CPU",
        emptyState: "No messages yet",
        buildContext: (question: string) => question,
      },
    });

    expect(screen.getByRole("region", { name: "CPU Assistant" })).toBeInTheDocument();
    expect(screen.getByPlaceholderText("Ask about CPU")).toBeInTheDocument();
    expect(screen.getByText("No messages yet")).toBeInTheDocument();
  });
});
