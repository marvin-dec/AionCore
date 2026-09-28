//! Migration 045 seeds StarCLI as a builtin ACP agent.
//!
//! StarCLI is a global-install CLI (`npm install -g starcli` → `star` on PATH),
//! not an npx-bridged Registry distribution. The assertions pin the launch argv
//! (direct `star acp`), the binary_name used for PATH detection, and that
//! unprobed handshake fields stay NULL until a live connection populates them.

use aionui_db::{IAgentMetadataRepository, SqliteAgentMetadataRepository, init_database_memory};

const BACKEND: &str = "starcli";

#[tokio::test]
async fn seeds_starcli_as_a_direct_cli_builtin() {
    // Using snake_case function name per Rust convention; product name is StarCLI.
    let db = init_database_memory().await.unwrap();
    let repo = SqliteAgentMetadataRepository::new(db.pool().clone());

    let row = repo
        .find_builtin_by_backend(BACKEND)
        .await
        .unwrap()
        .expect("StarCLI is seeded by migration 045");

    assert_eq!(row.id, "b1a7c5f2");
    assert_eq!(row.user_id, None, "builtin rows are machine-level, user_id stays NULL");
    assert_eq!(row.name, "StarCLI");
    assert_eq!(row.agent_type, "acp");
    assert_eq!(row.agent_source, "builtin");
    assert!(row.enabled);
    assert_eq!(
        row.icon.as_deref(),
        Some("/api/assets/logos/acp-registry/starcli.svg"),
        "icon points to the bundled SVG asset"
    );

    // StarCLI is launched directly — no npx bridge, no version pin.
    assert_eq!(row.command.as_deref(), Some("star"));
    assert_eq!(row.args.as_deref(), Some(r#"["acp"]"#));
    assert_eq!(row.env.as_deref(), Some("[]"));

    // PATH detection looks for the global binary `star`.
    let source: serde_json::Value =
        serde_json::from_str(row.agent_source_info.as_deref().expect("agent_source_info")).unwrap();
    assert_eq!(source["binary_name"], "star");
    assert!(
        source.get("bridge_binary").is_none(),
        "StarCLI is launched directly, no bridge binary"
    );
}

/// Capabilities are seeded with a conservative baseline; `auth_methods` and
/// `yolo_id` stay NULL until a live probe confirms the exact handshake shape.
/// Neither may be listed in the migration's `ON CONFLICT DO UPDATE` set — a
/// re-seed must not reset what a live handshake later teaches this install.
#[tokio::test]
async fn seeds_baseline_capabilities_and_leaves_unprobed_fields_null() {
    let db = init_database_memory().await.unwrap();
    let repo = SqliteAgentMetadataRepository::new(db.pool().clone());

    let row = repo
        .find_builtin_by_backend(BACKEND)
        .await
        .unwrap()
        .expect("StarCLI is seeded");

    let caps: serde_json::Value =
        serde_json::from_str(row.agent_capabilities.as_deref().expect("agent_capabilities seeded")).unwrap();
    assert_eq!(caps["load_session"], true);
    assert_eq!(
        caps["mcp_capabilities"]["stdio"], true,
        "Team must route it to the MCP transport"
    );
    assert_eq!(caps["mcp_capabilities"]["http"], true);
    assert_eq!(caps["mcp_capabilities"]["sse"], true);
    assert!(
        row.agent_capabilities.as_deref().unwrap().contains("load_session")
            && !row.agent_capabilities.as_deref().unwrap().contains("loadSession"),
        "handshake columns are stored snake_case (migration 003 contract)"
    );

    assert_eq!(
        row.auth_methods, None,
        "auth_methods not probed yet; nothing is synthesized"
    );
    assert_eq!(
        row.yolo_id, None,
        "yolo_id not confirmed by probe yet"
    );
    assert_eq!(
        row.native_skills_dirs, None,
        "no project-relative skills directory is documented"
    );

    let policy: serde_json::Value =
        serde_json::from_str(row.behavior_policy.as_deref().expect("behavior_policy")).unwrap();
    assert_eq!(policy["supports_side_question"], false);
    assert!(
        policy.get("supports_team").is_none(),
        "no negative team flag (retired by 033)"
    );
}

/// Bad path: the CLI binary name and the npm package are not backends. A lookup
/// by either must miss, so nothing can accidentally seed a second row under an
/// alias and split the agent's identity.
#[tokio::test]
async fn aliases_are_not_registered_as_backends() {
    let db = init_database_memory().await.unwrap();
    let repo = SqliteAgentMetadataRepository::new(db.pool().clone());

    for alias in ["star", "star-cli", "star_cli", "@star/cli"] {
        assert!(
            repo.find_builtin_by_backend(alias).await.unwrap().is_none(),
            "{alias} must not resolve to a builtin row"
        );
    }
}
