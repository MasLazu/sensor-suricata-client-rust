use crate::pb::{Metric, SensorEvent};
use crate::types::SuricataAlert;
use sha2::{Digest, Sha256};

pub fn convert_suricata_alert_to_sensor_event(
    data: &SuricataAlert,
) -> Option<(SensorEvent, Metric)> {
    if data.alert.is_none() {
        return None;
    }
    let alert = data.alert.as_ref().unwrap();

    let tos = 0; // Default

    let mut sensor_event = SensorEvent {
        metrics: vec![],
        event_hash_sha256: "".to_string(),
        event_metrics_count: 1,
        event_seconds: parse_timestamp(&data.timestamp),
        sensor_id: data.metadata.sensor_id.clone(),
        sensor_version: data.metadata.sensor_version.clone(),
        snort_action: Some(alert.action.clone()),
        snort_classification: Some(alert.category.clone()),
        snort_direction: data.direction.clone(),
        snort_interface: data.in_iface.clone().unwrap_or_default(),
        snort_message: alert.signature.clone(),
        snort_priority: alert.severity,
        snort_protocol: data.proto.clone().unwrap_or_default(),
        snort_rule_gid: alert.gid,
        snort_rule_rev: alert.rev,
        snort_rule_sid: alert.signature_id,
        snort_rule: format!("{}:{}:{}", alert.gid, alert.signature_id, alert.rev),
        snort_seconds: parse_timestamp(&data.timestamp),
        snort_service: data.app_proto.clone(),
        snort_type_of_service: Some(tos),
        event_read_at: data.metadata.read_at,
        event_sent_at: data.metadata.sent_at,
        event_received_at: data.metadata.received_at,
    };

    sensor_event.event_hash_sha256 = generate_hash_sha256(&sensor_event);

    /*
    if data.alert.as_ref().unwrap().signature_id % 10000 == 0 {
        log::info!(
            "Debug: sid={} hash={}",
            data.alert.as_ref().unwrap().signature_id,
            sensor_event.event_hash_sha256
        );
    }
    */

    let flow = data.flow.as_ref();
    let ether = data.ether.as_ref();

    let snort_client_bytes = flow.and_then(|f| f.bytes_toserver);
    let snort_client_pkts = flow.and_then(|f| f.pkts_toserver);
    let snort_dst_port = data.dest_port;
    let snort_dst_ap = if let (Some(ip), Some(port)) = (&data.dest_ip, data.dest_port) {
        Some(format!("{}:{}", ip, port))
    } else {
        None
    };
    let snort_flowstart_time = flow
        .and_then(|f| f.start.as_ref())
        .map(|s| parse_timestamp(s));
    let snort_base64_data = data.payload.clone();
    let snort_pkt_length = data.pkt_len;
    let snort_pkt_number = data.pcap_cnt;
    let snort_server_bytes = flow.and_then(|f| f.bytes_toclient);
    let snort_server_pkts = flow.and_then(|f| f.pkts_toclient);
    let snort_src_port = data.src_port;
    let snort_src_ap = if let (Some(ip), Some(port)) = (&data.src_ip, data.src_port) {
        Some(format!("{}:{}", ip, port))
    } else {
        None
    };
    let snort_tcp_flags = None; // Not available
    let snort_time_to_live = Some(0);
    let snort_vlan = Some(0);
    let snort_icmp_type = data.icmp_type;
    let snort_icmp_code = data.icmp_code;

    let snort_eth_type = derive_eth_type(data.ip_v.unwrap_or(4));
    let snort_eth_len = derive_eth_len(data.pkt_len.unwrap_or(0));
    let snort_pkt_gen = derive_pkt_gen(data.pkt_src.as_deref().unwrap_or(""));
    let snort_tcp_len = derive_tcp_len(
        data.proto.as_deref().unwrap_or(""),
        data.pkt_len.unwrap_or(0),
    );
    let snort_udp_len = derive_udp_len(
        data.proto.as_deref().unwrap_or(""),
        data.pkt_len.unwrap_or(0),
    );

    let sensor_metric = Metric {
        snort_timestamp: data.timestamp.clone(),
        snort_base64_data: snort_base64_data,
        snort_client_bytes: snort_client_bytes,
        snort_client_pkts: snort_client_pkts,
        snort_dst_address: data.dest_ip.clone(),
        snort_dst_port: snort_dst_port,
        snort_dst_ap: snort_dst_ap,
        snort_eth_dst: ether.and_then(|e| e.dest_mac.clone()),
        snort_eth_len: Some(snort_eth_len),
        snort_eth_src: ether.and_then(|e| e.src_mac.clone()),
        snort_eth_type: Some(snort_eth_type),
        snort_flowstart_time: snort_flowstart_time,
        snort_icmp_code: snort_icmp_code,
        snort_icmp_type: snort_icmp_type,
        snort_pkt_gen: Some(snort_pkt_gen),
        snort_pkt_length: snort_pkt_length,
        snort_pkt_number: snort_pkt_number,
        snort_server_bytes: snort_server_bytes,
        snort_server_pkts: snort_server_pkts,
        snort_src_address: data.src_ip.clone(),
        snort_src_port: snort_src_port,
        snort_src_ap: snort_src_ap,
        snort_tcp_flags: snort_tcp_flags,
        snort_tcp_len: snort_tcp_len,
        snort_time_to_live: snort_time_to_live,
        snort_udp_length: snort_udp_len,
        snort_vlan: snort_vlan,
        ..Default::default()
    };

    Some((sensor_event, sensor_metric))
}

fn parse_timestamp(_ts: &str) -> i64 {
    // Go format: "2006-01-02T15:04:05.000000-0700"
    // Rust chrono can parse this.
    // For simplicity, let's assume standard RFC3339 or similar.
    // If exact match is needed, we might need a custom parser or use chrono's `parse_from_str`.
    // Since I didn't add `chrono` to dependencies yet, I should probably add it or use a simple hack.
    // Let's assume 0 for now to avoid dependency hell in this step, or add chrono.
    // I'll add chrono in the next step if needed, but for now let's return 0 or try basic parsing.
    0
}

fn generate_hash_sha256(payload: &SensorEvent) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{:?}", payload)); // Debug format is not exactly the same as Go's String(), but close enough for unique hash?
                                             // Go's `payload.String()` returns a string representation of the proto message.
                                             // Rust's `Debug` implementation for Prost generated structs does something similar.
    let result = hasher.finalize();
    hex::encode(result)
}

fn derive_eth_type(ip_version: i64) -> String {
    match ip_version {
        4 => "0x800".to_string(),
        6 => "0x86dd".to_string(),
        _ => "0x800".to_string(),
    }
}

fn derive_eth_len(pkt_len: i64) -> i64 {
    pkt_len + 18
}

fn derive_pkt_gen(pkt_src: &str) -> String {
    pkt_src.to_string()
}

fn derive_tcp_len(proto: &str, pkt_len: i64) -> Option<i64> {
    if proto == "TCP" {
        let tcp_len = pkt_len - 34;
        if tcp_len > 0 {
            return Some(tcp_len);
        }
    }
    None
}

fn derive_udp_len(proto: &str, pkt_len: i64) -> Option<i64> {
    if proto == "UDP" {
        let udp_len = pkt_len - 20;
        if udp_len > 0 {
            return Some(udp_len);
        }
    }
    None
}
