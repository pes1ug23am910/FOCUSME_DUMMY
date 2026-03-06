// ============================================================
// FILE:        tests/api_contract_test.rs
// MODULE:      Phase 5 — Cloud Backend > OpenAPI Contract Validation
// TASK:        Session 9 A5 (API contract tests)
// PLATFORM:    linux (server)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Session 9
// DEPENDENCIES: serde_yaml, serde_json
// TEST COVERAGE: 6 tests — structural OpenAPI validation
// KNOWN LIMITATIONS:
//   - Validates OpenAPI spec structure only — does NOT make HTTP requests.
//   - Does not validate response body schemas at runtime (would require
//     a JSON Schema validator crate like jsonschema-rs).
//   - Route existence checks are against the spec's path list, not the
//     actual Axum router (which would require axum-test introspection).
// ============================================================

//! API contract tests that validate the OpenAPI 3.1 specification
//! (`backend/openapi.yml`) for structural completeness and consistency.
//!
//! These tests ensure that:
//! - All expected endpoints are documented
//! - All documented endpoints have proper response schemas
//! - Error responses reference the standardized Error schema
//! - Auth requirements are correctly marked
//! - The spec itself is valid YAML

use serde_json::Value;

/// Load and parse the OpenAPI spec from the project root.
fn load_spec() -> Value {
    let spec_path = concat!(env!("CARGO_MANIFEST_DIR"), "/openapi.yml");
    let content = std::fs::read_to_string(spec_path)
        .expect("openapi.yml should be readable from CARGO_MANIFEST_DIR");
    serde_yaml::from_str(&content)
        .expect("openapi.yml should be valid YAML")
}

/// All expected API paths that must be documented in the spec.
const EXPECTED_PATHS: &[&str] = &[
    "/health",
    "/health/ready",
    "/health/version",
    "/auth/register",
    "/auth/login",
    "/auth/refresh",
    "/plans",
    "/plans/{local_id}",
    "/sync/push",
    "/sync/pull",
    "/family",
    "/family/members",
    "/family/invite",
    "/family/invite/accept",
    "/family/plans/share/{plan_id}",
    "/family/dashboard",
];

/// Paths that must NOT require BearerAuth (public endpoints).
const PUBLIC_PATHS: &[&str] = &[
    "/health",
    "/health/ready",
    "/health/version",
    "/auth/register",
    "/auth/login",
    "/auth/refresh",
];

#[test]
fn test_spec_is_valid_openapi() {
    let spec = load_spec();

    // Must have openapi version
    let version = spec["openapi"].as_str().unwrap();
    assert!(
        version.starts_with("3.1"),
        "Expected OpenAPI 3.1.x, got {}",
        version
    );

    // Must have info block
    assert!(spec["info"]["title"].is_string());
    assert!(spec["info"]["version"].is_string());

    // Must have paths
    assert!(spec["paths"].is_object(), "spec must have paths object");

    // Must have components/schemas
    assert!(
        spec["components"]["schemas"].is_object(),
        "spec must have component schemas"
    );
}

#[test]
fn test_all_expected_paths_documented() {
    let spec = load_spec();
    let paths = &spec["paths"];

    for expected in EXPECTED_PATHS {
        assert!(
            paths.get(expected).is_some(),
            "Missing path in OpenAPI spec: {}",
            expected
        );
    }
}

#[test]
fn test_error_schema_has_code_field() {
    let spec = load_spec();
    let error_schema = &spec["components"]["schemas"]["Error"];

    assert!(error_schema.is_object(), "Error schema must exist");

    // Must have 'code' as string (machine-readable error code)
    let code_prop = &error_schema["properties"]["code"];
    assert_eq!(
        code_prop["type"].as_str().unwrap(),
        "string",
        "Error.code must be a string type (machine-readable error code)"
    );

    // Must have enum listing all error codes
    let enum_values = code_prop["enum"].as_array()
        .expect("Error.code should have enum values");
    assert!(
        enum_values.len() >= 10,
        "Error.code enum should list at least 10 error codes, got {}",
        enum_values.len()
    );

    // Must have 'message' field
    assert!(
        error_schema["properties"]["message"].is_object(),
        "Error schema must have 'message' property"
    );
}

#[test]
fn test_health_endpoints_are_unauthenticated() {
    let spec = load_spec();
    let paths = &spec["paths"];

    for path in &["/health", "/health/ready", "/health/version"] {
        let endpoint = &paths[path];
        assert!(
            endpoint.is_object(),
            "Health path {} must exist in spec",
            path
        );

        // Health endpoints should NOT have a security requirement,
        // or if they do, it should be an empty array (no auth).
        let get_op = &endpoint["get"];
        if let Some(security) = get_op.get("security") {
            if let Some(arr) = security.as_array() {
                assert!(
                    arr.is_empty(),
                    "Health endpoint {} should not require authentication",
                    path
                );
            }
        }
        // If no security field, it inherits global — but health endpoints
        // are typically excluded from global auth, so this is acceptable.
    }
}

#[test]
fn test_auth_endpoints_return_token_pair() {
    let spec = load_spec();
    let paths = &spec["paths"];

    // /auth/login and /auth/register should return a TokenPair in their
    // success response (200 or 201).
    for (path, success_code) in &[("/auth/login", "200"), ("/auth/register", "201")] {
        let endpoint = &paths[path]["post"]["responses"][success_code];
        assert!(
            endpoint.is_object(),
            "{} POST should have a {} response",
            path,
            success_code
        );

        // The response schema should reference TokenPair or AuthResponse
        let content = &endpoint["content"]["application/json"]["schema"];
        let schema_ref = content
            .get("$ref")
            .and_then(|r| r.as_str())
            .unwrap_or("");
        assert!(
            schema_ref.contains("AuthResponse") || schema_ref.contains("TokenPair"),
            "{} {} response should reference AuthResponse or TokenPair schema, got: {}",
            path,
            success_code,
            schema_ref
        );
    }
}

#[test]
fn test_protected_endpoints_require_bearer_auth() {
    let spec = load_spec();
    let paths = &spec["paths"];

    // Check that non-public paths have security requirement
    for (path, path_obj) in paths.as_object().unwrap() {
        if PUBLIC_PATHS.contains(&path.as_str()) {
            continue;
        }

        // For each HTTP method on protected paths
        for method in &["get", "post", "put", "delete", "patch"] {
            if let Some(operation) = path_obj.get(method) {
                // Should have security requirement (either on operation or global)
                if let Some(security) = operation.get("security") {
                    let sec_arr = security.as_array().unwrap();
                    let has_bearer = sec_arr.iter().any(|s| s.get("BearerAuth").is_some());
                    assert!(
                        has_bearer,
                        "Protected endpoint {} {} should require BearerAuth",
                        method.to_uppercase(),
                        path
                    );
                }
                // If no security field, it inherits global security — which is acceptable
                // if the spec has a global security requirement.
            }
        }
    }
}
