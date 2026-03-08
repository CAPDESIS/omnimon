# Rust Performance Profiling — v5.2.0+

## 1. Custom Memory Allocator: mimalloc

### Decisión
Se integra **mimalloc** (Microsoft) como `#[global_allocator]` en ambos binarios:
- `v4/crates/cli/src/main.rs` (CLI)
- `v4/apps/desktop/src-tauri/src/main.rs` (Desktop/Tauri)

### Justificación sobre jemalloc
| Criterio | mimalloc | jemalloc |
|---|---|---|
| macOS soporte | Nativo, sin conflictos | Problemas conocidos con macOS Zone allocator |
| Fragmentación | ~1.3x menos que glibc | Similar |
| Throughput (small allocs) | ~7% más rápido | ~5% más rápido |
| Tamaño binario | ~50KB | ~250KB |
| Mantenimiento | Activo (Microsoft) | Activo (Meta) |

mimalloc es superior para nuestro caso: demonio macOS 24/7 con miles de asignaciones pequeñas (Strings de nombres de proceso) cada 2 segundos.

### Impacto esperado
- **Reducción de fragmentación**: mimalloc usa thread-local free lists con page-level decommit, evitando la fragmentación progresiva que sufre el allocator del sistema en procesos long-running.
- **Throughput**: ~7% mejora en throughput de small allocations (benchmarks publicados por Microsoft).
- **RSS estable**: el memory footprint no crece monótonamente como con el allocator del sistema.

---

## 2. Fuzz Testing

### Infraestructura
Directorio: `v4/crates/core/fuzz/`

Herramienta: `cargo-fuzz` (libFuzzer backend via `libfuzzer-sys`).

### Targets

| Target | Archivo | Superficie de ataque |
|---|---|---|
| `fuzz_rules_payload` | `fuzz_targets/fuzz_rules_payload.rs` | JSON malformado en `upsert_rules_from_ai_json()` — parser de reglas AI |
| `fuzz_geoip_payload` | `fuzz_targets/fuzz_geoip_payload.rs` | JSON malformado en `replace_geoip_db_from_json()` — parser GeoIP con CIDR |
| `fuzz_rules_evaluate` | `fuzz_targets/fuzz_rules_evaluate.rs` | Pipeline completo: carga de reglas + eventos de conexión → evaluación |

### Ejecución
```bash
cd v4/crates/core
cargo +nightly fuzz run fuzz_rules_payload -- -max_total_time=300
cargo +nightly fuzz run fuzz_geoip_payload -- -max_total_time=300
cargo +nightly fuzz run fuzz_rules_evaluate -- -max_total_time=300
```

### Cobertura de riesgo
- **JSON injection desde frontend**: El frontend envía payloads JSON via IPC (`apply_ai_rules`). Un payload malformado no debe causar `panic!`.
- **CIDR parsing**: Prefijos IPv4 inválidos (>32, negativos, sin `/`).
- **Temporal correlation**: Reglas encadenadas con `within_seconds` extremos.

---

## 3. Zero-Allocation Hot Path

### Problema
El watcher tick corre cada 2 segundos y antes de esta optimización creaba:
- 1× `Vec<CachedProcessInfo>` (~500 elementos) — fresh allocation
- 1× `HashMap<u32, (u64, u64)>` — fresh allocation para throughput merge
- 1× `Vec<ProcessRuntime>` (~500 elementos) — fresh allocation para rules engine

Cada uno de estos contenedores se aloca en el heap, se llena, se consume, y se destruye. En un demonio 24/7, esto genera ~43,200 ciclos de alloc/dealloc por día solo para estos tres contenedores.

### Solución: `WatcherBuffers`
```rust
struct WatcherBuffers {
    process_info: Vec<CachedProcessInfo>,    // Pre-allocated, cleared each tick
    throughput_map: HashMap<u32, (u64, u64)>, // Pre-allocated, cleared each tick
    runtime: Vec<crate::rules_engine::ProcessRuntime>, // Pre-allocated, cleared each tick
    last_process_count: usize,               // Capacity hint from previous tick
}
```

### Patrón de reutilización
1. **`clear()` retains capacity**: `Vec::clear()` y `HashMap::clear()` no liberan la memoria del heap. Solo resetean el length/count a 0.
2. **Capacity hint**: `last_process_count` alimenta `reserve()` para que el buffer nunca necesite realocar después del warm-up.
3. **Buffer swap**: Después de escribir el snapshot al cache, recuperamos el `Vec<CachedProcessInfo>` del estado anterior via `std::mem::take()` para reutilizar su capacidad en el siguiente tick.

### Impacto
- **Warm-up**: Después de ~3 ticks, todos los buffers alcanzan capacidad estable.
- **Steady-state**: 0 heap allocations para Vec/HashMap containers en el hot path.
- **String allocations**: Las conversiones `OsStr → String` para nombres de proceso siguen siendo necesarias (vienen de sysinfo), pero mimalloc las maneja eficientemente.
- **Backend label**: Se usa `const BACKEND_UNKNOWN: &str` en lugar de `"Unknown".to_string()` para el caso default.
