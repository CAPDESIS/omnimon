# Frontend Optimization Report (OmniMon v4)

## Scope

- Domain isolation for implementation changes: `v4/apps/desktop/src/**/*.svelte` and `v4/apps/desktop/src/**/*.ts`
- No Git commands used
- Focus: UX correctness, Svelte reactivity/render smoothing, TypeScript diagnostics

## Applied Fixes

### 1) Critical UX: Network traffic chart live updates + real timeline

Updated `v4/apps/desktop/src/components/NetworkMap.svelte`:

- Added reactive traffic-series synchronization effect that pushes new RX/TX points whenever `$metricsHistory` updates.
- Fixed static-init behavior by storing chart/series handles and continuously updating them after chart creation.
- Replaced synthetic time axis (`now - i * interval`) with real `snapshot.time` values from metrics history.
- Added reset logic for rolling-buffer shifts and history discontinuities (including sleep/wake gaps).
- Added cleanup for chart instance and `ResizeObserver` on collapse/theme recreation to avoid stale observers.

### 2) Process table render/reactivity optimization

Updated `v4/apps/desktop/src/components/ProcessTable.svelte`:

- Added `processByPid` derived map to decouple row identity from full object cloning.
- Added cached `sortedPids` derivation with snapshot-based memoization to skip repeated full `O(n log n)` resort when sort inputs are unchanged.
- Changed flat virtual rows to PID-based rows (instead of embedding full process objects) and resolve process data lazily at render time.
- Updated rank-change tracking to consume sorted PID order directly.
- Reduced duplicate per-cell work in `detail` and `group` columns by memoizing computed values with `{@const ...}`.

### 3) TypeScript / diagnostics hardening (svelte-check zero errors)

Updated `v4/apps/desktop/src` code and tests:

- `SystemDashboard.svelte`: fixed canvas context nullability by stabilizing a non-null context reference.
- `stores/preferences.ts`: aligned plugin-store load options with required typing (`defaults` included).
- `lib/i18n.ts`: removed fragile JSON default import assumption; added robust locale module normalization.
- Test files: removed `require(...)` usage and switched to typed imports from `svelte/store`.
- Test typing fixes for strict mode:
  - Added explicit writable generics where needed (`ProcessEntry[]`, `SystemStats | null`, etc.).
  - Added `BehaviorIndicator` typing in AI insight tests.
  - Fixed theme test key typing (`keyof ThemeTokens`) to remove unsafe record cast.
- Added compatibility declarations:
  - `v4/apps/desktop/src/types/compat-shims.d.ts`
  - Shims for `@vitest/utils/display` and `@testing-library/svelte-core` type resolution.

## Validation

Command executed:

- `npm exec -- svelte-check`

Result:

- `svelte-check found 0 errors and 0 warnings`
