use agentguard_protocol::{
    crypto::{self, Domain},
    ingress::{Role, Session},
    migration::import_legacy,
    parse::{MAX_FRAME_BYTES, parse_json},
    schema::Schemas,
};
use serde_json::{Value, json};
use std::sync::{Arc, OnceLock};

fn schemas() -> Arc<Schemas> {
    static SET: OnceLock<Arc<Schemas>> = OnceLock::new();
    SET.get_or_init(|| Arc::new(Schemas::new().unwrap()))
        .clone()
}

fn fixtures() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/p0.1.json")).unwrap()
}

fn fixture(id: &str) -> Value {
    fixtures()
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == id)
        .unwrap()["value"]
        .clone()
}

fn request(sequence: u64, method: &str, payload: Value) -> Vec<u8> {
    serde_json::to_vec(&json!({"major":2,"minor":0,"sequence":sequence,
        "challenge":"fixture-challenge","method":method,"payload":payload}))
    .unwrap()
}

fn observation() -> Value {
    json!({"source_event_id":"source-1","agent_id":"A01","tool_name":"fixture-read",
           "command":null,"arguments":{"path":"virtual.txt"}})
}

fn session(role: Role) -> Session {
    Session::new(role, "fixture-challenge".into(), schemas())
}

#[test]
fn all_65_fixtures_match_frozen_schemas() {
    assert_eq!(schemas().count(), 9);
    let data = fixtures();
    assert_eq!(data.as_array().unwrap().len(), 65);
    for f in data.as_array().unwrap() {
        let valid = schemas()
            .validate(f["schema"].as_str().unwrap(), &f["value"])
            .is_ok();
        assert_eq!(valid, f["valid"].as_bool().unwrap(), "{}", f["id"]);
    }
}

#[test]
fn twenty_one_cross_language_vectors_match_bytes_frames_and_macs() {
    let bundle = fixture("CV-BUNDLE");
    let mut key = [0; 32];
    for (index, byte) in key.iter_mut().enumerate() {
        *byte = index as u8;
    }
    for vector in bundle["vectors"].as_array().unwrap() {
        let domain = if vector["domain"] == "secret-v2" {
            Domain::Secret
        } else {
            Domain::Operation
        };
        let input = &vector["input"];
        assert_eq!(
            crypto::hex(&crypto::typed_encode(input).unwrap()),
            vector["encoded_hex"]
        );
        assert_eq!(
            crypto::hex(&crypto::frame(domain, input).unwrap()),
            vector["framed_hex"]
        );
        assert_eq!(
            crypto::hex(&crypto::mac(&key, domain, input).unwrap()),
            vector["hmac_sha256"]
        );
        crypto::verify(&key, domain, input, vector["hmac_sha256"].as_str().unwrap()).unwrap();
    }
}

#[test]
fn domains_and_changed_values_invalidate_mac() {
    let key = [7; 32];
    let value = json!({"argument":1});
    let signed = crypto::hex(&crypto::mac(&key, Domain::Operation, &value).unwrap());
    assert!(crypto::verify(&key, Domain::Secret, &value, &signed).is_err());
    assert!(crypto::verify(&key, Domain::Operation, &json!({"argument":2}), &signed).is_err());
    assert!(crypto::verify(&[8; 32], Domain::Operation, &value, &signed).is_err());
    assert!(crypto::verify(&key, Domain::Operation, &value, &signed.to_uppercase()).is_err());
    assert!(crypto::verify(&key, Domain::Operation, &value, "short").is_err());
}

#[test]
fn map_order_is_stable_but_unicode_paths_and_arrays_remain_exact() {
    assert_eq!(
        crypto::typed_encode(&json!({"b":2,"a":1})).unwrap(),
        crypto::typed_encode(&json!({"a":1,"b":2})).unwrap()
    );
    for (left, right) in [
        (json!("é"), json!("e\u{301}")),
        (json!("D:\\GuardLab"), json!("d:\\guardlab")),
        (Value::Null, json!("")),
        (json!([1, 2]), json!([2, 1])),
    ] {
        assert_ne!(
            crypto::typed_encode(&left).unwrap(),
            crypto::typed_encode(&right).unwrap()
        );
    }
}

#[test]
fn parser_rejects_ambiguous_numbers_unicode_and_duplicate_keys() {
    for raw in [
        r#"{"x":1,"x":2}"#,
        r#"{"x":1,"\u0078":2}"#,
        "1.0",
        "1e0",
        "-0",
        "NaN",
        "Infinity",
        "9007199254740992",
        "-9007199254740992",
        r#""\ud800""#,
        r#""\udfff""#,
        "\u{feff}{}",
        "{}{}",
    ] {
        assert!(parse_json(raw.as_bytes()).is_err(), "{raw}");
    }
    assert!(parse_json(&[0xff, 0xfe]).is_err());
    assert_eq!(
        parse_json(b"9007199254740991").unwrap(),
        json!(9007199254740991i64)
    );
    assert_eq!(parse_json(br#""\ud83d\ude00""#).unwrap(), json!("😀"));
}

#[test]
fn parser_and_encoder_enforce_resource_budgets() {
    assert!(parse_json(&vec![b' '; MAX_FRAME_BYTES + 1]).is_err());
    let deep = format!("{}0{}", "[".repeat(10), "]".repeat(10));
    assert!(parse_json(deep.as_bytes()).is_err());
    assert!(parse_json(serde_json::to_string(&vec![0; 129]).unwrap().as_bytes()).is_err());
    assert!(crypto::typed_encode(&json!(vec![0; 129])).is_err());
    let map: serde_json::Map<_, _> = (0..65).map(|i| (i.to_string(), json!(i))).collect();
    assert!(parse_json(serde_json::to_string(&map).unwrap().as_bytes()).is_err());
    assert!(crypto::typed_encode(&Value::Object(map)).is_err());
    assert!(crypto::typed_encode(&json!(1.0)).is_err());
    assert!(crypto::frame(Domain::Operation, &json!("x".repeat(MAX_FRAME_BYTES))).is_err());
}

#[test]
fn eight_legacy_records_migrate_without_authority_and_preserve_source_digest() {
    for index in 1..=8 {
        let old = fixture(&format!("LEG{index:02}"));
        let raw = serde_json::to_vec(&old).unwrap();
        let imported = import_legacy(
            &schemas(),
            &raw,
            "2026-11-06T10:00:00Z",
            "00000000-0000-4000-8000-000000000999",
            index,
        )
        .unwrap();
        let event = imported.event;
        assert_eq!(event["event_id"], old["event_id"]);
        assert_eq!(event["record_kind"], "imported");
        assert_eq!(event["policy_decision"], "not_evaluated");
        assert_eq!(event["execution_mode"], "none");
        assert_eq!(event["capabilities"]["level"], "L0");
        for flag in [
            "telemetry_verified",
            "pre_execution_block_verified",
            "parameter_binding_verified",
            "single_use_grant_verified",
            "os_isolation_verified",
        ] {
            assert_eq!(event["capabilities"][flag], false);
        }
        for name in [
            "grant",
            "operation_digest",
            "prepared_id",
            "semantic_digest",
            "broker_instance",
            "decision_id",
        ] {
            assert!(event[name].is_null());
        }
        assert!(event["principal_snapshot"]["os_verified"].is_null());
        assert_eq!(imported.original_sha256.len(), 64);
        schemas().validate("event-v2", &event).unwrap();
    }
}

#[test]
fn every_grant_state_pair_matches_contract() {
    let states = [
        "created",
        "pending",
        "reserved",
        "consumed",
        "revoked",
        "expired",
        "invalidated",
        "failed",
    ];
    for before in states {
        for after in states {
            let mut grant = fixture(&format!("GR-{after}"));
            grant["previous_state"] = json!(before);
            let expected = matches!(
                (before, after),
                ("created", "pending") | ("pending", "reserved") | ("reserved", "consumed")
            ) || (["created", "pending", "reserved"].contains(&before)
                && ["revoked", "expired", "invalidated", "failed"].contains(&after));
            assert_eq!(
                schemas().validate("grant-v2", &grant).is_ok(),
                expected,
                "{before}->{after}"
            );
        }
    }
}

#[test]
fn formats_unknown_major_and_security_extensions_are_rejected() {
    for (field, value) in [
        ("received_at", json!("not-a-time")),
        ("event_id", json!("not-a-uuid")),
        ("schema_version", json!("3.0.0")),
    ] {
        let mut event = fixture("EV01");
        event[field] = value;
        assert!(schemas().validate("event-v2", &event).is_err());
    }
    let mut event = fixture("EV01");
    event["extensions"] = json!({"operation_digest":"fake"});
    assert!(schemas().validate("event-v2", &event).is_err());
    assert!(schemas().validate("newer-schema", &event).is_err());
}

#[test]
fn adapter_protected_fields_and_handle_integers_are_rejected() {
    for key in [
        "policy_decision",
        "risk_level",
        "capabilities",
        "grant",
        "principal_snapshot",
        "operation_digest",
        "handle",
    ] {
        let mut payload = observation();
        payload[key] = json!(123);
        let error = session(Role::Adapter)
            .accept(&request(1, "report_event", payload))
            .unwrap_err();
        assert_eq!(error.code, "protected_fields");
    }
    for key in ["command", "tool_name"] {
        let mut payload = observation();
        payload.as_object_mut().unwrap().remove(key);
        let error = session(Role::Adapter)
            .accept(&request(1, "report_event", payload))
            .unwrap_err();
        assert_eq!(error.code, "missing_observation_field");
    }
}

#[test]
fn methods_are_pinned_to_authenticated_role_and_no_authorization_exists() {
    assert!(
        session(Role::Ui)
            .accept(&request(1, "report_event", observation()))
            .is_err()
    );
    assert!(
        session(Role::Broker)
            .accept(&request(1, "report_event", observation()))
            .is_err()
    );
    assert_eq!(
        session(Role::Sensor)
            .accept(&request(1, "evaluate", json!({})))
            .unwrap_err()
            .code,
        "role_denied"
    );
    assert_eq!(
        session(Role::Adapter)
            .accept(&request(1, "evaluate", json!({})))
            .unwrap_err()
            .code,
        "authorization_unavailable"
    );
    assert!(
        session(Role::Adapter)
            .accept(&request(1, "redeem", json!({})))
            .is_err()
    );
}

#[test]
fn sequences_challenges_and_source_duplicates_are_bounded() {
    let mut connection = session(Role::Adapter);
    assert_eq!(
        connection
            .accept(&request(1, "report_event", observation()))
            .unwrap()["status"],
        "validated_not_persisted"
    );
    assert_eq!(
        connection
            .accept(&request(2, "report_event", observation()))
            .unwrap()["status"],
        "duplicate"
    );
    let mut changed = observation();
    changed["arguments"] = json!({"path":"changed"});
    assert_eq!(
        connection
            .accept(&request(3, "report_event", changed))
            .unwrap_err()
            .code,
        "source_conflict"
    );
    assert!(
        connection
            .accept(&request(3, "heartbeat", json!({})))
            .is_err()
    );
    let mut wrong: Value = serde_json::from_slice(&request(1, "heartbeat", json!({}))).unwrap();
    wrong["challenge"] = json!("old-connection");
    assert!(
        session(Role::Adapter)
            .accept(&serde_json::to_vec(&wrong).unwrap())
            .is_err()
    );
}

#[test]
fn response_errors_do_not_echo_sensitive_arguments() {
    let mut payload = observation();
    payload["policy_decision"] = json!("FAKE_SECRET_DO_NOT_LOG");
    let error = session(Role::Adapter)
        .accept(&request(1, "report_event", payload))
        .unwrap_err();
    assert!(
        !serde_json::to_string(&error)
            .unwrap()
            .contains("FAKE_SECRET")
    );
}

#[test]
fn generic_argument_limits_and_unicode_character_lengths_follow_schema() {
    let mut payload = observation();
    payload["source_event_id"] = json!("中".repeat(128));
    session(Role::Adapter)
        .accept(&request(1, "report_event", payload))
        .unwrap();
    let mut payload = observation();
    payload["arguments"] = json!({"secret":"x".repeat(16385)});
    assert!(
        session(Role::Adapter)
            .accept(&request(1, "report_event", payload))
            .is_err()
    );
}
