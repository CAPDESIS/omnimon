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

- Base URL: `http://localhost:1420` by default; set `OMNIMON_E2E_PORT` to use another port
- A server already listening on that port is never reused (it could be another checkout or a running `tauri dev`, and the suite would pass on a build that is not under test): the run fails instead. Set `OMNIMON_E2E_REUSE=1` to opt in on purpose
- Browser target: Chromium only
- The mocked fixture covers metrics, browser tabs, network data, store persistence, and AI missing-key failures
- Playwright browsers are not installed automatically in CI yet
