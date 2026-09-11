# OmniMon CVE Tracking Report

## CVE-2026-25727 — time crate (historical 0.3.45)

### Status: **Mitigated in `v4/Cargo.lock`**

### Summary
- **Historical pin:** `time = "0.3.45"` via older `mac-notification-sys`
- **Fixed version:** `time >= 0.3.47`
- **Current lockfile:** `time 0.3.54` (2026-09-11 inspection of `v4/Cargo.lock`)
- `mac-notification-sys` in the current lock is `0.6.15` (no longer the 0.3.45 exact pin)

### Dependency Chain (historical)
```
omnimon-desktop
  └── tauri-plugin-notification
       └── notify-rust
            └── mac-notification-sys (was 0.6.11 with time = "=0.3.45")
```

The 2026-03-08 note that `[patch]` could not override `=0.3.45` is historical.
The lockfile now resolves `time 0.3.54`. On 2026-09-11 `cargo audit` also flagged
`h2 0.4.15` (`RUSTSEC-2026-0258`); the lockfile was bumped to `h2 0.4.16`.
Re-run `cargo audit` in `v4/` if the advisory DB changes.

### Audit Output (lockfile, 2026-09-11)
```
[[package]]
name = "time"
version = "0.3.54"
```

---

## GHSA-5c6j-r48x-rmvq — serialize-javascript <=7.0.2

### Status: **Dev dependency only — bajo riesgo**

### Summary
- **Affected dependency:** `serialize-javascript <= 7.0.2`
- **Severity:** High (RCE via RegExp.flags y Date.prototype.toISOString)
- **Source:** https://github.com/advisories/GHSA-5c6j-r48x-rmvq

### Dependency Chain
```
@wdio/mocha-framework
  └── mocha
       └── serialize-javascript <= 7.0.2
```

### Mitigation
1. **Solo dev dependency:** `serialize-javascript` se usa exclusivamente a través
   de `@wdio/mocha-framework` (framework de testing E2E). No se incluye en el
   binario de producción.
2. **Sin exposición a usuarios finales:** El código vulnerable solo se ejecuta
   durante el desarrollo/CI en el runner de tests.
3. **Resolución:** Esperar actualización de `@wdio/mocha-framework` con versión
   parcheada de `serialize-javascript > 7.0.2`.

### Audit Output
```
$ bun audit
serialize-javascript  <=7.0.2
  @wdio/mocha-framework › mocha › serialize-javascript
  high: RCE via RegExp.flags and Date.prototype.toISOString()
1 vulnerabilities (1 high)
```

---

## Dependencias sin mantenimiento (warnings)

Los siguientes crates reportan `unmaintained` pero son dependencias transitivas
de Tauri/GTK3 y no tienen fix disponible actualmente:

| Crate | RUSTSEC | Notas |
|-------|---------|-------|
| `atk 0.18.2` | RUSTSEC-2024-0413 | GTK3 bindings (solo Linux) |
| `atk-sys 0.18.2` | RUSTSEC-2024-0416 | GTK3 sys bindings (solo Linux) |
| `fxhash 0.2.1` | — | Usado por GTK (solo Linux) |
| `unic-*` | RUSTSEC-2025-0080/0098/0100 | Usado por `urlpattern` → `tauri-utils` |

Estas dependencias se resolverán cuando Tauri migre a GTK4 y actualice
`urlpattern`. No hay CVEs de seguridad asociados, solo estado de mantenimiento.

---

*Last updated: 2026-03-08*
