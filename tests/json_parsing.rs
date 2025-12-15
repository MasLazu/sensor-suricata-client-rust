use sensor_suricata_service_rust::types;

#[test]
fn parses_suricata_eve_without_top_level_metadata() {
    // Real Suricata EVE JSON lines typically do NOT have our internal `metadata` field.
    let json = r#"{
        \"timestamp\": \"2025-12-15T07:46:41.123456+0000\",
        \"event_type\": \"alert\",
        \"src_ip\": \"1.2.3.4\",
        \"dest_ip\": \"5.6.7.8\",
        \"proto\": \"TCP\",
        \"in_iface\": \"eth0\",
        \"alert\": {
            \"action\": \"allowed\",
            \"gid\": 1,
            \"signature_id\": 2100498,
            \"rev\": 9,
            \"signature\": \"GPL ATTACK_RESPONSE id check returned root\",
            \"category\": \"Potentially Bad Traffic\",
            \"severity\": 2
        }
    }"#;

    let mut bytes = json.as_bytes().to_vec();
    let alert: types::SuricataAlert = simd_json::from_slice(&mut bytes).expect("should parse");

    // defaults should apply
    assert_eq!(alert.metadata.sensor_id, "");
    assert_eq!(alert.metadata.sensor_version, "unknown");
}

#[test]
fn parses_suricata_eve_with_metadata_present() {
    let json = r#"{
        \"metadata\": {
            \"sensor_id\": \"sensor-x\",
            \"sensor_version\": \"1.0.0\",
            \"sent_at\": 123,
            \"hash_sha256\": \"abc\",
            \"read_at\": 456,
            \"received_at\": 789
        },
        \"timestamp\": \"2025-12-15T07:46:41.123456+0000\",
        \"event_type\": \"alert\",
        \"alert\": {
            \"action\": \"allowed\",
            \"gid\": 1,
            \"signature_id\": 1,
            \"rev\": 1,
            \"signature\": \"x\",
            \"category\": \"x\",
            \"severity\": 1
        }
    }"#;

    let mut bytes = json.as_bytes().to_vec();
    let alert: types::SuricataAlert = simd_json::from_slice(&mut bytes).expect("should parse");

    assert_eq!(alert.metadata.sensor_id, "sensor-x");
    assert_eq!(alert.metadata.sensor_version, "1.0.0");
    assert_eq!(alert.metadata.sent_at, 123);
}
