# E2E tests (Playwright + Vite)

This suite runs the desktop frontend in standalone mode through Vite and mocks the Tauri APIs that are normally provided by the host shell.

## Run

```bash
bun run test:e2e
```

If you want the HTML report after execution:

```bash
bunx playwright show-report
```

## Notes

- Base URL: `http://localhost:1420`
- Browser target: Chromium only
- The mocked fixture covers metrics, browser tabs, network data, store persistence, and AI missing-key failures
- Playwright browsers are not installed automatically in CI yet
