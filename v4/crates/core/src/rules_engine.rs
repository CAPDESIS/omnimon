use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::sync::{OnceLock, RwLock};

pub const AI_RULES_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleKind {
    ProcessCountry,
    ProcessIp,
    ProcessCidr,
    ProcessPort,
    ProcessMemory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleProtocol {
    Any,
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub kind: RuleKind,
    pub process_contains: Option<String>,
    pub country_code: Option<String>,
    pub destination_ip: Option<String>,
    pub destination_cidr: Option<String>,
    pub destination_port: Option<u16>,
    pub protocol: Option<RuleProtocol>,
    pub process_memory_mb_gt: Option<u64>,
    pub mitre_technique_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiRulesPayload {
    pub schema_version: u32,
    pub rules: Vec<AlertRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoIpCidr {
    pub cidr: String,
    pub country_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicAlert {
    pub rule_id: String,
    pub rule_name: String,
    pub pid: u32,
    pub process_name: String,
    pub dst_ip: String,
    pub dst_port: u16,
    pub country_code: Option<String>,
    pub mitre_technique_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProcessRuntime {
    pub pid: u32,
    pub process_name: String,
    pub memory_bytes: u64,
}

#[derive(Debug, Clone)]
struct ParsedCidr {
    network: u32,
    mask: u32,
    prefix: u8,
}

impl ParsedCidr {
    fn parse(cidr: &str) -> Option<Self> {
        let (ip_part, prefix_part) = cidr.split_once('/')?;
        let ip = Ipv4Addr::from_str(ip_part).ok()?;
        let prefix = prefix_part.parse::<u8>().ok()?;
        if prefix > 32 {
            return None;
        }
        let mask = if prefix == 0 {
            0
        } else {
            u32::MAX << (32 - u32::from(prefix))
        };
        Some(Self {
            network: u32::from(ip) & mask,
            mask,
            prefix,
        })
    }

    fn contains_ip_str(&self, ip: &str) -> bool {
        let Ok(parsed) = Ipv4Addr::from_str(ip) else {
            return false;
        };
        if self.prefix == 0 {
            return true;
        }
        (u32::from(parsed) & self.mask) == self.network
    }
}

#[derive(Debug, Clone)]
struct ParsedGeoCidr {
    cidr: ParsedCidr,
    country_code: String,
}

impl ParsedGeoCidr {
    fn parse(raw: &GeoIpCidr) -> Option<Self> {
        Some(Self {
            cidr: ParsedCidr::parse(&raw.cidr)?,
            country_code: raw.country_code.to_ascii_uppercase(),
        })
    }

    fn contains(&self, ip: &str) -> bool {
        self.cidr.contains_ip_str(ip)
    }
}

#[derive(Default)]
struct RulesState {
    rules: Vec<AlertRule>,
    geo_db: Vec<ParsedGeoCidr>,
}

static RULES_STATE: OnceLock<RwLock<RulesState>> = OnceLock::new();

fn state() -> &'static RwLock<RulesState> {
    RULES_STATE.get_or_init(|| {
        RwLock::new(RulesState {
            rules: Vec::new(),
            geo_db: default_geo_db(),
        })
    })
}

pub fn ai_rules_schema_json() -> String {
    serde_json::json!({
        "schema_version": AI_RULES_SCHEMA_VERSION,
        "type": "object",
        "required": ["schema_version", "rules"],
        "properties": {
            "schema_version": {"type": "integer", "const": AI_RULES_SCHEMA_VERSION},
            "rules": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["id", "name", "enabled", "kind"],
                    "properties": {
                        "id": {"type": "string"},
                        "name": {"type": "string"},
                        "enabled": {"type": "boolean"},
                        "kind": {"enum": ["process_country", "process_ip", "process_cidr", "process_port", "process_memory"]},
                        "process_contains": {"type": ["string", "null"]},
                        "country_code": {"type": ["string", "null"], "description": "ISO-3166 alpha-2"},
                        "destination_ip": {"type": ["string", "null"]},
                        "destination_cidr": {"type": ["string", "null"], "description": "IPv4 CIDR, e.g. 36.0.0.0/8"},
                        "destination_port": {"type": ["integer", "null"], "minimum": 1, "maximum": 65535},
                        "protocol": {"enum": ["any", "tcp", "udp", null]},
                        "process_memory_mb_gt": {"type": ["integer", "null"]},
                        "mitre_technique_id": {"type": ["string", "null"]}
                    }
                }
            }
        }
    })
    .to_string()
}

pub fn upsert_rules_from_ai_json(payload_json: &str) -> Result<usize, String> {
    let payload: AiRulesPayload = serde_json::from_str(payload_json)
        .map_err(|e| format!("invalid rules payload JSON: {e}"))?;

    if payload.schema_version != AI_RULES_SCHEMA_VERSION {
        return Err(format!(
            "unsupported schema_version {}, expected {}",
            payload.schema_version, AI_RULES_SCHEMA_VERSION
        ));
    }

    let mut guard = state()
        .write()
        .map_err(|_| "rules lock poisoned".to_string())?;
    guard.rules = payload.rules;
    Ok(guard.rules.len())
}

pub fn replace_geoip_db_from_json(payload_json: &str) -> Result<usize, String> {
    let rows: Vec<GeoIpCidr> = serde_json::from_str(payload_json)
        .map_err(|e| format!("invalid GeoIP JSON payload: {e}"))?;

    let parsed = rows
        .iter()
        .filter_map(ParsedGeoCidr::parse)
        .collect::<Vec<_>>();

    let mut guard = state()
        .write()
        .map_err(|_| "rules lock poisoned".to_string())?;
    guard.geo_db = parsed;
    Ok(guard.geo_db.len())
}

pub fn active_rules() -> Vec<AlertRule> {
    state().read().map(|g| g.rules.clone()).unwrap_or_default()
}

pub fn evaluate_events(
    events: &[crate::network::ProcessConnectionEvent],
    runtime: &[ProcessRuntime],
) -> Vec<DynamicAlert> {
    let Ok(guard) = state().read() else {
        return Vec::new();
    };

    let runtime_by_pid = runtime
        .iter()
        .map(|r| (r.pid, r))
        .collect::<HashMap<u32, &ProcessRuntime>>();

    let mut alerts = Vec::new();
    for event in events {
        let process = runtime_by_pid.get(&event.pid).copied();
        let process_name = process
            .map(|r| r.process_name.clone())
            .unwrap_or_else(|| format!("pid-{}", event.pid));
        let process_memory_mb = process
            .map(|r| r.memory_bytes / 1_048_576)
            .unwrap_or_default();

        let country = country_for_ip(&guard.geo_db, &event.dst_ip);

        for rule in guard.rules.iter().filter(|r| r.enabled) {
            if !matches_process(rule, &process_name) {
                continue;
            }

            if !matches_protocol(rule, event.protocol) {
                continue;
            }

            let matched = match rule.kind {
                RuleKind::ProcessCountry => rule
                    .country_code
                    .as_ref()
                    .map(|c| country.as_deref() == Some(&c.to_ascii_uppercase()))
                    .unwrap_or(false),
                RuleKind::ProcessIp => rule
                    .destination_ip
                    .as_ref()
                    .map(|ip| ip == &event.dst_ip)
                    .unwrap_or(false),
                RuleKind::ProcessCidr => rule
                    .destination_cidr
                    .as_ref()
                    .and_then(|c| ParsedCidr::parse(c))
                    .map(|cidr| cidr.contains_ip_str(&event.dst_ip))
                    .unwrap_or(false),
                RuleKind::ProcessPort => rule.destination_port == Some(event.dst_port),
                RuleKind::ProcessMemory => rule
                    .process_memory_mb_gt
                    .map(|threshold| process_memory_mb > threshold)
                    .unwrap_or(false),
            };

            if !matched {
                continue;
            }

            alerts.push(DynamicAlert {
                rule_id: rule.id.clone(),
                rule_name: rule.name.clone(),
                pid: event.pid,
                process_name: process_name.clone(),
                dst_ip: event.dst_ip.clone(),
                dst_port: event.dst_port,
                country_code: country.clone(),
                mitre_technique_id: rule
                    .mitre_technique_id
                    .clone()
                    .unwrap_or_else(|| "T1571".to_string()),
                message: format!(
                    "Rule '{}' matched: {} -> {}:{} (mem={}MB)",
                    rule.name, process_name, event.dst_ip, event.dst_port, process_memory_mb
                ),
            });
        }
    }

    alerts
}

fn matches_process(rule: &AlertRule, process_name: &str) -> bool {
    match rule.process_contains.as_ref() {
        Some(needle) => process_name
            .to_ascii_lowercase()
            .contains(&needle.to_ascii_lowercase()),
        None => true,
    }
}

fn matches_protocol(rule: &AlertRule, protocol: crate::network::TransportProtocol) -> bool {
    match rule.protocol.as_ref().unwrap_or(&RuleProtocol::Any) {
        RuleProtocol::Any => true,
        RuleProtocol::Tcp => protocol == crate::network::TransportProtocol::Tcp,
        RuleProtocol::Udp => protocol == crate::network::TransportProtocol::Udp,
    }
}

fn country_for_ip(geo_db: &[ParsedGeoCidr], ip: &str) -> Option<String> {
    geo_db
        .iter()
        .find(|entry| entry.contains(ip))
        .map(|entry| entry.country_code.clone())
}

fn default_geo_db() -> Vec<ParsedGeoCidr> {
    [
        GeoIpCidr {
            cidr: "36.0.0.0/8".to_string(),
            country_code: "CN".to_string(),
        },
        GeoIpCidr {
            cidr: "39.0.0.0/8".to_string(),
            country_code: "CN".to_string(),
        },
        GeoIpCidr {
            cidr: "8.8.8.0/24".to_string(),
            country_code: "US".to_string(),
        },
    ]
    .iter()
    .filter_map(ParsedGeoCidr::parse)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_json_is_versioned() {
        let schema = ai_rules_schema_json();
        assert!(schema.contains("schema_version"));
        assert!(schema.contains("process_cidr"));
    }

    #[test]
    fn ai_rules_are_loaded_and_matched() {
        let payload = r#"{"schema_version":1,"rules":[{"id":"r1","name":"CN process","enabled":true,"kind":"process_country","process_contains":"chrome","country_code":"CN","destination_ip":null,"destination_cidr":null,"destination_port":null,"protocol":"tcp","process_memory_mb_gt":null,"mitre_technique_id":"T1571"}]}"#;
        let count = upsert_rules_from_ai_json(payload).expect("load rules");
        assert_eq!(count, 1);

        let events = vec![crate::network::ProcessConnectionEvent {
            pid: 77,
            protocol: crate::network::TransportProtocol::Tcp,
            direction: crate::network::TrafficDirection::Outbound,
            src_ip: "10.0.0.4".to_string(),
            dst_ip: "36.1.2.3".to_string(),
            src_port: 53000,
            dst_port: 443,
            bytes: 100,
        }];
        let runtime = vec![ProcessRuntime {
            pid: 77,
            process_name: "chrome renderer".to_string(),
            memory_bytes: 600 * 1_048_576,
        }];
        let alerts = evaluate_events(&events, &runtime);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].country_code.as_deref(), Some("CN"));
    }

    #[test]
    fn memory_rule_matches_threshold() {
        let payload = r#"{"schema_version":1,"rules":[{"id":"r2","name":"node high mem","enabled":true,"kind":"process_memory","process_contains":"node","country_code":null,"destination_ip":null,"destination_cidr":null,"destination_port":null,"protocol":"any","process_memory_mb_gt":1024,"mitre_technique_id":"T1499"}]}"#;
        let _ = upsert_rules_from_ai_json(payload).expect("load memory rule");
        let events = vec![crate::network::ProcessConnectionEvent {
            pid: 991,
            protocol: crate::network::TransportProtocol::Tcp,
            direction: crate::network::TrafficDirection::Outbound,
            src_ip: "10.1.0.8".to_string(),
            dst_ip: "8.8.8.8".to_string(),
            src_port: 51234,
            dst_port: 443,
            bytes: 200,
        }];
        let runtime = vec![ProcessRuntime {
            pid: 991,
            process_name: "node".to_string(),
            memory_bytes: 1_500 * 1_048_576,
        }];
        let alerts = evaluate_events(&events, &runtime);
        assert_eq!(alerts.len(), 1);
    }
}
