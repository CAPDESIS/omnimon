describe("OmniMon critical flows", () => {
  it("shows native confirmation when closing selected tabs", async function () {
    const tabCheckboxes = await $$(".chrome-manager .tab-row input[type='checkbox']");

    if (tabCheckboxes.length === 0) {
      this.skip();
      return;
    }

    await tabCheckboxes[0].click();

    const closeSelectedButton = await $(".chrome-manager .btn-close-selected");
    await closeSelectedButton.waitForExist({ timeout: 5000 });
    await closeSelectedButton.click();

    await browser.waitUntil(async () => browser.isAlertOpen(), {
      timeout: 5000,
      timeoutMsg: "expected native confirmation dialog to open",
    });

    const alertText = await browser.getAlertText();
    expect(alertText.length).toBeGreaterThan(0);

    await browser.dismissAlert();
  });

  it("changes language to Spanish from Settings", async () => {
    const settingsButton = await $("button[title='AI Settings']");
    await settingsButton.waitForClickable({ timeout: 10000 });
    await settingsButton.click();

    const localeSelect = await $("#locale-select");
    await localeSelect.waitForExist({ timeout: 10000 });
    await localeSelect.selectByAttribute("value", "es");

    const footer = await $("footer.statusline");
    await browser.waitUntil(
      async () => {
        const txt = await footer.getText();
        return txt.toLowerCase().includes("procesos");
      },
      {
        timeout: 10000,
        timeoutMsg: "expected UI text to switch to Spanish",
      },
    );
  });
});
