use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(rename = "sensor_id")]
    pub sensor_id: String,
    #[serde(rename = "sensor_version")]
    pub sensor_version: String,
    #[serde(rename = "sent_at")]
    pub sent_at: i64,
    #[serde(rename = "hash_sha256")]
    pub hash_sha256: String,
    #[serde(rename = "read_at")]
    pub read_at: i64,
    #[serde(rename = "received_at")]
    pub received_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuricataAlert {
    pub metadata: Metadata,
    pub timestamp: String,
    #[serde(rename = "flow_id")]
    pub flow_id: Option<i64>,
    #[serde(rename = "pcap_cnt")]
    pub pcap_cnt: Option<i64>,
    #[serde(rename = "event_type")]
    pub event_type: Option<String>,
    #[serde(rename = "src_ip")]
    pub src_ip: Option<String>,
    #[serde(rename = "src_port")]
    pub src_port: Option<i64>,
    #[serde(rename = "dest_ip")]
    pub dest_ip: Option<String>,
    #[serde(rename = "dest_port")]
    pub dest_port: Option<i64>,
    #[serde(rename = "proto")]
    pub proto: Option<String>,
    #[serde(rename = "ip_v")]
    pub ip_v: Option<i64>,
    #[serde(rename = "pkt_src")]
    pub pkt_src: Option<String>,
    #[serde(rename = "in_iface")]
    pub in_iface: Option<String>,
    #[serde(rename = "icmp_type")]
    pub icmp_type: Option<i64>,
    #[serde(rename = "icmp_code")]
    pub icmp_code: Option<i64>,
    #[serde(rename = "payload")]
    pub payload: Option<String>,
    #[serde(rename = "pkt_len")]
    pub pkt_len: Option<i64>,
    pub ether: Option<Ether>,
    #[serde(rename = "tx_id")]
    pub tx_id: Option<i64>,
    pub alert: Option<Alert>,
    pub http: Option<HTTP>,
    pub files: Option<Vec<FileInfo>>,
    #[serde(rename = "app_proto")]
    pub app_proto: Option<String>,
    pub direction: Option<String>,
    pub flow: Option<Flow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ether {
    #[serde(rename = "src_mac")]
    pub src_mac: Option<String>,
    #[serde(rename = "dest_mac")]
    pub dest_mac: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub action: String,
    pub gid: i64,
    #[serde(rename = "signature_id")]
    pub signature_id: i64,
    pub rev: i64,
    pub signature: String,
    pub category: String,
    pub severity: i64,
    pub metadata: Option<SuricataMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuricataMetadata {
    #[serde(rename = "affected_product")]
    pub affected_product: Option<Vec<String>>,
    #[serde(rename = "attack_target")]
    pub attack_target: Option<Vec<String>>,
    #[serde(rename = "created_at")]
    pub created_at: Option<Vec<String>>,
    #[serde(rename = "deployment")]
    pub deployment: Option<Vec<String>>,
    #[serde(rename = "former_category")]
    pub former_category: Option<Vec<String>>,
    #[serde(rename = "signature_severity")]
    pub signature_severity: Option<Vec<String>>,
    #[serde(rename = "updated_at")]
    pub updated_at: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTTP {
    pub hostname: Option<String>,
    #[serde(rename = "http_port")]
    pub http_port: Option<i64>,
    pub url: Option<String>,
    #[serde(rename = "http_content_type")]
    pub http_content_type: Option<String>,
    #[serde(rename = "http_method")]
    pub http_method: Option<String>,
    pub protocol: Option<String>,
    pub status: Option<i64>,
    pub length: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub filename: Option<String>,
    pub gaps: Option<bool>,
    pub state: Option<String>,
    pub stored: Option<bool>,
    pub size: Option<i64>,
    #[serde(rename = "tx_id")]
    pub tx_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flow {
    #[serde(rename = "pkts_toserver")]
    pub pkts_toserver: Option<i64>,
    #[serde(rename = "pkts_toclient")]
    pub pkts_toclient: Option<i64>,
    #[serde(rename = "bytes_toserver")]
    pub bytes_toserver: Option<i64>,
    #[serde(rename = "bytes_toclient")]
    pub bytes_toclient: Option<i64>,
    pub start: Option<String>,
    #[serde(rename = "src_ip")]
    pub src_ip: Option<String>,
    #[serde(rename = "dest_ip")]
    pub dest_ip: Option<String>,
    #[serde(rename = "src_port")]
    pub src_port: Option<i64>,
    #[serde(rename = "dest_port")]
    pub dest_port: Option<i64>,
}
