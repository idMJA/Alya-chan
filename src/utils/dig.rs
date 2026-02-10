use crate::utils::{dns, embed, table};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use twilight_model::channel::message::component::{
    ActionRow, Button, ButtonStyle, Component, SelectMenu, SelectMenuOption, SelectMenuType,
};
use twilight_model::channel::message::embed::Embed;

const DNSSEC_WARN: &str = ":warning: cd bit set, DNSSEC validation disabled";

#[derive(Clone, Debug)]
pub struct Provider {
    pub name: &'static str,
    pub info: &'static str,
    pub doh: dns::Endpoint,
    pub dig: &'static str,
}

pub static PROVIDERS: &[Provider] = &[
    Provider {
        name: "1.1.1.1 (Cloudflare)",
        info: "https://developers.cloudflare.com/1.1.1.1/",
        doh: dns::Endpoint {
            endpoint: "https://cloudflare-dns.com/dns-query",
            mode: dns::EndpointMode::Json,
        },
        dig: "1.1.1.1",
    },
    Provider {
        name: "1.1.1.2 (Cloudflare Malware Blocking)",
        info: "https://developers.cloudflare.com/1.1.1.1/setup/#1111-for-families",
        doh: dns::Endpoint {
            endpoint: "https://1.1.1.2/dns-query",
            mode: dns::EndpointMode::Json,
        },
        dig: "1.1.1.2",
    },
    Provider {
        name: "1.1.1.3 (Cloudflare Malware + Adult Content Blocking)",
        info: "https://developers.cloudflare.com/1.1.1.1/setup/#1111-for-families",
        doh: dns::Endpoint {
            endpoint: "https://1.1.1.3/dns-query",
            mode: dns::EndpointMode::Json,
        },
        dig: "1.1.1.3",
    },
    Provider {
        name: "8.8.8.8 (Google)",
        info: "https://developers.google.com/speed/public-dns",
        doh: dns::Endpoint {
            endpoint: "https://dns.google/resolve",
            mode: dns::EndpointMode::Json,
        },
        dig: "8.8.8.8",
    },
    Provider {
        name: "9.9.9.9 (Quad9)",
        info: "https://www.quad9.net/",
        doh: dns::Endpoint {
            endpoint: "https://dns.quad9.net/dns-query",
            mode: dns::EndpointMode::Dns,
        },
        dig: "9.9.9.9",
    },
];

pub const VALID_TYPES: &[&str] = &[
    "A", "AAAA", "CAA", "CERT", "CNAME", "MX", "NS", "SPF", "SRV", "TXT", "DNSKEY", "DS", "LOC",
    "URI", "HTTPS", "NAPTR", "PTR", "SMIMEA", "SOA", "SSHFP", "SVCB", "TLSA", "HINFO", "CDS",
    "CDNSKEY",
];

pub const ALL_TYPES: &[&str] = &[
    "A",
    "AAAA",
    "AFSDB",
    "APL",
    "CAA",
    "CDNSKEY",
    "CDS",
    "CERT",
    "CNAME",
    "CSYNC",
    "DHCID",
    "DLV",
    "DNAME",
    "DNSKEY",
    "DS",
    "EUI48",
    "EUI64",
    "HINFO",
    "HIP",
    "HTTPS",
    "IPSECKEY",
    "KEY",
    "KX",
    "LOC",
    "MX",
    "NAPTR",
    "NS",
    "NSEC",
    "NSEC3",
    "NSEC3PARAM",
    "OPENPGPKEY",
    "PTR",
    "RP",
    "SMIMEA",
    "SOA",
    "SPF",
    "SRV",
    "SSHFP",
    "SVCB",
    "TA",
    "TKEY",
    "TLSA",
    "TXT",
    "URI",
    "ZONEMD",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Req {
    pub domain: String,
    pub types: Vec<String>,
    pub short: bool,
    pub cdflag: bool,
    pub provider: String,
}

struct Entry {
    exp: Instant,
    req: Req,
}

static STORE: Lazy<RwLock<HashMap<String, Entry>>> = Lazy::new(|| RwLock::new(HashMap::new()));

pub fn dom(input: &str) -> Option<String> {
    let raw = input.trim();
    if raw.is_empty() {
        return None;
    }
    let stripped = if raw.contains("://") {
        url::Url::parse(raw)
            .ok()
            .and_then(|u| u.host_str().map(|s| s.to_string()))
    } else {
        Some(raw.to_string())
    }?;
    let without_path = stripped
        .split('/')
        .next()
        .unwrap_or("")
        .trim()
        .trim_end_matches('.');
    let without_port = without_path.split(':').next().unwrap_or("");
    if without_port.is_empty() {
        None
    } else {
        Some(without_port.to_lowercase())
    }
}

pub fn prov(name: &str) -> Option<&'static Provider> {
    PROVIDERS.iter().find(|p| p.name == name)
}

pub fn types(raw: Option<&str>) -> Vec<String> {
    match raw.map(|r| r.trim()) {
        Some("*") => ALL_TYPES.iter().map(|t| t.to_string()).collect(),
        Some(v) if !v.is_empty() => v
            .split_whitespace()
            .map(|t| t.trim().to_uppercase())
            .filter(|t| !t.is_empty())
            .collect(),
        _ => vec!["A".to_string()],
    }
}

pub async fn put(req: Req) -> String {
    let key = uuid::Uuid::new_v4().simple().to_string();
    let mut map = STORE.write().await;
    map.insert(
        key.clone(),
        Entry {
            exp: Instant::now() + Duration::from_secs(600),
            req,
        },
    );
    key
}

pub async fn set_provider(key: &str, provider: &str) {
    let mut map = STORE.write().await;
    if let Some(entry) = map.get_mut(key) {
        entry.req.provider = provider.to_string();
    }
}

pub async fn run(key: &str) -> Option<(Vec<Embed>, Vec<Component>)> {
    let req = {
        let mut map = STORE.write().await;
        if let Some(entry) = map.get(key) {
            if entry.exp > Instant::now() {
                entry.req.clone()
            } else {
                map.remove(key);
                return None;
            }
        } else {
            return None;
        }
    };

    let provider = prov(&req.provider)?;
    let mut embeds = Vec::new();

    for t in &req.types {
        let lookup =
            dns::lookup(&req.domain, t, &provider.doh, dns::Flags { cd: req.cdflag }).await;

        let desc = present(&req.domain, t, provider, &lookup, req.short, req.cdflag);
        embeds.push(embed::make(
            &format!("{} records", t),
            &desc,
            Some("diggy diggy hole"),
        ));
    }

    let components = components(key, provider.name);
    Some((embeds, components))
}

fn present(
    domain: &str,
    t: &str,
    p: &Provider,
    data: &dns::Lookup,
    short: bool,
    cdflag: bool,
) -> String {
    let mut parts = vec![
        domain.to_string(),
        t.to_string(),
        format!("@{}", p.dig),
        "+noall".to_string(),
        "+answer".to_string(),
    ];
    if short {
        parts.push("+short".to_string());
    }
    if cdflag {
        parts.push("+cdflag".to_string());
    }

    let dig_cmd = format!("`{}`", parts.join(" "));

    if let Some(msg) = &data.message {
        return format!("{}\n{}", dig_cmd, msg);
    }

    if data.answer.is_empty() {
        return format!(
            "{}\nNo records found{}",
            dig_cmd,
            if data.flags.cd {
                format!("\n\n{DNSSEC_WARN}")
            } else {
                String::new()
            }
        );
    }

    let source_rows: Vec<String> = if short {
        data.answer.iter().map(|x| x.data.clone()).collect()
    } else {
        data.answer
            .iter()
            .map(|x| format!("{}\n{}\n{}", x.name, x.ttl, x.data))
            .collect()
    };

    let output = |rows: &[String]| -> String {
        let trunc = source_rows.len().saturating_sub(rows.len());
        let trunc_str = if trunc > 0 {
            format!(
                "\n...({} row{} truncated)",
                trunc,
                if trunc == 1 { "" } else { "s" }
            )
        } else {
            String::new()
        };

        let rows_str = if short {
            rows.join("\n")
        } else {
            let mut table_rows = Vec::with_capacity(rows.len() + 1);
            table_rows.push(vec![
                "NAME".to_string(),
                "TTL".to_string(),
                "DATA".to_string(),
            ]);
            for row in rows {
                table_rows.push(row.split('\n').map(|s| s.to_string()).collect::<Vec<_>>());
            }
            table::present_table(&table_rows)
        };

        format!("{}\n```\n{}{}\n```", dig_cmd, rows_str, trunc_str)
    };

    let max_len = 4096 - if data.flags.cd { DNSSEC_WARN.len() } else { 0 };

    if short {
        let mut out = Vec::new();
        for row in source_rows.iter() {
            let test = output(&[out.clone(), vec![row.clone()]].concat());
            if test.len() > max_len {
                break;
            }
            out.push(row.clone());
        }
        let mut result = output(&out);
        if data.flags.cd {
            result.push_str(&format!("\n{DNSSEC_WARN}"));
        }
        return result;
    }

    let mut out = Vec::new();
    for row in data.answer.iter() {
        let row = format!("{}\n{}\n{}", row.name, row.ttl, row.data);
        let test = output(&[out.clone(), vec![row.clone()]].concat());
        if test.len() > max_len {
            break;
        }
        out.push(row);
    }

    let mut result = output(&out);
    if data.flags.cd {
        result.push_str(&format!("\n{DNSSEC_WARN}"));
    }
    result
}

fn components(key: &str, provider: &str) -> Vec<Component> {
    let options = PROVIDERS
        .iter()
        .map(|p| SelectMenuOption {
            default: p.name == provider,
            emoji: None,
            description: None,
            label: p.name.to_string(),
            value: p.name.to_string(),
        })
        .collect::<Vec<_>>();

    let select = SelectMenu {
        id: None,
        channel_types: None,
        custom_id: format!("dig_provider:{}", key),
        default_values: None,
        disabled: false,
        kind: SelectMenuType::Text,
        max_values: Some(1),
        min_values: Some(1),
        options: Some(options),
        placeholder: Some("Select DNS provider".to_string()),
        required: None,
    };

    let refresh = Button {
        id: None,
        custom_id: Some(format!("dig_refresh:{}", key)),
        label: Some("Refresh".to_string()),
        style: ButtonStyle::Secondary,
        disabled: false,
        emoji: None,
        url: None,
        sku_id: None,
    };

    vec![
        Component::ActionRow(ActionRow {
            id: None,
            components: vec![Component::SelectMenu(select)],
        }),
        Component::ActionRow(ActionRow {
            id: None,
            components: vec![Component::Button(refresh)],
        }),
    ]
}
