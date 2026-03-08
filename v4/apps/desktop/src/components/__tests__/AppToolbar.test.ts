import { fireEvent, render, screen } from "@testing-library/svelte";
import AppToolbar from "../AppToolbar.svelte";

vi.mock("../AlertPanel.svelte", () => ({
  default: () => ({
    $$render: () => '<div data-testid="alert-panel">alerts</div>',
  }),
}));

describe("AppToolbar", () => {
  function renderToolbar() {
    return render(AppToolbar, {
      props: {
        searchValue: "chrome",
        onsearch: vi.fn(),
        onclearsearch: vi.fn(),
        selectedCount: 2,
        selectedRamMB: 512,
        grouping: true,
        totalFindings: 3,
        aiLoading: false,
        aiProfile: "developer",
        fontSize: 14,
        onselectall: vi.fn(),
        onselectnone: vi.fn(),
        onkillselected: vi.fn(),
        ontogglegrouping: vi.fn(),
        onchangepofile: vi.fn(),
        onanalyze: vi.fn(),
        onopensecurity: vi.fn(),
        ontoggledashboard: vi.fn(),
        dashboardCollapsed: false,
        ontoggleautomations: vi.fn(),
        onopenplugins: vi.fn(),
        onopensettings: vi.fn(),
        onopenhelp: vi.fn(),
        ondecreasefont: vi.fn(),
        onincreasefont: vi.fn(),
      },
    });
  }

  it("renders sponsor, grouping and profile controls", () => {
    renderToolbar();
    expect(screen.getByRole("link", { name: /Sponsor/i })).toBeInTheDocument();
    expect(screen.getByText("Groups")).toBeInTheDocument();
    expect(screen.getByDisplayValue("Developer")).toBeInTheDocument();
    expect(screen.getByText("Security")).toBeInTheDocument();
  });

  it("calls profile callback when profile changes", async () => {
    const onchangepofile = vi.fn();
    render(AppToolbar, {
      props: {
        searchValue: "",
        onsearch: vi.fn(),
        onclearsearch: vi.fn(),
        selectedCount: 0,
        selectedRamMB: 0,
        grouping: false,
        totalFindings: 0,
        aiLoading: false,
        aiProfile: "general",
        fontSize: 12,
        onselectall: vi.fn(),
        onselectnone: vi.fn(),
        onkillselected: vi.fn(),
        ontogglegrouping: vi.fn(),
        onchangepofile,
        onanalyze: vi.fn(),
        onopensecurity: vi.fn(),
        ontoggledashboard: vi.fn(),
        dashboardCollapsed: false,
        ontoggleautomations: vi.fn(),
        onopenplugins: vi.fn(),
        onopensettings: vi.fn(),
        onopenhelp: vi.fn(),
        ondecreasefont: vi.fn(),
        onincreasefont: vi.fn(),
      },
    });

    await fireEvent.change(screen.getAllByLabelText("AI profile")[0], { target: { value: "gaming" } });
    expect(onchangepofile).toHaveBeenCalledWith("gaming");
  });
});
