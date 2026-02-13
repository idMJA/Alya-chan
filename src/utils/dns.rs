use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hickory_proto::op::{Message, MessageType, OpCode, Query};
use hickory_proto::rr::{Name, RData, RecordType};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct Endpoint {
    pub endpoint: &'static str,
    pub mode: EndpointMode,
}

#[derive(Clone, Debug)]
pub enum EndpointMode {
    Json,
    Dns,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Flags {
    pub cd: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Answer {
    pub name: String,
    pub ttl: u32,
    pub data: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lookup {
    pub name: String,
    pub flags: Flags,
    pub message: Option<String>,
    pub answer: Vec<Answer>,
}

const RCODES: &[(u16, &str)] = &[
    (
        1,
        "A format error [1 - FormErr] occurred when looking up the domain",
    ),
    (
        2,
        "An unexpected server failure [2 - ServFail] occurred when looking up the domain",
    ),
    (
        3,
        "A non-existent domain [3 - NXDomain] was requested and could not be found",
    ),
    (
        4,
        "A request was made that is not implemented [4 - NotImp] by the resolver",
    ),
    (5, "The query was refused [5 - Refused] by the DNS resolver"),
];

pub async fn lookup(domain: &str, t: &str, ep: &Endpoint, flags: Flags) -> Lookup {
    match ep.mode {
        EndpointMode::Json => lookup_json(domain, t, ep, flags.clone()).await,
        EndpointMode::Dns => lookup_dns(domain, t, ep, flags.clone()).await,
    }
}

async fn lookup_json(domain: &str, t: &str, ep: &Endpoint, flags: Flags) -> Lookup {
    let mut url = format!("{}?name={}&type={}", ep.endpoint, domain, t);
    let _ = write!(url, "&cd={}", flags.cd);

    let res = reqwest::Client::new()
        .get(url)
        .header("Accept", "application/dns-json")
        .send()
        .await;

    if let Ok(res) = res {
        if let Ok(json) = res.json::<serde_json::Value>().await {
            let status = json
                .get("Status")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            if status != 0 {
                return Lookup {
                    name: domain.to_string(),
                    flags,
                    message: Some(rcode(u16::try_from(status).unwrap_or(0))),
                    answer: vec![],
                };
            }

            let mut answers = Vec::new();
            if let Some(arr) = json.get("Answer").and_then(|v| v.as_array()) {
                for a in arr {
                    let name = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let ttl = u32::try_from(
                        a.get("TTL")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or(0),
                    )
                    .unwrap_or(0);
                    let data = a
                        .get("data")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    answers.push(Answer {
                        name: name.to_string(),
                        ttl,
                        data,
                    });
                }
            }

            return Lookup {
                name: domain.to_string(),
                flags,
                message: None,
                answer: answers,
            };
        }
    }

    Lookup {
        name: domain.to_string(),
        flags,
        message: Some("An unexpected error occurred".to_string()),
        answer: vec![],
    }
}

async fn lookup_dns(domain: &str, t: &str, ep: &Endpoint, flags: Flags) -> Lookup {
    let Ok(name) = Name::from_utf8(domain) else {
        return Lookup {
            name: domain.to_string(),
            flags,
            message: Some("An unexpected error occurred".to_string()),
            answer: vec![],
        };
    };

    let mut msg = Message::new();
    msg.set_id(rand::thread_rng().gen_range(1..=65534));
    msg.set_message_type(MessageType::Query);
    msg.set_op_code(OpCode::Query);
    let record_type = RecordType::from_str(&t.to_uppercase()).unwrap_or(RecordType::A);
    msg.add_query(Query::query(name, record_type));
    msg.set_recursion_desired(true);
    msg.set_checking_disabled(flags.cd);

    let packet = msg.to_vec().unwrap_or_default();
    let encoded = URL_SAFE_NO_PAD.encode(packet);

    let url = format!("{}?dns={}", ep.endpoint, encoded);

    let res = reqwest::Client::new()
        .get(url)
        .header("Accept", "application/dns-message")
        .send()
        .await;

    if let Ok(res) = res {
        if let Ok(bytes) = res.bytes().await {
            if let Ok(msg) = Message::from_vec(&bytes) {
                if msg.response_code() != hickory_proto::op::ResponseCode::NoError {
                    return Lookup {
                        name: domain.to_string(),
                        flags,
                        message: Some(rcode(msg.response_code().into())),
                        answer: vec![],
                    };
                }

                let answers = msg
                    .answers()
                    .iter()
                    .map(|r| Answer {
                        name: r.name().to_string(),
                        ttl: r.ttl(),
                        data: r.data().map(rdata_to_string).unwrap_or_default(),
                    })
                    .collect::<Vec<_>>();

                return Lookup {
                    name: domain.to_string(),
                    flags,
                    message: None,
                    answer: answers,
                };
            }
        }
    }

    Lookup {
        name: domain.to_string(),
        flags,
        message: Some("An unexpected error occurred".to_string()),
        answer: vec![],
    }
}

fn rdata_to_string(data: &RData) -> String {
    data.to_string()
}

fn rcode(code: u16) -> String {
    if code == 0 {
        return "No error".to_string();
    }
    for (k, v) in RCODES {
        if *k == code {
            return v.to_string();
        }
    }
    format!("An unexpected error occurred [{code}]")
}
