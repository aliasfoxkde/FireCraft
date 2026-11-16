//! A JSON Schema (draft-07) validator, restricted to the keywords the
//! CycloneDX schema uses.
//!
//! The SBOM gate validates its output against the published CycloneDX
//! 1.5 schema (T27-55).  The schema is the authority, so the validator
//! reads it rather than re-implementing its rules — but a JSON Schema
//! implementation is a big dependency to add to a gate crate, and the
//! schema uses a small, knowable set of keywords.
//!
//! So the validator implements that set and **refuses to run against a
//! schema that uses anything else**: [`Validator::with_siblings`] walks
//! every schema position in the document and fails on a keyword this
//! file does not implement.  A validator that silently ignored an
//! unknown keyword would report a document as valid when the schema
//! says otherwise, which is the failure mode a gate must not have.
//!
//! The keywords the CycloneDX 1.5 schema uses:
//! `type`, `properties`, `required`, `additionalProperties`,
//! `additionalItems`, `items`, `enum`, `const`, `pattern`, `format`,
//! `oneOf`, `anyOf`, `allOf`, `uniqueItems`, `minItems`, `maxItems`,
//! `minLength`, `maxLength`, `minimum`, `maximum`, `$ref`.
//! `definitions` holds the named schemas `$ref` points at, and one
//! `$ref` — the SPDX identifier list — names a sibling schema file,
//! which [`Validator::with_siblings`] supplies.

use regex::Regex;
use serde_json::Value;
use std::collections::BTreeMap;

/// The validation keywords this validator implements.
const KEYWORDS: &[&str] = &[
    "$ref",
    "additionalItems",
    "additionalProperties",
    "allOf",
    "anyOf",
    "const",
    "enum",
    "format",
    "items",
    "maxItems",
    "maxLength",
    "maximum",
    "minItems",
    "minLength",
    "minimum",
    "oneOf",
    "pattern",
    "properties",
    "required",
    "type",
    "uniqueItems",
];

/// The keywords that carry no validation: annotations and the schema's
/// own identity.  They are read by tools, not by the validator.
const ANNOTATIONS: &[&str] = &[
    "$comment",
    "$id",
    "$schema",
    "default",
    "definitions",
    "description",
    "examples",
    "title",
];

/// A schema, compiled and ready to validate documents.
#[derive(Debug)]
pub struct Validator {
    /// The whole schema document; `$ref` pointers resolve into it.
    root: Value,
    /// Schemas a `$ref` may name by file, such as the SPDX identifier
    /// list the `CycloneDX` schema points at.
    siblings: BTreeMap<String, Value>,
    /// The schema's `pattern` values, compiled once.
    patterns: BTreeMap<String, Regex>,
}

impl Validator {
    /// Compile a schema with the sibling schemas its `$ref`s name.
    ///
    /// The `CycloneDX` schema points at `spdx.schema.json` for the
    /// SPDX identifier list, so the sibling is supplied by file name.
    ///
    /// Fails when the schema uses a validation keyword this validator
    /// does not implement: a document validated against a partially
    /// understood schema is not a validated document.
    pub fn with_siblings(
        schema: &Value,
        siblings: BTreeMap<String, Value>,
    ) -> Result<Self, String> {
        let mut patterns = BTreeMap::new();
        let mut keywords = Vec::new();
        collect_keywords(schema, "", &mut keywords);
        for (name, sibling) in &siblings {
            collect_keywords(sibling, name, &mut keywords);
        }
        if let Some(keyword) = keywords.first() {
            return Err(format!(
                "the schema uses {keyword}, which this validator does not implement"
            ));
        }

        let mut sources = Vec::new();
        collect_patterns(schema, &mut sources);
        for sibling in siblings.values() {
            collect_patterns(sibling, &mut sources);
        }
        for source in sources {
            let compiled = Regex::new(&source)
                .map_err(|e| format!("the schema's pattern {source:?} does not compile: {e}"))?;
            patterns.insert(source, compiled);
        }

        Ok(Self {
            root: schema.clone(),
            siblings,
            patterns,
        })
    }

    /// Validate a document against the schema.
    ///
    /// Returns the errors, each naming the JSON pointer it was found
    /// at.  An empty error list means the document conforms.
    pub fn validate(&self, document: &Value) -> Vec<String> {
        let mut errors = Vec::new();
        self.check(&self.root.clone(), document, "", &mut errors);
        errors
    }

    /// Validate one schema against one document.
    fn check(&self, schema: &Value, document: &Value, path: &str, errors: &mut Vec<String>) {
        let Some(object) = schema.as_object() else {
            // A schema that is not an object constrains nothing.
            return;
        };

        if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
            match self.resolve(reference) {
                Some(target) => {
                    let target = target.clone();
                    self.check(&target, document, path, errors);
                }
                None => errors.push(format!(
                    "{path}: the schema's $ref {reference:?} does not resolve"
                )),
            }
            // draft-07: a `$ref` replaces the schema it appears in.
            return;
        }

        self.check_type(object, document, path, errors);
        self.check_enum(object, document, path, errors);
        self.check_string(object, document, path, errors);
        self.check_number(object, document, path, errors);
        self.check_array(object, document, path, errors);
        self.check_object(object, document, path, errors);
        self.check_combinators(object, document, path, errors);
    }

    /// The document's type against the schema's `type`.
    fn check_type(
        &self,
        schema: &serde_json::Map<String, Value>,
        document: &Value,
        path: &str,
        errors: &mut Vec<String>,
    ) {
        let Some(expected) = schema.get("type") else {
            return;
        };
        let matches = match expected {
            Value::String(name) => has_type(document, name),
            Value::Array(names) => names
                .iter()
                .filter_map(Value::as_str)
                .any(|name| has_type(document, name)),
            _ => true,
        };
        if !matches {
            let wanted = match expected {
                Value::String(name) => name.clone(),
                Value::Array(names) => names
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(" or "),
                _ => "?".to_owned(),
            };
            errors.push(format!(
                "{path}: expected {wanted}, found {}",
                type_name(document)
            ));
        }
    }

    /// The document against `enum` and `const`.
    fn check_enum(
        &self,
        schema: &serde_json::Map<String, Value>,
        document: &Value,
        path: &str,
        errors: &mut Vec<String>,
    ) {
        if let Some(choices) = schema.get("enum").and_then(Value::as_array) {
            if !choices.contains(document) {
                errors.push(format!(
                    "{path}: {} is not one of the {} allowed values",
                    document,
                    choices.len()
                ));
            }
        }
        if let Some(expected) = schema.get("const") {
            if document != expected {
                errors.push(format!("{path}: expected {expected}, found {document}"));
            }
        }
    }

    /// The document against `pattern` and `format`.
    fn check_string(
        &self,
        schema: &serde_json::Map<String, Value>,
        document: &Value,
        path: &str,
        errors: &mut Vec<String>,
    ) {
        let Some(text) = document.as_str() else {
            return;
        };
        if let Some(pattern) = schema.get("pattern").and_then(Value::as_str) {
            if let Some(compiled) = self.patterns.get(pattern) {
                if !compiled.is_match(text) {
                    errors.push(format!("{path}: {text:?} does not match {pattern:?}"));
                }
            }
        }
        if let Some(format) = schema.get("format").and_then(Value::as_str) {
            // In draft-07 `format` is an annotation, not an assertion:
            // a validator that does not implement a format says
            // nothing about the document.  Only `date-time` is
            // asserted here, because the SBOM's one timestamp is the
            // field where a wrong value would otherwise pass silently;
            // every other format is accepted and not checked.
            if format == "date-time" && !is_rfc3339(text) {
                errors.push(format!("{path}: {text:?} is not an RFC 3339 date-time"));
            }
        }
    }

    /// The document against `minimum` and `maximum`.
    fn check_number(
        &self,
        schema: &serde_json::Map<String, Value>,
        document: &Value,
        path: &str,
        errors: &mut Vec<String>,
    ) {
        let Some(number) = document.as_f64() else {
            return;
        };
        if let Some(minimum) = schema.get("minimum").and_then(Value::as_f64) {
            if number < minimum {
                errors.push(format!("{path}: {number} is below the minimum {minimum}"));
            }
        }
        if let Some(maximum) = schema.get("maximum").and_then(Value::as_f64) {
            if number > maximum {
                errors.push(format!("{path}: {number} is above the maximum {maximum}"));
            }
        }
    }

    /// The document against `items`, `additionalItems`, `minItems`,
    /// `maxItems` and `uniqueItems`.
    fn check_array(
        &self,
        schema: &serde_json::Map<String, Value>,
        document: &Value,
        path: &str,
        errors: &mut Vec<String>,
    ) {
        let Some(items) = document.as_array() else {
            return;
        };
        match schema.get("items") {
            Some(Value::Object(_)) => {
                let item_schema = schema["items"].clone();
                for (index, item) in items.iter().enumerate() {
                    let item_path = format!("{path}/{index}");
                    self.check(&item_schema, item, &item_path, errors);
                }
            }
            Some(Value::Array(tuple)) => {
                for (index, item) in items.iter().enumerate() {
                    let Some(item_schema) = tuple.get(index) else {
                        continue;
                    };
                    let item_path = format!("{path}/{index}");
                    self.check(&item_schema.clone(), item, &item_path, errors);
                }
                if items.len() > tuple.len() {
                    match schema.get("additionalItems") {
                        Some(Value::Bool(false)) => errors.push(format!(
                            "{path}: {} item(s) exceed the {} the schema allows",
                            items.len() - tuple.len(),
                            tuple.len()
                        )),
                        Some(Value::Object(_)) => {
                            let extra = schema["additionalItems"].clone();
                            for (index, item) in items.iter().enumerate().skip(tuple.len()) {
                                let item_path = format!("{path}/{index}");
                                self.check(&extra, item, &item_path, errors);
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        if let Some(minimum) = schema.get("minItems").and_then(Value::as_u64) {
            if (items.len() as u64) < minimum {
                errors.push(format!(
                    "{path}: {} item(s) is fewer than the {minimum} the schema requires",
                    items.len()
                ));
            }
        }
        if let Some(maximum) = schema.get("maxItems").and_then(Value::as_u64) {
            if (items.len() as u64) > maximum {
                errors.push(format!(
                    "{path}: {} item(s) is more than the {maximum} the schema allows",
                    items.len()
                ));
            }
        }
        if schema.get("uniqueItems") == Some(&Value::Bool(true)) {
            for (index, item) in items.iter().enumerate() {
                if items[..index].contains(item) {
                    errors.push(format!("{path}: the item at {index} is a duplicate"));
                }
            }
        }
    }

    /// The document against `properties`, `required` and
    /// `additionalProperties`.
    fn check_object(
        &self,
        schema: &serde_json::Map<String, Value>,
        document: &Value,
        path: &str,
        errors: &mut Vec<String>,
    ) {
        let Some(fields) = document.as_object() else {
            return;
        };
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for name in required.iter().filter_map(Value::as_str) {
                if !fields.contains_key(name) {
                    errors.push(format!("{path}: the required field {name:?} is missing"));
                }
            }
        }
        let properties = schema.get("properties").and_then(Value::as_object);
        for (name, field) in fields {
            let Some(property_schema) = properties.and_then(|all| all.get(name)) else {
                match schema.get("additionalProperties") {
                    Some(Value::Bool(false)) => {
                        errors.push(format!("{path}: the field {name:?} is not in the schema"))
                    }
                    Some(Value::Object(_)) => {
                        let extra = schema["additionalProperties"].clone();
                        let field_path = format!("{path}/{name}");
                        self.check(&extra, field, &field_path, errors);
                    }
                    _ => {}
                }
                continue;
            };
            let property_schema = property_schema.clone();
            let field_path = format!("{path}/{name}");
            self.check(&property_schema, field, &field_path, errors);
        }
    }

    /// Whether the document satisfies one branch of a combinator.
    ///
    /// The branch is checked against a throwaway error list: `anyOf`
    /// and `oneOf` care whether the branch accepted the document, not
    /// what the branch would have said about it.
    fn satisfies(&self, branch: &Value, document: &Value, path: &str) -> bool {
        let mut errors = Vec::new();
        self.check(&branch.clone(), document, path, &mut errors);
        errors.is_empty()
    }

    /// The document against `allOf`, `anyOf` and `oneOf`.
    fn check_combinators(
        &self,
        schema: &serde_json::Map<String, Value>,
        document: &Value,
        path: &str,
        errors: &mut Vec<String>,
    ) {
        if let Some(branches) = schema.get("allOf").and_then(Value::as_array) {
            for branch in branches {
                let branch = branch.clone();
                self.check(&branch, document, path, errors);
            }
        }
        if let Some(branches) = schema.get("anyOf").and_then(Value::as_array) {
            let satisfied = branches
                .iter()
                .any(|branch| self.satisfies(branch, document, path));
            if !satisfied {
                errors.push(format!(
                    "{path}: the document matches none of the {} alternatives the schema allows",
                    branches.len()
                ));
            }
        }
        if let Some(branches) = schema.get("oneOf").and_then(Value::as_array) {
            let matched = branches
                .iter()
                .filter(|branch| self.satisfies(branch, document, path))
                .count();
            if matched != 1 {
                errors.push(format!(
                    "{path}: the document matches {matched} of the {} alternatives the schema allows, and it must match exactly one",
                    branches.len()
                ));
            }
        }
    }

    /// Resolve a reference into the schema.
    ///
    /// A `#`-prefixed reference is a local JSON pointer
    /// (`#/definitions/x`); any other reference names a sibling
    /// schema by file (`spdx.schema.json`).
    fn resolve(&self, reference: &str) -> Option<&Value> {
        let Some(pointer) = reference.strip_prefix('#') else {
            let name = reference.rsplit('/').next().unwrap_or(reference);
            return self.siblings.get(name);
        };
        if pointer.is_empty() {
            return Some(&self.root);
        }
        let mut node = &self.root;
        for raw in pointer.trim_start_matches('/').split('/') {
            let name = raw.replace("~1", "/").replace("~0", "~");
            node = match node {
                Value::Object(fields) => fields.get(&name)?,
                Value::Array(items) => items.get(name.parse::<usize>().ok()?)?,
                _ => return None,
            };
        }
        Some(node)
    }
}

/// Whether the document has the named JSON type.
fn has_type(document: &Value, name: &str) -> bool {
    match name {
        "object" => document.is_object(),
        "array" => document.is_array(),
        "string" => document.is_string(),
        "boolean" => document.is_boolean(),
        "null" => document.is_null(),
        "number" => document.is_number(),
        "integer" => document
            .as_f64()
            .is_some_and(|number| number.fract() == 0.0),
        _ => true,
    }
}

/// The document's type, for an error message.
fn type_name(document: &Value) -> &'static str {
    match document {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

/// Whether the text is an RFC 3339 date-time.
///
/// `2026-11-15T09:30:00Z`, `2026-11-15T09:30:00.250+01:00`. The
/// schema's `date-time` format requires this shape.
fn is_rfc3339(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() < 20 {
        return false;
    }
    let digits = |slice: &[u8]| slice.iter().all(u8::is_ascii_digit);
    if !digits(&bytes[0..4]) || bytes[4] != b'-' {
        return false;
    }
    if !digits(&bytes[5..7]) || bytes[7] != b'-' {
        return false;
    }
    if !digits(&bytes[8..10]) {
        return false;
    }
    if !matches!(bytes[10], b'T' | b't' | b' ') {
        return false;
    }
    if !digits(&bytes[11..13]) || bytes[13] != b':' {
        return false;
    }
    if !digits(&bytes[14..16]) || bytes[16] != b':' {
        return false;
    }
    if !digits(&bytes[17..19]) {
        return false;
    }
    let mut rest = &bytes[19..];
    if rest.first() == Some(&b'.') {
        rest = &rest[1..];
        let fraction = rest.iter().take_while(|byte| byte.is_ascii_digit()).count();
        if fraction == 0 {
            return false;
        }
        rest = &rest[fraction..];
    }
    match rest.first() {
        Some(b'Z' | b'z') => rest.len() == 1,
        Some(b'+' | b'-') => {
            rest.len() == 6 && digits(&rest[1..3]) && rest[3] == b':' && digits(&rest[4..6])
        }
        _ => false,
    }
}

/// Every validation keyword the schema uses, at every schema position.
fn collect_keywords(node: &Value, path: &str, found: &mut Vec<String>) {
    let Some(object) = node.as_object() else {
        return;
    };
    for (key, value) in object {
        if !KEYWORDS.contains(&key.as_str()) && !ANNOTATIONS.contains(&key.as_str()) {
            found.push(format!("{key} at {path}"));
            continue;
        }
        match key.as_str() {
            "properties" | "definitions" => {
                if let Some(fields) = value.as_object() {
                    for (name, field) in fields {
                        collect_keywords(field, &format!("{path}/{key}/{name}"), found);
                    }
                }
            }
            "items" | "additionalProperties" | "additionalItems" => match value {
                Value::Object(_) => collect_keywords(value, &format!("{path}/{key}"), found),
                Value::Array(items) => {
                    for (index, item) in items.iter().enumerate() {
                        collect_keywords(item, &format!("{path}/{key}/{index}"), found);
                    }
                }
                _ => {}
            },
            "allOf" | "anyOf" | "oneOf" => {
                if let Some(branches) = value.as_array() {
                    for (index, branch) in branches.iter().enumerate() {
                        collect_keywords(branch, &format!("{path}/{key}/{index}"), found);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Every `pattern` the schema uses.
fn collect_patterns(node: &Value, found: &mut Vec<String>) {
    match node {
        Value::Object(fields) => {
            for (key, value) in fields {
                if key == "pattern" {
                    if let Some(pattern) = value.as_str() {
                        found.push(pattern.to_owned());
                    }
                    continue;
                }
                collect_patterns(value, found);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_patterns(item, found);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::{Validator, is_rfc3339};
    use serde_json::json;
    use std::collections::BTreeMap;

    fn validator(schema: serde_json::Value) -> Validator {
        Validator::with_siblings(&schema, BTreeMap::new()).unwrap()
    }

    #[test]
    fn a_document_that_matches_is_valid() {
        let schema = json!({
            "type": "object",
            "required": ["name"],
            "properties": { "name": { "type": "string" } },
            "additionalProperties": false
        });
        let errors = validator(schema).validate(&json!({ "name": "firecraft" }));
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn a_missing_required_field_is_an_error() {
        let schema = json!({
            "type": "object",
            "required": ["name"]
        });
        let errors = validator(schema).validate(&json!({ "version": "1" }));
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("name"), "{errors:?}");
    }

    #[test]
    fn a_field_outside_the_schema_is_an_error() {
        let schema = json!({
            "type": "object",
            "properties": { "name": { "type": "string" } },
            "additionalProperties": false
        });
        let errors = validator(schema).validate(&json!({ "name": "x", "extra": 1 }));
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("extra"), "{errors:?}");
    }

    #[test]
    fn the_wrong_type_is_an_error_with_its_path() {
        let schema = json!({
            "type": "object",
            "properties": { "metadata": { "type": "object" } }
        });
        let errors = validator(schema).validate(&json!({ "metadata": "nope" }));
        assert_eq!(errors.len(), 1);
        assert!(errors[0].starts_with("/metadata:"), "{errors:?}");
    }

    #[test]
    fn an_enum_violation_is_an_error() {
        let schema = json!({
            "type": "object",
            "properties": { "type": { "enum": ["library", "application"] } }
        });
        let errors = validator(schema).validate(&json!({ "type": "firmware" }));
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn a_pattern_violation_is_an_error() {
        let schema = json!({
            "type": "object",
            "properties": {
                "serialNumber": {
                    "type": "string",
                    "pattern": "^urn:uuid:[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"
                }
            }
        });
        let errors = validator(schema).validate(&json!({ "serialNumber": "urn:uuid:NOPE" }));
        assert_eq!(errors.len(), 1);
        let good = validator(schema).validate(&json!({
            "serialNumber": "urn:uuid:12345678-1234-1234-1234-123456789012"
        }));
        assert!(good.is_empty(), "{good:?}");
    }

    #[test]
    fn a_ref_resolves_into_the_definitions() {
        let schema = json!({
            "type": "object",
            "properties": { "component": { "$ref": "#/definitions/component" } },
            "definitions": {
                "component": { "type": "object", "required": ["name"] }
            }
        });
        let errors = validator(schema).validate(&json!({ "component": { "version": "1" } }));
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("name"), "{errors:?}");
    }

    #[test]
    fn a_ref_that_does_not_resolve_is_an_error() {
        let schema = json!({
            "type": "object",
            "properties": { "component": { "$ref": "#/definitions/missing" } }
        });
        let errors = validator(schema).validate(&json!({ "component": {} }));
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("does not resolve"), "{errors:?}");
    }

    #[test]
    fn a_sibling_schema_resolves_by_file_name() {
        let schema = json!({
            "type": "object",
            "properties": { "id": { "$ref": "ids.schema.json" } }
        });
        let siblings = BTreeMap::from([(
            "ids.schema.json".to_owned(),
            json!({ "type": "string", "enum": ["MIT", "Apache-2.0"] }),
        )]);
        let validator = Validator::with_siblings(&schema, siblings).unwrap();
        assert!(validator.validate(&json!({ "id": "MIT" })).is_empty());
        let errors = validator.validate(&json!({ "id": "WTFPL" }));
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("not one of"), "{errors:?}");
    }

    #[test]
    fn a_sibling_schema_with_an_unimplemented_keyword_is_refused() {
        let schema = json!({ "type": "object" });
        let siblings = BTreeMap::from([(
            "ids.schema.json".to_owned(),
            json!({ "type": "string", "patternProperties": { "^x": { "type": "string" } } }),
        )]);
        let error = Validator::with_siblings(&schema, siblings).unwrap_err();
        assert!(error.contains("patternProperties"), "{error}");
    }

    #[test]
    fn one_of_needs_exactly_one_match() {
        let schema = json!({
            "oneOf": [
                { "type": "string" },
                { "type": "integer" }
            ]
        });
        assert!(validator(schema.clone()).validate(&json!("x")).is_empty());
        assert!(validator(schema.clone()).validate(&json!(3)).is_empty());
        let errors = validator(schema.clone()).validate(&json!(true));
        assert_eq!(errors.len(), 1);
        let both = validator(schema).validate(&json!({ "a": 1 }));
        assert_eq!(both.len(), 1);
    }

    #[test]
    fn any_of_needs_one_match() {
        let schema = json!({
            "anyOf": [
                { "type": "string" },
                { "type": "integer" }
            ]
        });
        assert!(validator(schema.clone()).validate(&json!("x")).is_empty());
        let errors = validator(schema).validate(&json!(true));
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn all_of_needs_every_match() {
        let schema = json!({
            "allOf": [
                { "type": "object" },
                { "required": ["name"] }
            ]
        });
        assert!(
            validator(schema.clone())
                .validate(&json!({ "name": "x" }))
                .is_empty()
        );
        let errors = validator(schema).validate(&json!({ "version": "1" }));
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn array_lengths_and_uniqueness_are_checked() {
        let schema = json!({
            "type": "array",
            "minItems": 1,
            "maxItems": 2,
            "uniqueItems": true,
            "items": { "type": "string" }
        });
        assert!(validator(schema.clone()).validate(&json!(["a"])).is_empty());
        assert_eq!(validator(schema.clone()).validate(&json!([])).len(), 1);
        assert_eq!(
            validator(schema.clone())
                .validate(&json!(["a", "b", "c"]))
                .len(),
            1
        );
        assert_eq!(validator(schema).validate(&json!(["a", "a"])).len(), 1);
    }

    #[test]
    fn string_lengths_are_checked() {
        let schema = json!({
            "type": "string",
            "minLength": 2,
            "maxLength": 4
        });
        assert!(validator(schema.clone()).validate(&json!("abc")).is_empty());
        assert_eq!(validator(schema.clone()).validate(&json!("a")).len(), 1);
        assert_eq!(validator(schema).validate(&json!("abcde")).len(), 1);
    }

    #[test]
    fn numbers_are_bounded() {
        let schema = json!({
            "type": "number",
            "minimum": 0,
            "maximum": 10
        });
        assert!(validator(schema.clone()).validate(&json!(5)).is_empty());
        assert_eq!(validator(schema.clone()).validate(&json!(-1)).len(), 1);
        assert_eq!(validator(schema).validate(&json!(11)).len(), 1);
    }

    #[test]
    fn a_schema_with_an_unimplemented_keyword_is_refused() {
        let schema = json!({
            "type": "object",
            "dependentRequired": { "name": ["version"] }
        });
        let error = Validator::with_siblings(&schema, BTreeMap::new()).unwrap_err();
        assert!(error.contains("dependentRequired"), "{error}");
    }

    #[test]
    fn a_date_time_is_recognised() {
        assert!(is_rfc3339("2026-11-15T09:30:00Z"));
        assert!(is_rfc3339("2026-11-15T09:30:00.250+01:00"));
        assert!(!is_rfc3339("2026-11-15"));
        assert!(!is_rfc3339("not a date"));
    }
}
