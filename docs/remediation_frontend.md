# Frontend Remediation Plan (Track Frontend)

## Scope

This remediation is restricted to frontend sources (`apps/desktop/src`) plus this planning note.
No Rust, Cargo, CI, or repo configuration files are modified.

## 1) Memory leak hardening for Tauri `listen()` promises

### Problem

`listen()` is async and returns an unlisten function later. If a component unmounts before the promise resolves, the listener can remain active and leak.

### Approach

- In `App.svelte`, replace ad-hoc `let unlistenX` assignment with a centralized registration helper.
- Track `disposed` state.
- When a `listen()` promise resolves after dispose, invoke unlisten immediately.
- Ensure cleanup always runs all registered unlisteners.
- Also force cleanup of temporary `window` drag listeners on unmount.

## 2) Race-condition hardening for CloudSync

### Problem

`CloudSync` allows overlapping `invoke()` calls (double clicks / repeated actions), with no loading guards.

### Approach

- Add explicit UI states: `loadingKey`, `savingKey`.
- Prevent concurrent save while loading/saving.
- Use a mounted/disposed guard to avoid state updates after unmount.
- Add typed `invoke<T>()` usage for safer payload handling.
- Disable controls while active operations are in flight.

## 3) Type drift alignment (TypeScript contract)

### Problem

Frontend TypeScript drifted from backend runtime payloads (e.g., new behavior indicators, optional context, temporal correlation fields in AI rules).

### Approach

- Update `types.ts` to include:
  - `BehaviorIndicator::SuspiciousNetworkConnection`
  - optional `context` in `ProcessThreatLabel`
  - `temporal_correlation` in `AiRuleV1`
- Update `aiConfigBridge.ts` parsing/prompt contract so generated AI rules include temporal correlation shape.

## Validation

After edits, run frontend checks:

- `npm test`
- `npm run build`

Goal: zero regressions in tests/build and no leaked listeners from unresolved unlisten promises.
