use crate::utils::{embed, table};
use chrono::Utc;
use serde_json::Value;
use std::collections::HashSet;
use twilight_model::channel::message::embed::Embed;

pub async fn who(q: &str) -> Result<Embed, String> {
    let query = norm(q);
    if query.is_empty() {
        return Err("The query does not appear to be a valid domain name, IP address or ASN, or no results could be found".to_string());
    }

    let rdap_result = lookup_rdap(&query).await;

    let whois_result = if rdap_result.is_none() {
        lookup_whois(&query).await
    } else {
        None
    };

    let cfwho_result = if rdap_result.is_none() && whois_result.is_none() {
        lookup_cfwho(&query).await
    } else {
        None
    };

    let fields = combine_results(vec![rdap_result, whois_result, cfwho_result]);

    if fields.is_empty() {
        return Err("The query does not appear to be a valid domain name, IP address or ASN, or no results could be found".to_string());
    }

    Ok(build(&query, &fields))
}

async fn lookup_rdap(query: &str) -> Option<Vec<(String, String)>> {
    let url = format!("https://rdap.cloud/api/v1/{}", query);
    let res = reqwest::get(&url).await.ok()?;
    if !res.status().is_success() {
        return None;
    }

    let json = res.json::<Value>().await.ok()?;

    let data = json
        .get("results")
        .and_then(|r| r.get(query))
        .and_then(|q| q.get("success"))
        .and_then(|s| if s.as_bool()? { Some(()) } else { None })
        .and_then(|_| json.get("results")?.get(query)?.get("data"))?;

    let fields = extract_rdap_fields(data);
    if fields.is_empty() {
        None
    } else {
        Some(fields)
    }
}

async fn lookup_whois(query: &str) -> Option<Vec<(String, String)>> {
    let url = format!("https://whoisjs.com/api/v1/{}", query);
    let res = reqwest::get(&url).await.ok()?;
    if !res.status().is_success() {
        return None;
    }

    let json = res.json::<Value>().await.ok()?;

    let data = json
        .get(query)
        .and_then(|q| q.get("success"))
        .and_then(|s| if s.as_bool()? { Some(()) } else { None })
        .and_then(|_| json.get(query)?.get("data"))?;

    let fields = extract_whois_fields(data);
    if fields.is_empty() {
        None
    } else {
        Some(fields)
    }
}

async fn lookup_cfwho(query: &str) -> Option<Vec<(String, String)>> {
    let url = format!("https://cfwho.com/api/v1/{}", query);
    let res = reqwest::get(&url).await.ok()?;
    if !res.status().is_success() {
        return None;
    }

    let json = res.json::<Value>().await.ok()?;

    let data = json
        .get(query)
        .and_then(|q| q.get("success"))
        .and_then(|s| if s.as_bool()? { Some(()) } else { None })
        .and_then(|_| json.get(query))?;

    let fields = extract_cfwho_fields(data);
    if fields.is_empty() {
        None
    } else {
        Some(fields)
    }
}

fn combine_results(results: Vec<Option<Vec<(String, String)>>>) -> Vec<(String, String)> {
    let mut combined = std::collections::HashMap::new();

    for result in results.into_iter().flatten() {
        for (key, value) in result {
            // Only add if key doesn't exist yet (first source wins)
            combined.entry(key).or_insert(value);
        }
    }

    let mut fields: Vec<_> = combined.into_iter().collect();
    fields.sort_by(|a, b| a.0.cmp(&b.0));
    fields
}

fn build(query: &str, fields: &[(String, String)]) -> Embed {
    let preferred_order = [
        "Registrar",
        "Registrant",
        "Registration",
        "Expiration",
        "Status",
        "Abuse",
        "Nameservers",
        "CIDR",
        "ASN",
        "Handle",
        "Name",
    ];

    let mut used = HashSet::new();
    let mut ordered = Vec::new();

    for key in preferred_order.iter() {
        if let Some(value) = fields
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| format_field_value(key, v))
        {
            ordered.push(((*key).to_string(), value));
            used.insert((*key).to_string());
        }
    }

    for (key, value) in fields {
        if used.contains(key) {
            continue;
        }
        ordered.push((key.clone(), format_field_value(key, value)));
    }

    let table_rows: Vec<Vec<String>> = std::iter::once(vec!["".to_string(), "".to_string()])
        .chain(ordered.into_iter().map(|(k, v)| vec![k, v]))
        .collect();

    let table = table::present_table(&table_rows)
        .lines()
        .skip(1)
        .collect::<Vec<_>>()
        .join("\n");

    let title = if query.chars().all(|c| c.is_ascii_digit()) {
        format!("AS{}", query)
    } else {
        query.to_string()
    };

    embed::make("WHOIS", &format!("```\n{}\n{}\n```", title, table), None)
}

fn format_field_value(key: &str, value: &str) -> String {
    match key {
        "Registration" | "Expiration" => {
            format_timestamp(value).unwrap_or_else(|| value.to_string())
        }
        _ => value.to_string(),
    }
}

fn format_timestamp(value: &str) -> Option<String> {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(value) {
        return Some(
            dt.with_timezone(&Utc)
                .format("%a, %d %b %Y %H:%M:%S GMT")
                .to_string(),
        );
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(value) {
        return Some(
            dt.with_timezone(&Utc)
                .format("%a, %d %b %Y %H:%M:%S GMT")
                .to_string(),
        );
    }
    None
}

fn extract_rdap_fields(json: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();

    if let Some(v) = json.get("name").and_then(|v| v.as_str()) {
        out.push(("Name".to_string(), v.to_string()));
    }

    if let Some(v) = json.get("handle").and_then(|v| v.as_str()) {
        out.push(("Handle".to_string(), v.to_string()));
    }

    if let Some(registrant) = find_entity_name("registrant", json) {
        out.push(("Registrant".to_string(), registrant));
    }

    if let Some(registrar) = find_entity_name("registrar", json) {
        out.push(("Registrar".to_string(), registrar));
    }

    if let Some(registration) = find_event_date("registration", json) {
        out.push(("Registration".to_string(), registration));
    }

    if let Some(expiration) = find_event_date("expiration", json) {
        out.push(("Expiration".to_string(), expiration));
    }

    if let Some(status) = json.get("status").and_then(|v| v.as_array()) {
        let status_str = status
            .iter()
            .filter_map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        if !status_str.is_empty() {
            out.push(("Status".to_string(), status_str));
        }
    }

    if let Some(asn_arr) = json
        .get("arin_originas0_originautnums")
        .and_then(|v| v.as_array())
    {
        let asn_list: Vec<String> = asn_arr
            .iter()
            .filter_map(|v| v.as_u64().map(|n| format!("AS{}", n)))
            .collect();
        if !asn_list.is_empty() {
            out.push(("ASN".to_string(), asn_list.join(", ")));
        }
    }

    if let Some(cidr) = json.get("cidr0_cidrs").and_then(|v| v.as_array()) {
        let cidr_list: Vec<String> = cidr
            .iter()
            .filter_map(|c| {
                let prefix = c
                    .get("v4prefix")
                    .or(c.get("v6prefix"))
                    .and_then(|v| v.as_str())?;
                let length = c.get("length").and_then(|v| v.as_u64())?;
                Some(format!("{}/{}", prefix, length))
            })
            .collect();
        if !cidr_list.is_empty() {
            out.push(("CIDR".to_string(), cidr_list.join(", ")));
        }
    }

    if let Some(ns) = json.get("nameservers").and_then(|v| v.as_array()) {
        let ns_list: Vec<String> = ns
            .iter()
            .filter_map(|n| n.get("ldhName").and_then(|v| v.as_str()))
            .map(|s| s.to_string())
            .collect();
        if !ns_list.is_empty() {
            out.push(("Nameservers".to_string(), ns_list.join(", ")));
        }
    }

    if let Some(abuse) = find_abuse_email(json) {
        out.push(("Abuse".to_string(), abuse));
    }

    out
}

fn find_entity_name(role: &str, json: &Value) -> Option<String> {
    let entities = json.get("entities")?.as_array()?;

    let mut names = Vec::new();
    for entity in entities {
        if let Some(roles) = entity.get("roles").and_then(|v| v.as_array()) {
            if roles
                .iter()
                .any(|r| r.as_str().map(|s| s.to_lowercase()) == Some(role.to_lowercase()))
            {
                if let Some(name) = get_vcard_field(entity, "fn") {
                    names.push(name);
                } else if let Some(handle) = entity.get("handle").and_then(|v| v.as_str()) {
                    names.push(handle.to_string());
                }
            }
        }
    }

    if names.is_empty() {
        None
    } else {
        Some(unique_comma_sep(&names))
    }
}

fn find_event_date(action: &str, json: &Value) -> Option<String> {
    let events = json.get("events")?.as_array()?;

    for event in events {
        if let Some(event_action) = event.get("eventAction").and_then(|v| v.as_str()) {
            if event_action.to_lowercase() == action.to_lowercase() {
                if let Some(date) = event.get("eventDate").and_then(|v| v.as_str()) {
                    return Some(date.to_string());
                }
            }
        }
    }

    None
}

fn get_vcard_field(entity: &Value, field_name: &str) -> Option<String> {
    let vcard_array = entity.get("vcardArray")?.as_array()?;
    let vcard_fields = vcard_array.get(1)?.as_array()?;

    for field in vcard_fields {
        if let Some(field_arr) = field.as_array() {
            if field_arr.first()?.as_str()? == field_name {
                return field_arr.get(3)?.as_str().map(|s| s.to_string());
            }
        }
    }

    None
}

fn find_abuse_email(json: &Value) -> Option<String> {
    let entities = json.get("entities")?.as_array()?;

    for entity in entities {
        if let Some(roles) = entity.get("roles").and_then(|v| v.as_array()) {
            if roles.iter().any(|r| r.as_str() == Some("abuse")) {
                if let Some(email) = get_vcard_field(entity, "email") {
                    return Some(email);
                }
            }
        }
    }

    for entity in entities {
        if let Some(roles) = entity.get("roles").and_then(|v| v.as_array()) {
            if roles.iter().any(|r| r.as_str() == Some("registrar")) {
                if let Some(nested_entities) = entity.get("entities").and_then(|v| v.as_array()) {
                    for nested in nested_entities {
                        if let Some(nested_roles) = nested.get("roles").and_then(|v| v.as_array()) {
                            if nested_roles.iter().any(|r| r.as_str() == Some("abuse")) {
                                if let Some(email) = get_vcard_field(nested, "email") {
                                    return Some(email);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

fn unique_comma_sep(items: &[String]) -> String {
    let mut unique: Vec<String> = items.iter().cloned().collect();
    unique.sort();
    unique.dedup();
    unique.join(", ")
}

fn extract_whois_fields(json: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();

    let data_array = match json.as_array() {
        Some(arr) => arr,
        None => return out,
    };

    let find_attr = |names: &[&str]| -> Option<String> {
        for name in names {
            for entry in data_array {
                if let (Some(key), Some(value)) = (
                    entry.get("key").and_then(|k| k.as_str()),
                    entry.get("value").and_then(|v| v.as_str()),
                ) {
                    if key == *name {
                        return Some(value.to_string());
                    }
                }
            }
        }
        None
    };

    let find_date_attr = |names: &[&str]| -> Option<String> { find_attr(names) };

    if let Some(v) = find_attr(&["domain name", "domain"]) {
        out.push(("Name".to_string(), v));
    }

    if let Some(v) = find_attr(&["registrant", "registrant name"]) {
        out.push(("Registrant".to_string(), v));
    }

    if let Some(v) = find_attr(&["registrar", "sponsoring registrar"]) {
        out.push(("Registrar".to_string(), v));
    }

    if let Some(v) = find_date_attr(&[
        "creation date",
        "registered",
        "registration time",
        "登録年月日",
    ]) {
        out.push(("Registration".to_string(), v));
    }

    if let Some(v) = find_date_attr(&["registry expiry date", "expiry date", "有効期限"]) {
        out.push(("Expiration".to_string(), v));
    }

    if let Some(v) = find_attr(&["registrar abuse contact email"]) {
        out.push(("Abuse".to_string(), v));
    }

    out
}

fn extract_cfwho_fields(json: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();

    if let Some(v) = json.get("netname").and_then(|v| v.as_str()) {
        out.push(("Name".to_string(), v.to_string()));
    }

    if let Some(v) = json.get("asn").and_then(|v| v.as_str()) {
        out.push(("ASN".to_string(), v.to_string()));
    }

    if let Some(v) = json.get("network").and_then(|v| v.as_str()) {
        out.push(("CIDR".to_string(), v.to_string()));
    }

    if let Some(contacts) = json
        .get("contacts")
        .and_then(|c| c.get("abuse"))
        .and_then(|a| a.as_array())
    {
        let emails: Vec<String> = contacts
            .iter()
            .filter_map(|e| e.as_str().map(|s| s.to_string()))
            .collect();
        if !emails.is_empty() {
            out.push(("Abuse".to_string(), unique_comma_sep(&emails)));
        }
    }

    out
}

fn norm(q: &str) -> String {
    let mut cleaned = q.trim().to_string();

    cleaned = cleaned
        .trim_start_matches("AS")
        .trim_start_matches("as")
        .to_string();

    if cleaned.contains("://") {
        if let Some(idx) = cleaned.find("://") {
            cleaned = cleaned[idx + 3..].to_string();
        }
    }

    if cleaned.contains(':') && cleaned.split('.').count() == 4 {
        if let Some(idx) = cleaned.rfind(':') {
            cleaned = cleaned[..idx].to_string();
        }
    }

    cleaned = cleaned.trim_end_matches('/').to_string();

    cleaned
}
