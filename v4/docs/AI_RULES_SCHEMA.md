# AI Rules Schema v1

This document defines the JSON contract consumed by:

- Core parser: `upsert_rules_from_ai_json` in `v4/crates/core/src/rules_engine.rs`
- Tauri IPC command: `apply_ai_rules(payload: String)` in `v4/apps/desktop/src-tauri/src/lib.rs`

## Versioning

- Field: `schema_version`
- Current value: `1`
- Behavior: payload is rejected if `schema_version` does not match.

## Top-level structure

```json
{
  "schema_version": 1,
  "rules": [
    {
      "id": "string",
      "name": "string",
      "enabled": true,
      "kind": "process_country | process_ip | process_cidr | process_port | process_memory",
      "process_contains": "string|null",
      "country_code": "string|null",
      "destination_ip": "string|null",
      "destination_cidr": "string|null",
      "destination_port": 443,
      "protocol": "any|tcp|udp|null",
      "process_memory_mb_gt": 1024,
      "mitre_technique_id": "string|null"
    }
  ]
}
```

## Rule kinds

- `process_country`: matches destination IP country from local GeoIP DB (`country_code`).
- `process_ip`: matches an exact destination IP (`destination_ip`).
- `process_cidr`: matches destination IPv4 CIDR (`destination_cidr`, e.g. `103.27.202.0/24`).
- `process_port`: matches destination port (`destination_port`).
- `process_memory`: matches process memory threshold (`process_memory_mb_gt`).

### Optional narrowing fields

- `process_contains`: case-insensitive process name filter.
- `protocol`: `any` (default), `tcp`, or `udp`.
- `mitre_technique_id`: custom MITRE technique ID attached to the generated alert.

## Examples

### 1) GeoIP rule ("avísame si Chrome se conecta a China")

```json
{
  "schema_version": 1,
  "rules": [
    {
      "id": "geo-cn-001",
      "name": "Alert Chrome to CN",
      "enabled": true,
      "kind": "process_country",
      "process_contains": "chrome",
      "country_code": "CN",
      "destination_ip": null,
      "destination_cidr": null,
      "destination_port": null,
      "protocol": "tcp",
      "process_memory_mb_gt": null,
      "mitre_technique_id": "T1571"
    }
  ]
}
```

### 2) CIDR block rule

```json
{
  "schema_version": 1,
  "rules": [
    {
      "id": "cidr-block-002",
      "name": "Suspicious CIDR",
      "enabled": true,
      "kind": "process_cidr",
      "process_contains": null,
      "country_code": null,
      "destination_ip": null,
      "destination_cidr": "103.27.202.0/24",
      "destination_port": null,
      "protocol": "any",
      "process_memory_mb_gt": null,
      "mitre_technique_id": "T1043"
    }
  ]
}
```

### 3) Port + process rule

```json
{
  "schema_version": 1,
  "rules": [
    {
      "id": "port-rule-003",
      "name": "Node unusual outbound port",
      "enabled": true,
      "kind": "process_port",
      "process_contains": "node",
      "country_code": null,
      "destination_ip": null,
      "destination_cidr": null,
      "destination_port": 4444,
      "protocol": "tcp",
      "process_memory_mb_gt": null,
      "mitre_technique_id": "T1571"
    }
  ]
}
```

### 4) Process memory rule

```json
{
  "schema_version": 1,
  "rules": [
    {
      "id": "proc-mem-004",
      "name": "Alert if node > 1GB",
      "enabled": true,
      "kind": "process_memory",
      "process_contains": "node",
      "country_code": null,
      "destination_ip": null,
      "destination_cidr": null,
      "destination_port": null,
      "protocol": "any",
      "process_memory_mb_gt": 1024,
      "mitre_technique_id": "T1499"
    }
  ]
}
```

## Frontend integration notes

- Apply rules: `invoke("apply_ai_rules", { payload })`
- Read contract dynamically: `invoke("get_ai_rules_schema")`
- Listen alerts: `listen("security-alert", ...)` (payload is `DynamicAlert`)
