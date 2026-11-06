use crate::{ProtocolError, Result};
use jsonschema::{Draft, Retrieve, Uri, Validator};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

const SOURCES: &[(&str, &str)] = &[
    (
        "types-v2",
        include_str!("../../../protocol/schemas/types-v2.json"),
    ),
    (
        "event-v2",
        include_str!("../../../protocol/schemas/event-v2.json"),
    ),
    (
        "operation-v2",
        include_str!("../../../protocol/schemas/operation-v2.json"),
    ),
    (
        "permit-v2",
        include_str!("../../../protocol/schemas/permit-v2.json"),
    ),
    (
        "network-grant-v2",
        include_str!("../../../protocol/schemas/network-grant-v2.json"),
    ),
    (
        "capabilities-v2",
        include_str!("../../../protocol/schemas/capabilities-v2.json"),
    ),
    (
        "grant-v2",
        include_str!("../../../protocol/schemas/grant-v2.json"),
    ),
    (
        "event-v1",
        include_str!("../../../protocol/schemas/event-v1.json"),
    ),
    (
        "crypto-vectors-v2",
        include_str!("../../../protocol/schemas/crypto-vectors-v2.json"),
    ),
];

#[derive(Clone)]
struct LocalResources(Arc<HashMap<String, Value>>);

impl Retrieve for LocalResources {
    fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> std::result::Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        self.0
            .get(uri.as_str())
            .cloned()
            .ok_or_else(|| "Only registered local schema URNs are available".into())
    }
}

pub struct Schemas {
    compiled: BTreeMap<String, Validator>,
    arguments: Validator,
}

impl Schemas {
    pub fn new() -> Result<Self> {
        let mut resources = HashMap::new();
        let mut parsed = Vec::new();
        for (name, source) in SOURCES {
            // Schema documents are trusted build-time artifacts, not bounded operation frames.
            let schema: Value = serde_json::from_str(source)
                .map_err(|_| ProtocolError::new("schema_build", "协议定义无法加载。"))?;
            jsonschema::meta::validate(&schema)
                .map_err(|_| ProtocolError::new("schema_meta", "协议定义未通过标准检查。"))?;
            resources.insert(schema["$id"].as_str().unwrap().to_owned(), schema.clone());
            parsed.push((*name, schema));
        }
        let resources = LocalResources(Arc::new(resources));
        let mut compiled = BTreeMap::new();
        for (name, schema) in parsed {
            let validator = jsonschema::options()
                .with_draft(Draft::Draft202012)
                .should_validate_formats(true)
                .with_retriever(resources.clone())
                .build(&schema)
                .map_err(|_| ProtocolError::new("schema_reference", "协议引用无法在本地解析。"))?;
            compiled.insert(name.to_owned(), validator);
        }
        let argument_schema =
            serde_json::json!({"$ref": "urn:agentguard:types:2.0.0#/$defs/value"});
        let arguments = jsonschema::options()
            .with_draft(Draft::Draft202012)
            .should_validate_formats(true)
            .with_retriever(resources)
            .build(&argument_schema)
            .map_err(|_| ProtocolError::new("schema_reference", "参数结构无法在本地解析。"))?;
        Ok(Self {
            compiled,
            arguments,
        })
    }

    pub fn validate(&self, name: &str, value: &Value) -> Result<()> {
        let schema = self.compiled.get(name).ok_or_else(|| {
            ProtocolError::new(
                "unknown_schema",
                "不支持这个协议版本或结构，本次请求已拒绝。",
            )
        })?;
        if !schema.is_valid(value) {
            return Err(ProtocolError::new(
                "schema_invalid",
                "请求缺少必要字段或包含不支持的内容，本次请求已拒绝。",
            ));
        }
        Ok(())
    }

    pub fn count(&self) -> usize {
        self.compiled.len()
    }

    pub fn validate_arguments(&self, value: &Value) -> Result<()> {
        if !self.arguments.is_valid(value) {
            return Err(ProtocolError::new(
                "argument_bounds",
                "操作参数超过协议范围，本次请求已拒绝。",
            ));
        }
        Ok(())
    }
}
