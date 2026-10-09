use assert_cmd::Command;
use serde_json::{Value, json};
use tempfile::TempDir;

fn mcp_cmd() -> Command {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("markplane");
    cmd.arg("mcp");
    cmd
}

/// Initialize a markplane project in a tempdir and return the tempdir.
fn setup_project() -> TempDir {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join(".markplane");
    markplane_core::Project::init(root, "Test Project", "A test project").unwrap();
    tmp
}

/// Helper: send a JSON-RPC request via stdin and return the parsed response.
fn send_request(tmp: &TempDir, request: &Value) -> Value {
    let input = format!("{}\n", serde_json::to_string(request).unwrap());
    let output = mcp_cmd()
        .arg("--project")
        .arg(tmp.path().to_str().unwrap())
        .write_stdin(input)
        .output()
        .expect("failed to run markplane mcp");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // The response is on the first non-empty line
    for line in stdout.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            return serde_json::from_str(trimmed).expect("failed to parse JSON-RPC response");
        }
    }
    panic!("No response received from MCP server. stdout: {}", stdout);
}

/// Extract the ID from an MCP markplane_add tool response.
/// The response text is JSON like: {"id":"TASK-k7x9m","title":"..."}
fn extract_id_from_response(response: &Value) -> String {
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let result: Value = serde_json::from_str(text).unwrap();
    result["id"].as_str().unwrap().to_string()
}

/// Assert that a tool call returned an error via isError: true in the result.
/// Tool-level errors are returned as successful JSON-RPC responses with isError flag.
fn assert_tool_error(response: &Value, msg: &str) {
    assert!(
        response["error"].is_null(),
        "{}: should not be a JSON-RPC error",
        msg
    );
    assert_eq!(
        response["result"]["isError"], true,
        "{}: should have isError: true",
        msg
    );
}

// ── Initialize ──────────────────────────────────────────────────────────

#[test]
fn test_initialize() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }),
    );

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(
        response["result"]["serverInfo"]["name"]
            .as_str()
            .unwrap()
            .contains("markplane")
    );
    assert!(response["result"]["capabilities"]["tools"].is_object());
    assert!(response["result"]["capabilities"]["resources"].is_object());
    // Protocol version should be 2025-11-25
    assert_eq!(response["result"]["protocolVersion"], "2025-11-25");
    // serverInfo should include a description
    let description = response["result"]["serverInfo"]["description"]
        .as_str()
        .unwrap();
    assert!(description.contains("markdown-first"));
    // instructions field should be present and include project name
    let instructions = response["result"]["instructions"].as_str().unwrap();
    assert!(!instructions.is_empty());
    assert!(instructions.contains("Test Project"));
    assert!(instructions.contains("TASK-NNN"));
    assert!(instructions.contains("EPIC-NNN"));
    assert!(instructions.contains("PLAN-NNN"));
    assert!(instructions.contains("NOTE-NNN"));
    // instructions should describe file editing workflow
    assert!(instructions.contains("File Editing"));
    assert!(instructions.contains("free-form markdown"));
}

#[test]
fn test_startup_sync_with_missing_entity_dirs() {
    // On a fresh clone, directories holding only gitignored files don't exist.
    let tmp = setup_project();
    let root = tmp.path().join(".markplane");
    for dir in ["backlog", "roadmap", "plans", "notes", ".context"] {
        std::fs::remove_dir_all(root.join(dir)).unwrap();
    }

    let input = format!("{}\n", json!({"jsonrpc": "2.0", "id": 1, "method": "ping"}));
    let output = mcp_cmd()
        .arg("--project")
        .arg(tmp.path().to_str().unwrap())
        .write_stdin(input)
        .output()
        .expect("failed to run markplane mcp");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("sync warning"), "stderr: {stderr}");
    assert!(root.join("plans/INDEX.md").is_file());
    assert!(root.join(".context/summary.md").is_file());
}

// ── Ping ─────────────────────────────────────────────────────────────────

#[test]
fn test_ping() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "ping",
            "params": {}
        }),
    );

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(response["result"].is_object());
    assert!(response["error"].is_null());
}

// ── Method Not Found ─────────────────────────────────────────────────────

#[test]
fn test_unknown_method_returns_error() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "nonexistent/method",
            "params": {}
        }),
    );

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 3);
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32601); // METHOD_NOT_FOUND
    assert!(
        response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Method not found")
    );
}

// ── Malformed JSON ───────────────────────────────────────────────────────

#[test]
fn test_malformed_json_returns_parse_error() {
    let tmp = setup_project();
    let output = mcp_cmd()
        .arg("--project")
        .arg(tmp.path().to_str().unwrap())
        .write_stdin("this is not valid json\n")
        .output()
        .expect("failed to run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let response: Value = stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l.trim()).unwrap())
        .next()
        .expect("no response");

    assert_eq!(response["error"]["code"], -32700); // PARSE_ERROR
}

// ── Tools List ───────────────────────────────────────────────────────────

#[test]
fn test_tools_list() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/list",
            "params": {}
        }),
    );

    assert!(response["error"].is_null());
    let tools = &response["result"]["tools"];
    assert!(tools.is_array());
    let tool_names: Vec<&str> = tools
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(tool_names.contains(&"markplane_summary"));
    assert!(tool_names.contains(&"markplane_query"));
    assert!(tool_names.contains(&"markplane_show"));
    assert!(tool_names.contains(&"markplane_add"));
    assert!(tool_names.contains(&"markplane_update"));
    assert!(
        !tool_names.contains(&"markplane_start"),
        "markplane_start should be removed"
    );
    assert!(
        !tool_names.contains(&"markplane_done"),
        "markplane_done should be removed"
    );
    assert!(tool_names.contains(&"markplane_sync"));
    assert!(tool_names.contains(&"markplane_context"));
    assert!(tool_names.contains(&"markplane_graph"));
    assert!(tool_names.contains(&"markplane_promote"));
    assert!(tool_names.contains(&"markplane_plan"));
    assert!(tool_names.contains(&"markplane_link"));
    assert!(tool_names.contains(&"markplane_check"));
    assert!(tool_names.contains(&"markplane_archive"));
    assert!(tool_names.contains(&"markplane_unarchive"));
    assert!(tool_names.contains(&"markplane_move"));
    assert!(tool_names.contains(&"markplane_comment"));
    assert!(tool_names.contains(&"markplane_resolve"));
    assert!(
        !tool_names.contains(&"markplane_stale"),
        "markplane_stale should be removed"
    );
    assert_eq!(
        tool_names.len(),
        17,
        "Expected 17 tools, got: {:?}",
        tool_names
    );
}

// ── Resources List ───────────────────────────────────────────────────────

#[test]
fn test_resources_list() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "resources/list",
            "params": {}
        }),
    );

    assert!(response["error"].is_null());
    let resources = &response["result"]["resources"];
    assert!(resources.is_array());
    let uris: Vec<&str> = resources
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["uri"].as_str().unwrap())
        .collect();
    assert!(uris.contains(&"markplane://summary"));
    assert!(uris.contains(&"markplane://active-work"));
    assert!(uris.contains(&"markplane://blocked"));
    assert!(uris.contains(&"markplane://maps"));

    // PLAN and NOTE resource templates should be present
    let templates = &response["result"]["resourceTemplates"];
    assert!(templates.is_array());
    let template_uris: Vec<&str> = templates
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["uriTemplate"].as_str().unwrap())
        .collect();
    assert!(template_uris.contains(&"markplane://task/{id}"));
    assert!(template_uris.contains(&"markplane://epic/{id}"));
    assert!(template_uris.contains(&"markplane://plan/{id}"));
    assert!(template_uris.contains(&"markplane://note/{id}"));
    assert!(template_uris.contains(&"markplane://map/{id}"));
}

// ── Tool: markplane_summary ──────────────────────────────────────────────

#[test]
fn test_tool_summary() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 20,
            "method": "tools/call",
            "params": {
                "name": "markplane_summary",
                "arguments": {}
            }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Test Project"));
    assert!(text.contains("Total items: 0") || text.contains("Backlog: 0 items"));
}

// ── Tool: markplane_add ──────────────────────────────────────────────────

#[test]
fn test_tool_add() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 21,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": {
                    "title": "Build the API",
                    "type": "feature",
                    "priority": "high",
                    "effort": "large",
                    "tags": ["api", "backend"]
                }
            }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let result: Value = serde_json::from_str(text).unwrap();
    let id = result["id"].as_str().unwrap();
    assert!(
        id.starts_with("TASK-"),
        "ID should start with TASK-, got: {}",
        id
    );
    assert_eq!(
        id.len(),
        10,
        "ID should be 10 chars (TASK-xxxxx), got: {}",
        id
    );
    assert_eq!(result["title"], "Build the API");
}

#[test]
fn test_tool_add_missing_title() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 22,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": {}
            }
        }),
    );

    // Should be an error because title is required
    assert_tool_error(&response, "missing title");
}

#[test]
fn test_tool_add_epic() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 23,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": {
                    "title": "Phase 1 rollout",
                    "kind": "epic",
                    "priority": "high"
                }
            }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let result: Value = serde_json::from_str(text).unwrap();
    let id = result["id"].as_str().unwrap();
    assert!(
        id.starts_with("EPIC-"),
        "ID should start with EPIC-, got: {}",
        id
    );
    assert_eq!(result["title"], "Phase 1 rollout");
}

#[test]
fn test_tool_add_note() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 24,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": {
                    "title": "Research caching strategies",
                    "kind": "note",
                    "note_type": "research",
                    "tags": ["perf"]
                }
            }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let result: Value = serde_json::from_str(text).unwrap();
    let id = result["id"].as_str().unwrap();
    assert!(
        id.starts_with("NOTE-"),
        "ID should start with NOTE-, got: {}",
        id
    );
    assert_eq!(result["title"], "Research caching strategies");
}

#[test]
fn test_tool_add_plan_standalone() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 24,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": {
                    "title": "Standalone MCP plan",
                    "kind": "plan",
                    "tags": ["ready-for-agent"]
                }
            }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let result: Value = serde_json::from_str(text).unwrap();
    let id = result["id"].as_str().unwrap();
    assert!(
        id.starts_with("PLAN-"),
        "ID should start with PLAN-, got: {}",
        id
    );
    assert_eq!(result["title"], "Standalone MCP plan");

    let content =
        std::fs::read_to_string(tmp.path().join(format!(".markplane/plans/items/{}.md", id)))
            .unwrap();
    assert!(content.contains("ready-for-agent"));
}

#[test]
fn test_tool_add_with_plan_parent() {
    let tmp = setup_project();

    let plan_resp = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": {
                    "title": "Parent plan",
                    "kind": "plan"
                }
            }
        }),
    );
    let plan_id = extract_id_from_response(&plan_resp);

    let add_resp = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": {
                    "title": "Under plan via MCP",
                    "plan": plan_id,
                    "tags": ["ready-for-agent"]
                }
            }
        }),
    );
    assert!(add_resp["error"].is_null());
    let task_id = extract_id_from_response(&add_resp);

    let task_content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    assert!(task_content.contains(&plan_id));
    assert!(task_content.contains("ready-for-agent"));

    let plan_content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/plans/items/{}.md", plan_id)),
    )
    .unwrap();
    assert!(plan_content.contains(&task_id));
}

#[test]
fn test_tool_query_filter_by_plan() {
    let tmp = setup_project();
    let root = tmp.path().join(".markplane");
    let project = markplane_core::Project::new(root);

    let plan = project
        .create_plan("Query plan", vec![], vec![], None, None)
        .unwrap();
    let other = project
        .create_plan("Other plan", vec![], vec![], None, None)
        .unwrap();

    let task_a = project
        .create_task(
            "Under plan",
            "feature",
            markplane_core::Priority::Medium,
            markplane_core::Effort::Small,
            None,
            None,
            vec![],
            None,
            None,
        )
        .unwrap();
    project
        .link_items(
            &task_a.id,
            &plan.id,
            markplane_core::LinkRelation::Plan,
            markplane_core::LinkAction::Add,
        )
        .unwrap();

    let task_b = project
        .create_task(
            "Under other",
            "feature",
            markplane_core::Priority::Medium,
            markplane_core::Effort::Small,
            None,
            None,
            vec![],
            None,
            None,
        )
        .unwrap();
    project
        .link_items(
            &task_b.id,
            &other.id,
            markplane_core::LinkRelation::Plan,
            markplane_core::LinkAction::Add,
        )
        .unwrap();

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 26,
            "method": "tools/call",
            "params": {
                "name": "markplane_query",
                "arguments": {
                    "kind": "tasks",
                    "plan": plan.id
                }
            }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let results: Value = serde_json::from_str(text).unwrap();
    let arr = results.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], task_a.id);
    assert_eq!(arr[0]["title"], "Under plan");
}

#[test]
fn test_tool_add_map() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 240,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": {
                    "title": "MCP map",
                    "kind": "map",
                    "tags": ["wayfinder"]
                }
            }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let result: Value = serde_json::from_str(text).unwrap();
    let id = result["id"].as_str().unwrap();
    assert!(
        id.starts_with("MAP-"),
        "ID should start with MAP-, got: {}",
        id
    );
    assert_eq!(result["title"], "MCP map");

    let path = tmp.path().join(format!(".markplane/maps/items/{}.md", id));
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("status: active"));
    assert!(content.contains("## Destination"));
    assert!(content.contains("tags:"));
    assert!(content.contains("wayfinder"));
}

#[test]
fn test_tool_add_invalid_kind() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 25,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": {
                    "title": "Bad kind",
                    "kind": "widget"
                }
            }
        }),
    );

    assert_tool_error(&response, "invalid kind");
}

#[test]
fn test_tool_add_with_body_matches_cli() {
    let tmp = setup_project();
    let body_content = "## Description\n\nShared body content for parity.\n";
    let body_path = tmp.path().join("body.md");
    std::fs::write(&body_path, body_content).unwrap();

    let cli_out = assert_cmd::cargo::cargo_bin_cmd!("markplane")
        .current_dir(tmp.path())
        .args([
            "add",
            "Parity title",
            "--body-file",
            body_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        cli_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&cli_out.stderr)
    );
    let cli_stdout = String::from_utf8_lossy(&cli_out.stdout);
    let cli_id = cli_stdout
        .split_whitespace()
        .find(|w| w.starts_with("TASK-"))
        .expect("CLI ID")
        .to_string();

    let mcp_response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 26,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": {
                    "title": "Parity title",
                    "body": body_content
                }
            }
        }),
    );
    assert!(mcp_response["error"].is_null());
    let mcp_id = extract_id_from_response(&mcp_response);

    let cli_file = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", cli_id)),
    )
    .unwrap();
    let mcp_file = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", mcp_id)),
    )
    .unwrap();

    // Bodies (after frontmatter) must match for the same title + body input.
    let cli_body = cli_file.splitn(3, "---").nth(2).unwrap_or("").trim();
    let mcp_body = mcp_file.splitn(3, "---").nth(2).unwrap_or("").trim();
    assert_eq!(cli_body, mcp_body);
    assert!(cli_body.contains("Shared body content for parity."));
}

// ── Tool: markplane_comment ──────────────────────────────────────────────

#[test]
fn test_tool_comment_matches_cli() {
    let tmp = setup_project();
    let comment_text = "Parity comment body.\n";

    let cli_add = assert_cmd::cargo::cargo_bin_cmd!("markplane")
        .current_dir(tmp.path())
        .args(["add", "Comment parity A"])
        .output()
        .unwrap();
    assert!(cli_add.status.success());
    let cli_id = String::from_utf8_lossy(&cli_add.stdout)
        .split_whitespace()
        .find(|w| w.starts_with("TASK-"))
        .expect("CLI ID")
        .to_string();

    let mcp_add = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 27,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "Comment parity B" }
            }
        }),
    );
    assert!(mcp_add["error"].is_null());
    let mcp_id = extract_id_from_response(&mcp_add);

    let body_path = tmp.path().join("comment.md");
    std::fs::write(&body_path, comment_text).unwrap();

    let cli_comment = assert_cmd::cargo::cargo_bin_cmd!("markplane")
        .current_dir(tmp.path())
        .env("USER", "alice")
        .args([
            "comment",
            &cli_id,
            "--body-file",
            body_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        cli_comment.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&cli_comment.stderr)
    );

    let mcp_response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 28,
            "method": "tools/call",
            "params": {
                "name": "markplane_comment",
                "arguments": {
                    "id": mcp_id,
                    "body": comment_text,
                    "author": "alice"
                }
            }
        }),
    );
    assert!(
        mcp_response["error"].is_null(),
        "MCP error: {}",
        mcp_response
    );
    assert_ne!(
        mcp_response["result"]["isError"], true,
        "tool error: {}",
        mcp_response["result"]["content"][0]["text"]
    );

    let cli_file = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", cli_id)),
    )
    .unwrap();
    let mcp_file = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", mcp_id)),
    )
    .unwrap();

    let cli_body = cli_file.splitn(3, "---").nth(2).unwrap_or("").trim();
    let mcp_body = mcp_file.splitn(3, "---").nth(2).unwrap_or("").trim();

    // Strip the title line (# Comment parity A/B) so remaining body including comments matches.
    let normalize = |body: &str| -> String {
        let mut lines = body.lines();
        let _title = lines.next();
        lines.collect::<Vec<_>>().join("\n").trim().to_string()
    };
    assert_eq!(normalize(cli_body), normalize(mcp_body));
    assert!(cli_body.contains("## Comments"));
    assert!(cli_body.contains("### "));
    assert!(cli_body.contains("alice"));
    assert!(cli_body.contains("Parity comment body."));
}

// ── Tool: markplane_show ─────────────────────────────────────────────────

#[test]
fn test_tool_show() {
    let tmp = setup_project();
    // Add an item first
    let add_response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "Show me" }
            }
        }),
    );
    let task_id = extract_id_from_response(&add_response);

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 30,
            "method": "tools/call",
            "params": {
                "name": "markplane_show",
                "arguments": { "id": task_id }
            }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains(&task_id));
    assert!(text.contains("Show me"));
}

#[test]
fn test_tool_show_invalid_id() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 31,
            "method": "tools/call",
            "params": {
                "name": "markplane_show",
                "arguments": { "id": "TASK-999" }
            }
        }),
    );

    assert_tool_error(&response, "invalid id");
}

// ── Tool: markplane_query ────────────────────────────────────────────────

#[test]
fn test_tool_query() {
    let tmp = setup_project();
    // Add items
    send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "Task A", "priority": "high" }
            }
        }),
    );
    send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "Task B", "priority": "low" }
            }
        }),
    );

    // Query all
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 40,
            "method": "tools/call",
            "params": {
                "name": "markplane_query",
                "arguments": {}
            }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let items: Vec<Value> = serde_json::from_str(text).unwrap();
    assert_eq!(items.len(), 2);

    // Query with priority filter
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 41,
            "method": "tools/call",
            "params": {
                "name": "markplane_query",
                "arguments": { "priority": ["high"] }
            }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let items: Vec<Value> = serde_json::from_str(text).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["title"], "Task A");
}

#[test]
fn test_tool_query_tasks_include_updated() {
    let tmp = setup_project();
    send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "Check updated" }
            }
        }),
    );

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "tools/call",
            "params": {
                "name": "markplane_query",
                "arguments": {}
            }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let items: Vec<Value> = serde_json::from_str(text).unwrap();
    assert_eq!(items.len(), 1);
    assert!(
        items[0]["updated"].is_string(),
        "Task query should include updated date"
    );
}

#[test]
fn test_tool_query_epics() {
    let tmp = setup_project();
    send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "Epic A", "kind": "epic", "priority": "high" }
            }
        }),
    );

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 43,
            "method": "tools/call",
            "params": {
                "name": "markplane_query",
                "arguments": { "kind": "epics" }
            }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let items: Vec<Value> = serde_json::from_str(text).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["title"], "Epic A");
    assert_eq!(items[0]["status"], "later");
    assert_eq!(items[0]["priority"], "high");
    assert!(
        items[0]["created"].is_string(),
        "epic query should include created date"
    );
    assert!(
        items[0]["updated"].is_string(),
        "epic query should include updated date"
    );
}

#[test]
fn test_tool_query_plans() {
    let tmp = setup_project();
    // Create a task first, then a plan linked to it
    let add_response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "Plan target task" }
            }
        }),
    );
    let task_id = extract_id_from_response(&add_response);

    send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "markplane_plan",
                "arguments": { "task_id": task_id }
            }
        }),
    );

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 44,
            "method": "tools/call",
            "params": {
                "name": "markplane_query",
                "arguments": { "kind": "plans" }
            }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let items: Vec<Value> = serde_json::from_str(text).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["status"], "draft");
    assert!(
        items[0]["updated"].is_string(),
        "Plan query should include updated date"
    );
}

#[test]
fn test_tool_query_notes() {
    let tmp = setup_project();
    send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "Note A", "kind": "note", "note_type": "idea" }
            }
        }),
    );

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 45,
            "method": "tools/call",
            "params": {
                "name": "markplane_query",
                "arguments": { "kind": "notes" }
            }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let items: Vec<Value> = serde_json::from_str(text).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["title"], "Note A");
    assert_eq!(items[0]["type"], "idea");
    assert!(
        items[0]["updated"].is_string(),
        "Note query should include updated date"
    );
}

#[test]
fn test_tool_query_invalid_kind() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 46,
            "method": "tools/call",
            "params": {
                "name": "markplane_query",
                "arguments": { "kind": "widgets" }
            }
        }),
    );

    assert_tool_error(&response, "invalid kind");
}

// ── Tool: markplane_update ───────────────────────────────────────────────

#[test]
fn test_tool_update() {
    let tmp = setup_project();
    let add_response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "Update me" }
            }
        }),
    );
    let task_id = extract_id_from_response(&add_response);

    // Update status
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 50,
            "method": "tools/call",
            "params": {
                "name": "markplane_update",
                "arguments": { "id": task_id, "status": "in-progress" }
            }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("success"));

    // Verify via show
    let show = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 51,
            "method": "tools/call",
            "params": {
                "name": "markplane_show",
                "arguments": { "id": task_id }
            }
        }),
    );
    let text = show["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("in-progress"));
}

// ── Tool: markplane_sync ─────────────────────────────────────────────────

#[test]
fn test_tool_sync() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 80,
            "method": "tools/call",
            "params": {
                "name": "markplane_sync",
                "arguments": {}
            }
        }),
    );

    assert!(response["error"].is_null());
    // Verify context files were created
    assert!(tmp.path().join(".markplane/.context/summary.md").is_file());
}

// ── Tool: markplane_context ──────────────────────────────────────────────

#[test]
fn test_tool_context_default() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 81,
            "method": "tools/call",
            "params": {
                "name": "markplane_context",
                "arguments": {}
            }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Test Project"));
}

#[test]
fn test_tool_context_focus_active_work() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 82,
            "method": "tools/call",
            "params": {
                "name": "markplane_context",
                "arguments": { "focus": "active-work" }
            }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("Active Work"));
}

// ── Tool: markplane_check ────────────────────────────────────────────────

#[test]
fn test_tool_check_clean() {
    let tmp = setup_project();
    send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "Clean item" }
            }
        }),
    );

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 90,
            "method": "tools/call",
            "params": {
                "name": "markplane_check",
                "arguments": {}
            }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains(
        "All cross-references, task statuses, reciprocal links, and dependency graphs are valid"
    ));
}

#[test]
fn test_tool_check_asymmetric_links() {
    let tmp = setup_project();

    // Create two tasks
    let resp1 = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "Blocker" }
            }
        }),
    );
    let id1 = extract_id_from_response(&resp1);

    let resp2 = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "Blocked" }
            }
        }),
    );
    let id2 = extract_id_from_response(&resp2);

    // Link them
    send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "markplane_link",
                "arguments": { "from": id1, "to": id2, "relation": "blocks" }
            }
        }),
    );

    // Manually corrupt: remove depends_on from the blocked task
    let item_path = tmp
        .path()
        .join(format!(".markplane/backlog/items/{}.md", id2));
    let content = std::fs::read_to_string(&item_path).unwrap();
    let content = content.replace(&format!("depends_on:\n- {}", id1), "depends_on: []");
    std::fs::write(&item_path, content).unwrap();

    // Check should detect the asymmetric link
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 91,
            "method": "tools/call",
            "params": {
                "name": "markplane_check",
                "arguments": {}
            }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("asymmetric reciprocal link"),
        "Expected asymmetric link report, got: {}",
        text
    );
    assert!(text.contains(&id1));
    assert!(text.contains(&id2));
}

// ── Tool: markplane_graph ────────────────────────────────────────────────

#[test]
fn test_tool_graph() {
    let tmp = setup_project();
    // Create linked items
    let add1 = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "Blocker" }
            }
        }),
    );
    let task1_id = extract_id_from_response(&add1);

    let add2 = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "Blocked" }
            }
        }),
    );
    let task2_id = extract_id_from_response(&add2);

    send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "markplane_link",
                "arguments": { "from": task1_id, "to": task2_id, "relation": "blocks" }
            }
        }),
    );

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 110,
            "method": "tools/call",
            "params": {
                "name": "markplane_graph",
                "arguments": { "id": task1_id }
            }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains(&format!("Reference Graph for {}", task1_id)));
    assert!(text.contains(&task2_id));
}

// ── Tool: markplane_promote ──────────────────────────────────────────────

#[test]
fn test_tool_promote() {
    let tmp = setup_project();
    // Create a note first (directly using core)
    let root = tmp.path().join(".markplane");
    let project = markplane_core::Project::new(root);
    let note = project
        .create_note("Good idea", "idea", vec!["cool".to_string()], None, None)
        .unwrap();

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 120,
            "method": "tools/call",
            "params": {
                "name": "markplane_promote",
                "arguments": { "note_id": note.id, "priority": "high" }
            }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let result: Value = serde_json::from_str(text).unwrap();
    assert!(result["id"].as_str().unwrap().starts_with("TASK-"));
    assert_eq!(result["promoted_from"], note.id);
}

// ── Tool: markplane_plan ─────────────────────────────────────────────────

#[test]
fn test_tool_plan() {
    let tmp = setup_project();
    let add_response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "Need a plan" }
            }
        }),
    );
    let task_id = extract_id_from_response(&add_response);

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 130,
            "method": "tools/call",
            "params": {
                "name": "markplane_plan",
                "arguments": {
                    "task_id": task_id,
                    "title": "My plan",
                    "body": "## Approach\n\nShip it.\n",
                    "tags": ["ready-for-agent"]
                }
            }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let result: Value = serde_json::from_str(text).unwrap();
    assert!(result["id"].as_str().unwrap().starts_with("PLAN-"));
    assert_eq!(result["implements"], task_id);

    let plan_id = result["id"].as_str().unwrap();
    let plan_content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/plans/items/{}.md", plan_id)),
    )
    .unwrap();
    assert!(plan_content.contains("## Approach"));
    assert!(plan_content.contains("Ship it."));
    assert!(plan_content.contains("ready-for-agent"));
}

#[test]
fn test_tool_plan_standalone() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 131,
            "method": "tools/call",
            "params": {
                "name": "markplane_plan",
                "arguments": {
                    "title": "Standalone via markplane_plan",
                    "body": "## Problem Statement\n\nNeed a plan.\n",
                    "tags": ["ready-for-agent"]
                }
            }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let result: Value = serde_json::from_str(text).unwrap();
    let plan_id = result["id"].as_str().unwrap();
    assert!(plan_id.starts_with("PLAN-"));
    assert_eq!(result["title"], "Standalone via markplane_plan");
    assert!(result.get("implements").is_none() || result["implements"].is_null());

    let plan_content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/plans/items/{}.md", plan_id)),
    )
    .unwrap();
    assert!(plan_content.contains("## Problem Statement"));
    assert!(plan_content.contains("Need a plan."));
    assert!(plan_content.contains("ready-for-agent"));
}

#[test]
fn test_tool_plan_standalone_requires_title() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 132,
            "method": "tools/call",
            "params": {
                "name": "markplane_plan",
                "arguments": {
                    "tags": ["ready-for-agent"]
                }
            }
        }),
    );
    assert_tool_error(&response, "standalone plan requires title");
}

// ── Tool: markplane_link ─────────────────────────────────────────────────

#[test]
fn test_tool_link_blocks() {
    let tmp = setup_project();
    let add1 = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "A" }
            }
        }),
    );
    let task1_id = extract_id_from_response(&add1);

    let add2 = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "B" }
            }
        }),
    );
    let task2_id = extract_id_from_response(&add2);

    let link_resp = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "markplane_link",
                "arguments": {
                    "from": task1_id,
                    "to": task2_id,
                    "relation": "blocks"
                }
            }
        }),
    );

    assert!(link_resp["error"].is_null());
    let text = link_resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("success"));
}

#[test]
fn test_tool_link_missing_params() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 140,
            "method": "tools/call",
            "params": {
                "name": "markplane_link",
                "arguments": { "from": "TASK-001" }
            }
        }),
    );

    assert_tool_error(&response, "link missing params");
}

#[test]
fn test_tool_link_epic() {
    let tmp = setup_project();
    let add_task = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "A task" }
            }
        }),
    );
    let task_id = extract_id_from_response(&add_task);

    // Create an epic by writing the file directly (no add_epic MCP tool)
    let epic_id = {
        use std::fs;
        let epic_dir = tmp.path().join(".markplane/roadmap/items");
        fs::create_dir_all(&epic_dir).unwrap();
        let id = "EPIC-test1";
        let content = format!(
            "---\nid: {}\ntitle: Test Epic\nstatus: planned\npriority: medium\ntags: []\ndepends_on: []\ncreated: 2026-01-01\nupdated: 2026-01-01\n---\n",
            id
        );
        fs::write(epic_dir.join(format!("{}.md", id)), content).unwrap();
        id.to_string()
    };

    let link_resp = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "markplane_link",
                "arguments": {
                    "from": task_id,
                    "to": epic_id,
                    "relation": "epic"
                }
            }
        }),
    );

    assert!(link_resp["error"].is_null());
    let text = link_resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("success"));

    // Verify task.epic is set
    let task_content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    assert!(task_content.contains(&epic_id));
}

#[test]
fn test_tool_link_plan_reciprocal() {
    let tmp = setup_project();
    let add_task = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "A task" }
            }
        }),
    );
    let task_id = extract_id_from_response(&add_task);

    let plan_resp = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "markplane_plan",
                "arguments": { "task_id": task_id }
            }
        }),
    );
    assert!(plan_resp["error"].is_null());
    let plan_text = plan_resp["result"]["content"][0]["text"].as_str().unwrap();
    let plan_data: serde_json::Value = serde_json::from_str(plan_text).unwrap();
    let plan_id = plan_data["id"].as_str().unwrap();

    // Verify task has plan set
    let task_content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    assert!(task_content.contains(plan_id));

    // Verify plan has implements set
    let plan_content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/plans/items/{}.md", plan_id)),
    )
    .unwrap();
    assert!(plan_content.contains(&task_id));
}

#[test]
fn test_tool_link_remove() {
    let tmp = setup_project();
    let add1 = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "A" }
            }
        }),
    );
    let task1_id = extract_id_from_response(&add1);

    let add2 = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "B" }
            }
        }),
    );
    let task2_id = extract_id_from_response(&add2);

    // Add link
    send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "markplane_link",
                "arguments": {
                    "from": task1_id,
                    "to": task2_id,
                    "relation": "blocks"
                }
            }
        }),
    );

    // Remove link
    let remove_resp = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "markplane_link",
                "arguments": {
                    "from": task1_id,
                    "to": task2_id,
                    "relation": "blocks",
                    "remove": true
                }
            }
        }),
    );

    assert!(remove_resp["error"].is_null());

    // Verify link is removed
    let task_content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task1_id)),
    )
    .unwrap();
    assert!(!task_content.contains(&task2_id));
}

#[test]
fn test_tool_link_related_bidirectional() {
    let tmp = setup_project();

    // Create a task
    let add_task = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "A task" }
            }
        }),
    );
    let task_id = extract_id_from_response(&add_task);

    // Create an epic
    let add_epic = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "An epic", "kind": "epic" }
            }
        }),
    );
    let epic_id = extract_id_from_response(&add_epic);

    // Link with related
    let link_resp = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "markplane_link",
                "arguments": {
                    "from": task_id,
                    "to": epic_id,
                    "relation": "related"
                }
            }
        }),
    );
    assert!(link_resp["error"].is_null());

    // Verify both files contain the reciprocal link
    let task_content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    let epic_content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/roadmap/items/{}.md", epic_id)),
    )
    .unwrap();
    assert!(
        task_content.contains(&epic_id),
        "Task should have related link to Epic"
    );
    assert!(
        epic_content.contains(&task_id),
        "Epic should have related link to Task"
    );

    // Verify show output includes the related field
    let show_resp = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "markplane_show",
                "arguments": { "id": task_id }
            }
        }),
    );
    let show_text = show_resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        show_text.contains(&epic_id),
        "Show output should contain related epic ID"
    );
}

#[test]
fn test_tool_link_spec() {
    let tmp = setup_project();

    let map_resp = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "Spec map", "kind": "map" }
            }
        }),
    );
    let map_id = extract_id_from_response(&map_resp);

    let epic_resp = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "Published epic", "kind": "epic" }
            }
        }),
    );
    let epic_id = extract_id_from_response(&epic_resp);

    let link_resp = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "markplane_link",
                "arguments": {
                    "from": map_id,
                    "to": epic_id,
                    "relation": "spec"
                }
            }
        }),
    );
    assert!(link_resp["error"].is_null());
    let text = link_resp["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("success"));

    let map_content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/maps/items/{}.md", map_id)),
    )
    .unwrap();
    let map_fm = map_content.split("---").nth(1).unwrap_or("");
    assert!(map_fm.contains("specs:"));
    assert!(map_fm.contains(&epic_id));

    let epic_content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/roadmap/items/{}.md", epic_id)),
    )
    .unwrap();
    let epic_fm = epic_content.split("---").nth(1).unwrap_or("");
    assert!(!epic_fm.contains(&map_id));
}

#[test]
fn test_tools_list_link_includes_spec() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/list",
            "params": {}
        }),
    );
    let tools = response["result"]["tools"].as_array().unwrap();
    let link = tools
        .iter()
        .find(|t| t["name"] == "markplane_link")
        .expect("markplane_link tool");
    let enum_vals = link["inputSchema"]["properties"]["relation"]["enum"]
        .as_array()
        .expect("relation enum");
    assert!(
        enum_vals.iter().any(|v| v.as_str() == Some("spec")),
        "markplane_link relation enum should include spec"
    );
}

// ── Unknown tool ─────────────────────────────────────────────────────────

#[test]
fn test_unknown_tool_returns_error() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 150,
            "method": "tools/call",
            "params": {
                "name": "nonexistent_tool",
                "arguments": {}
            }
        }),
    );

    assert!(response["error"].is_object());
    assert_eq!(
        response["error"]["code"].as_i64().unwrap(),
        -32601,
        "Unknown tool should return METHOD_NOT_FOUND (-32601)"
    );
    assert!(
        response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Unknown tool")
    );
}

// ── Missing params ───────────────────────────────────────────────────────

#[test]
fn test_tools_call_missing_params() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 160,
            "method": "tools/call"
        }),
    );

    assert!(response["error"].is_object());
}

#[test]
fn test_tools_call_missing_name() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 161,
            "method": "tools/call",
            "params": { "arguments": {} }
        }),
    );

    assert!(response["error"].is_object());
}

// ── Resources Read ───────────────────────────────────────────────────────

#[test]
fn test_resource_summary() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 200,
            "method": "resources/read",
            "params": { "uri": "markplane://summary" }
        }),
    );

    assert!(response["error"].is_null());
    let contents = &response["result"]["contents"];
    assert!(contents.is_array());
    assert_eq!(contents[0]["uri"], "markplane://summary");
    assert_eq!(contents[0]["mimeType"], "text/markdown");
    let text = contents[0]["text"].as_str().unwrap();
    assert!(text.contains("Test Project"));
}

#[test]
fn test_resource_active_work() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 201,
            "method": "resources/read",
            "params": { "uri": "markplane://active-work" }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["contents"][0]["text"].as_str().unwrap();
    assert!(text.contains("Active Work"));
    assert!(text.contains("No items currently in progress"));
}

#[test]
fn test_resource_blocked() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 202,
            "method": "resources/read",
            "params": { "uri": "markplane://blocked" }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["contents"][0]["text"].as_str().unwrap();
    assert!(text.contains("Blocked Items"));
    assert!(text.contains("No items with unresolved dependencies"));
}

#[test]
fn test_resource_task_item() {
    let tmp = setup_project();
    // Create an item first
    let root = tmp.path().join(".markplane");
    let project = markplane_core::Project::new(root);
    let task = project
        .create_task(
            "Resource test",
            "feature",
            markplane_core::Priority::Medium,
            markplane_core::Effort::Small,
            None,
            None,
            vec![],
            None,
            None,
        )
        .unwrap();

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 203,
            "method": "resources/read",
            "params": { "uri": format!("markplane://task/{}", task.id) }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["contents"][0]["text"].as_str().unwrap();
    assert!(text.contains(&task.id));
    assert!(text.contains("Resource test"));
}

#[test]
fn test_resource_epic_item() {
    let tmp = setup_project();
    let root = tmp.path().join(".markplane");
    let project = markplane_core::Project::new(root);
    let epic = project
        .create_epic(
            "Epic resource test",
            markplane_core::Priority::High,
            vec![],
            None,
            None,
        )
        .unwrap();

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 204,
            "method": "resources/read",
            "params": { "uri": format!("markplane://epic/{}", epic.id) }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["contents"][0]["text"].as_str().unwrap();
    assert!(text.contains(&epic.id));
}

#[test]
fn test_resource_unknown_uri() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 205,
            "method": "resources/read",
            "params": { "uri": "markplane://nonexistent" }
        }),
    );

    assert!(response["error"].is_object());
    assert!(
        response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Unknown resource URI")
    );
}

#[test]
fn test_resource_missing_uri() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 206,
            "method": "resources/read",
            "params": {}
        }),
    );

    assert!(response["error"].is_object());
}

// ── Plan and Note resource reads ─────────────────────────────────────────

#[test]
fn test_resource_plan_item() {
    let tmp = setup_project();
    let root = tmp.path().join(".markplane");
    let project = markplane_core::Project::new(root);
    let plan = project
        .create_plan("Plan resource test", vec![], vec![], None, None)
        .unwrap();

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 210,
            "method": "resources/read",
            "params": { "uri": format!("markplane://plan/{}", plan.id) }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["contents"][0]["text"].as_str().unwrap();
    assert!(text.contains(&plan.id));
    assert!(text.contains("Plan resource test"));
}

#[test]
fn test_resource_plan_wrong_prefix() {
    let tmp = setup_project();
    let root = tmp.path().join(".markplane");
    let project = markplane_core::Project::new(root);
    let task = project
        .create_task(
            "Not a plan",
            "feature",
            markplane_core::Priority::Medium,
            markplane_core::Effort::Small,
            None,
            None,
            vec![],
            None,
            None,
        )
        .unwrap();

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 211,
            "method": "resources/read",
            "params": { "uri": format!("markplane://plan/{}", task.id) }
        }),
    );

    assert!(response["error"].is_object());
    assert!(
        response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Expected PLAN-")
    );
}

#[test]
fn test_resource_note_item() {
    let tmp = setup_project();
    let root = tmp.path().join(".markplane");
    let project = markplane_core::Project::new(root);
    let note = project
        .create_note("Note resource test", "research", vec![], None, None)
        .unwrap();

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 212,
            "method": "resources/read",
            "params": { "uri": format!("markplane://note/{}", note.id) }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["contents"][0]["text"].as_str().unwrap();
    assert!(text.contains(&note.id));
    assert!(text.contains("Note resource test"));
}

#[test]
fn test_resource_note_wrong_prefix() {
    let tmp = setup_project();
    let root = tmp.path().join(".markplane");
    let project = markplane_core::Project::new(root);
    let task = project
        .create_task(
            "Not a note",
            "feature",
            markplane_core::Priority::Medium,
            markplane_core::Effort::Small,
            None,
            None,
            vec![],
            None,
            None,
        )
        .unwrap();

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 213,
            "method": "resources/read",
            "params": { "uri": format!("markplane://note/{}", task.id) }
        }),
    );

    assert!(response["error"].is_object());
    assert!(
        response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Expected NOTE-")
    );
}

#[test]
fn test_resource_maps() {
    let tmp = setup_project();
    let root = tmp.path().join(".markplane");
    let project = markplane_core::Project::new(root);
    let map = project.create_map("MCP Map", vec![], None, None).unwrap();
    project.generate_context_maps().unwrap();

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 220,
            "method": "resources/read",
            "params": { "uri": "markplane://maps" }
        }),
    );

    assert!(response["error"].is_null());
    let contents = &response["result"]["contents"];
    assert_eq!(contents[0]["uri"], "markplane://maps");
    assert_eq!(contents[0]["mimeType"], "text/markdown");
    let text = contents[0]["text"].as_str().unwrap();
    assert!(text.contains("Active Maps"));
    assert!(text.contains(&map.id));
    assert!(text.contains("MCP Map"));
}

#[test]
fn test_resource_map_item() {
    let tmp = setup_project();
    let root = tmp.path().join(".markplane");
    let project = markplane_core::Project::new(root);
    let map = project
        .create_map("Map resource test", vec![], None, None)
        .unwrap();

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 221,
            "method": "resources/read",
            "params": { "uri": format!("markplane://map/{}", map.id) }
        }),
    );

    assert!(response["error"].is_null());
    let text = response["result"]["contents"][0]["text"].as_str().unwrap();
    assert!(text.contains(&map.id));
    assert!(text.contains("Map resource test"));
    assert!(text.contains("## Destination"));
}

#[test]
fn test_resource_map_wrong_prefix() {
    let tmp = setup_project();
    let root = tmp.path().join(".markplane");
    let project = markplane_core::Project::new(root);
    let task = project
        .create_task(
            "Not a map",
            "feature",
            markplane_core::Priority::Medium,
            markplane_core::Effort::Small,
            None,
            None,
            vec![],
            None,
            None,
        )
        .unwrap();

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 222,
            "method": "resources/read",
            "params": { "uri": format!("markplane://map/{}", task.id) }
        }),
    );

    assert!(response["error"].is_object());
    assert!(
        response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Expected MAP-")
    );
}

// ── Tool: markplane_update (expanded) ────────────────────────────────────

#[test]
fn test_tool_update_effort_and_type() {
    let tmp = setup_project();
    let add_response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "Update effort test" }
            }
        }),
    );
    let task_id = extract_id_from_response(&add_response);

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 300,
            "method": "tools/call",
            "params": {
                "name": "markplane_update",
                "arguments": { "id": task_id, "effort": "large", "type": "bug" }
            }
        }),
    );

    assert!(response["error"].is_null());

    // Verify via show
    let show = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 301,
            "method": "tools/call",
            "params": {
                "name": "markplane_show",
                "arguments": { "id": task_id }
            }
        }),
    );
    let text = show["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("effort: large"));
    assert!(text.contains("type: bug"));
}

#[test]
fn test_tool_update_title() {
    let tmp = setup_project();
    let add_response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "Old title" }
            }
        }),
    );
    let task_id = extract_id_from_response(&add_response);

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 310,
            "method": "tools/call",
            "params": {
                "name": "markplane_update",
                "arguments": { "id": task_id, "title": "New title" }
            }
        }),
    );
    assert!(response["error"].is_null());

    let show = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 311,
            "method": "tools/call",
            "params": {
                "name": "markplane_show",
                "arguments": { "id": task_id }
            }
        }),
    );
    let text = show["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("title: New title"));
}

#[test]
fn test_tool_update_tags() {
    let tmp = setup_project();
    let add_response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "Tag test", "tags": ["old"] }
            }
        }),
    );
    let task_id = extract_id_from_response(&add_response);

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 320,
            "method": "tools/call",
            "params": {
                "name": "markplane_update",
                "arguments": { "id": task_id, "add_tags": ["new", "ui"], "remove_tags": ["old"] }
            }
        }),
    );
    assert!(response["error"].is_null());

    let show = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 321,
            "method": "tools/call",
            "params": {
                "name": "markplane_show",
                "arguments": { "id": task_id }
            }
        }),
    );
    let text = show["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("new"));
    assert!(text.contains("ui"));
    assert!(!text.contains("- old"));
}

#[test]
fn test_tool_update_clear_assignee() {
    let tmp = setup_project();
    let add_response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "Clear assignee test" }
            }
        }),
    );
    let task_id = extract_id_from_response(&add_response);

    // Set assignee
    send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 330,
            "method": "tools/call",
            "params": {
                "name": "markplane_update",
                "arguments": { "id": task_id, "assignee": "daniel" }
            }
        }),
    );

    // Clear assignee (null)
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 331,
            "method": "tools/call",
            "params": {
                "name": "markplane_update",
                "arguments": { "id": task_id, "assignee": null }
            }
        }),
    );
    assert!(response["error"].is_null());

    let show = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 332,
            "method": "tools/call",
            "params": {
                "name": "markplane_show",
                "arguments": { "id": task_id }
            }
        }),
    );
    let text = show["result"]["content"][0]["text"].as_str().unwrap();
    assert!(!text.contains("assignee: daniel"));
}

#[test]
fn test_tool_update_epic_priority_and_tags() {
    let tmp = setup_project();
    let root = tmp.path().join(".markplane");
    let project = markplane_core::Project::new(root);
    let epic = project
        .create_epic(
            "Epic update test",
            markplane_core::Priority::Medium,
            vec![],
            None,
            None,
        )
        .unwrap();

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 350,
            "method": "tools/call",
            "params": {
                "name": "markplane_update",
                "arguments": {
                    "id": epic.id,
                    "priority": "high",
                    "add_tags": ["core", "v2"],
                    "started": "2026-02-20"
                }
            }
        }),
    );
    assert!(response["error"].is_null());

    let show = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 351,
            "method": "tools/call",
            "params": {
                "name": "markplane_show",
                "arguments": { "id": epic.id }
            }
        }),
    );
    let text = show["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("priority: high"));
    assert!(text.contains("core"));
    assert!(text.contains("v2"));
    assert!(text.contains("started: 2026-02-20"));
}

#[test]
fn test_tool_update_note_type_and_tags() {
    let tmp = setup_project();
    let root = tmp.path().join(".markplane");
    let project = markplane_core::Project::new(root);
    let note = project
        .create_note(
            "Note update test",
            "idea",
            vec!["wip".to_string()],
            None,
            None,
        )
        .unwrap();

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 360,
            "method": "tools/call",
            "params": {
                "name": "markplane_update",
                "arguments": {
                    "id": note.id,
                    "note_type": "decision",
                    "add_tags": ["arch"],
                    "remove_tags": ["wip"]
                }
            }
        }),
    );
    assert!(response["error"].is_null());

    let show = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 361,
            "method": "tools/call",
            "params": {
                "name": "markplane_show",
                "arguments": { "id": note.id }
            }
        }),
    );
    let text = show["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("type: decision"));
    assert!(text.contains("arch"));
    assert!(!text.contains("- wip"));
}

#[test]
fn test_tool_update_clear_position_via_null() {
    let tmp = setup_project();
    let add_response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "Position clear test" }
            }
        }),
    );
    let task_id = extract_id_from_response(&add_response);

    // Set position
    send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 370,
            "method": "tools/call",
            "params": {
                "name": "markplane_update",
                "arguments": { "id": task_id, "position": "aaa" }
            }
        }),
    );

    // Clear via null
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 371,
            "method": "tools/call",
            "params": {
                "name": "markplane_update",
                "arguments": { "id": task_id, "position": null }
            }
        }),
    );
    assert!(response["error"].is_null());

    let show = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 372,
            "method": "tools/call",
            "params": {
                "name": "markplane_show",
                "arguments": { "id": task_id }
            }
        }),
    );
    let text = show["result"]["content"][0]["text"].as_str().unwrap();
    assert!(!text.contains("position: aaa"));
}

// ── Move tool ─────────────────────────────────────────────────────────

/// Helper: create a task with a given priority and position via core API.
fn create_positioned_task(
    tmp: &TempDir,
    title: &str,
    priority: markplane_core::Priority,
    position: &str,
) -> String {
    let root = tmp.path().join(".markplane");
    let project = markplane_core::Project::new(root);
    let task = project
        .create_task(
            title,
            "feature",
            priority,
            markplane_core::Effort::Medium,
            None,
            None,
            vec![],
            None,
            None,
        )
        .unwrap();
    project
        .update_task(
            &task.id,
            &markplane_core::TaskUpdate {
                position: markplane_core::Patch::Set(position.to_string()),
                ..Default::default()
            },
        )
        .unwrap();
    task.id
}

/// Helper: read a task's position from its frontmatter via MCP show.
fn read_position(tmp: &TempDir, task_id: &str) -> Option<String> {
    let show = send_request(
        tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 9000,
            "method": "tools/call",
            "params": { "name": "markplane_show", "arguments": { "id": task_id } }
        }),
    );
    let text = show["result"]["content"][0]["text"].as_str().unwrap();
    // Parse the YAML frontmatter for position
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(val) = trimmed.strip_prefix("position: ") {
            if val == "null" {
                return None;
            }
            return Some(val.to_string());
        }
    }
    None
}

#[test]
fn test_tool_move_to_top() {
    let tmp = setup_project();
    let t1 = create_positioned_task(&tmp, "First", markplane_core::Priority::High, "a2");
    let _t2 = create_positioned_task(&tmp, "Second", markplane_core::Priority::High, "a5");
    let t3 = create_positioned_task(&tmp, "Third", markplane_core::Priority::High, "a8");

    // Move t3 to top
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 400,
            "method": "tools/call",
            "params": {
                "name": "markplane_move",
                "arguments": { "id": t3, "to": "top" }
            }
        }),
    );
    assert!(
        response["error"].is_null(),
        "move failed: {:?}",
        response["error"]
    );

    let pos3 = read_position(&tmp, &t3).expect("t3 should have position");
    let pos1 = read_position(&tmp, &t1).expect("t1 should have position");
    assert!(
        pos3 < pos1,
        "t3 ({}) should sort before t1 ({})",
        pos3,
        pos1
    );
}

#[test]
fn test_tool_move_to_bottom() {
    let tmp = setup_project();
    let t1 = create_positioned_task(&tmp, "First", markplane_core::Priority::High, "a2");
    let _t2 = create_positioned_task(&tmp, "Second", markplane_core::Priority::High, "a5");
    let t3 = create_positioned_task(&tmp, "Third", markplane_core::Priority::High, "a8");

    // Move t1 to bottom
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 401,
            "method": "tools/call",
            "params": {
                "name": "markplane_move",
                "arguments": { "id": t1, "to": "bottom" }
            }
        }),
    );
    assert!(
        response["error"].is_null(),
        "move failed: {:?}",
        response["error"]
    );

    let pos1 = read_position(&tmp, &t1).expect("t1 should have position");
    let pos3 = read_position(&tmp, &t3).expect("t3 should have position");
    assert!(pos1 > pos3, "t1 ({}) should sort after t3 ({})", pos1, pos3);
}

#[test]
fn test_tool_move_before() {
    let tmp = setup_project();
    let t1 = create_positioned_task(&tmp, "First", markplane_core::Priority::High, "a2");
    let t2 = create_positioned_task(&tmp, "Second", markplane_core::Priority::High, "a5");
    let t3 = create_positioned_task(&tmp, "Third", markplane_core::Priority::High, "a8");

    // Move t3 before t2
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 402,
            "method": "tools/call",
            "params": {
                "name": "markplane_move",
                "arguments": { "id": t3, "before": t2 }
            }
        }),
    );
    assert!(
        response["error"].is_null(),
        "move failed: {:?}",
        response["error"]
    );

    let pos1 = read_position(&tmp, &t1).expect("t1 should have position");
    let pos3 = read_position(&tmp, &t3).expect("t3 should have position");
    let pos2 = read_position(&tmp, &t2).expect("t2 should have position");
    assert!(pos1 < pos3, "t1 ({}) < t3 ({})", pos1, pos3);
    assert!(pos3 < pos2, "t3 ({}) < t2 ({})", pos3, pos2);
}

#[test]
fn test_tool_move_after() {
    let tmp = setup_project();
    let t1 = create_positioned_task(&tmp, "First", markplane_core::Priority::High, "a2");
    let t2 = create_positioned_task(&tmp, "Second", markplane_core::Priority::High, "a5");
    let t3 = create_positioned_task(&tmp, "Third", markplane_core::Priority::High, "a8");

    // Move t1 after t2
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 403,
            "method": "tools/call",
            "params": {
                "name": "markplane_move",
                "arguments": { "id": t1, "after": t2 }
            }
        }),
    );
    assert!(
        response["error"].is_null(),
        "move failed: {:?}",
        response["error"]
    );

    let pos2 = read_position(&tmp, &t2).expect("t2 should have position");
    let pos1 = read_position(&tmp, &t1).expect("t1 should have position");
    let pos3 = read_position(&tmp, &t3).expect("t3 should have position");
    assert!(pos2 < pos1, "t2 ({}) < t1 ({})", pos2, pos1);
    assert!(pos1 < pos3, "t1 ({}) < t3 ({})", pos1, pos3);
}

#[test]
fn test_tool_move_different_priority_error() {
    let tmp = setup_project();
    let t1 = create_positioned_task(&tmp, "High task", markplane_core::Priority::High, "a0");
    let t2 = create_positioned_task(&tmp, "Low task", markplane_core::Priority::Low, "a0");

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 404,
            "method": "tools/call",
            "params": {
                "name": "markplane_move",
                "arguments": { "id": t1, "before": t2 }
            }
        }),
    );
    assert_tool_error(&response, "should error for different priorities");
}

#[test]
fn test_tool_move_self_reference_error() {
    let tmp = setup_project();
    let t1 = create_positioned_task(&tmp, "Task", markplane_core::Priority::High, "a0");

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 405,
            "method": "tools/call",
            "params": {
                "name": "markplane_move",
                "arguments": { "id": t1, "before": t1 }
            }
        }),
    );
    assert_tool_error(&response, "should error for self-reference");
}

#[test]
fn test_tool_move_invalid_directive() {
    let tmp = setup_project();
    let add_response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": "markplane_add", "arguments": { "title": "Test" } }
        }),
    );
    let task_id = extract_id_from_response(&add_response);

    // No positioning directive
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 406,
            "method": "tools/call",
            "params": {
                "name": "markplane_move",
                "arguments": { "id": task_id }
            }
        }),
    );
    assert_tool_error(&response, "should error with no directive");

    // Invalid 'to' value
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 407,
            "method": "tools/call",
            "params": {
                "name": "markplane_move",
                "arguments": { "id": task_id, "to": "middle" }
            }
        }),
    );
    assert_tool_error(&response, "should error with invalid 'to'");
}

#[test]
fn test_tool_move_nonexistent_target() {
    let tmp = setup_project();
    let t1 = create_positioned_task(&tmp, "Task", markplane_core::Priority::High, "a0");

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 408,
            "method": "tools/call",
            "params": {
                "name": "markplane_move",
                "arguments": { "id": t1, "after": "TASK-nope0" }
            }
        }),
    );
    assert_tool_error(&response, "should error for nonexistent target");
}

#[test]
fn test_tool_move_non_task_error() {
    let tmp = setup_project();
    let root = tmp.path().join(".markplane");
    let project = markplane_core::Project::new(root);
    let epic = project
        .create_epic("Epic", markplane_core::Priority::High, vec![], None, None)
        .unwrap();

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 409,
            "method": "tools/call",
            "params": {
                "name": "markplane_move",
                "arguments": { "id": epic.id, "to": "top" }
            }
        }),
    );
    assert_tool_error(&response, "should error for non-task");
}

#[test]
fn test_tool_update_rejects_invalid_field_for_type() {
    let tmp = setup_project();
    let root = tmp.path().join(".markplane");
    let project = markplane_core::Project::new(root);
    let plan = project
        .create_plan("Test plan", vec![], vec![], None, None)
        .unwrap();

    // priority is not valid for plans
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 340,
            "method": "tools/call",
            "params": {
                "name": "markplane_update",
                "arguments": { "id": plan.id, "priority": "high" }
            }
        }),
    );
    assert_tool_error(&response, "invalid field for plan");
}

// ── Template param ──────────────────────────────────────────────────

#[test]
fn test_tool_add_with_template() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 400,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": {
                    "title": "Bug report",
                    "type": "bug",
                    "template": "bug"
                }
            }
        }),
    );
    assert!(
        response["result"].is_object(),
        "Expected result, got: {:?}",
        response
    );

    let id = extract_id_from_response(&response);
    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", id)),
    )
    .unwrap();
    assert!(content.contains("## Steps to Reproduce"));
}

#[test]
fn test_tool_plan_with_template() {
    let tmp = setup_project();
    let root = tmp.path().join(".markplane");
    let project = markplane_core::Project::new(root);

    let task = project
        .create_task(
            "Refactor test",
            "feature",
            markplane_core::Priority::Medium,
            markplane_core::Effort::Medium,
            None,
            None,
            vec![],
            None,
            None,
        )
        .unwrap();

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 401,
            "method": "tools/call",
            "params": {
                "name": "markplane_plan",
                "arguments": {
                    "task_id": task.id,
                    "template": "refactor"
                }
            }
        }),
    );
    assert!(
        response["result"].is_object(),
        "Expected result, got: {:?}",
        response
    );

    let plan_id = extract_id_from_response(&response);
    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/plans/items/{}.md", plan_id)),
    )
    .unwrap();
    assert!(content.contains("## Motivation"));
}

#[test]
fn test_resource_templates() {
    let tmp = setup_project();
    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 402,
            "method": "resources/read",
            "params": {
                "uri": "markplane://templates"
            }
        }),
    );
    assert!(
        response["result"].is_object(),
        "Expected result, got: {:?}",
        response
    );

    let text = response["result"]["contents"][0]["text"].as_str().unwrap();
    assert!(text.contains("task:"));
    assert!(text.contains("epic:"));
    assert!(text.contains("plan:"));
    assert!(text.contains("note:"));
}

// ── Tool: markplane_query frontier ───────────────────────────────────────

#[test]
fn test_tool_query_frontier_and_richer_fields() {
    let tmp = setup_project();

    let epic_resp = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "kind": "epic", "title": "Frontier epic" }
            }
        }),
    );
    let epic_id = extract_id_from_response(&epic_resp);

    let blocker_resp = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": {
                    "title": "Blocker",
                    "epic": epic_id,
                    "tags": ["alpha"]
                }
            }
        }),
    );
    let blocker_id = extract_id_from_response(&blocker_resp);

    let dependent_resp = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": {
                    "title": "Dependent",
                    "epic": epic_id,
                    "tags": ["beta"]
                }
            }
        }),
    );
    let dependent_id = extract_id_from_response(&dependent_resp);

    let claimed_resp = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "title": "Claimed", "epic": epic_id }
            }
        }),
    );
    let claimed_id = extract_id_from_response(&claimed_resp);

    send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "markplane_link",
                "arguments": {
                    "from": blocker_id,
                    "to": dependent_id,
                    "relation": "blocks"
                }
            }
        }),
    );

    send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "markplane_update",
                "arguments": { "id": claimed_id, "assignee": "alice" }
            }
        }),
    );

    send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": {
                "name": "markplane_update",
                "arguments": { "id": blocker_id, "status": "cancelled" }
            }
        }),
    );

    let response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "tools/call",
            "params": {
                "name": "markplane_query",
                "arguments": { "epic": epic_id, "frontier": true }
            }
        }),
    );
    assert!(response["error"].is_null());
    let text = response["result"]["content"][0]["text"].as_str().unwrap();
    let items: Vec<Value> = serde_json::from_str(text).unwrap();
    let ids: Vec<&str> = items.iter().map(|i| i["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&dependent_id.as_str()));
    assert!(!ids.contains(&blocker_id.as_str()));
    assert!(!ids.contains(&claimed_id.as_str()));

    let dependent = items
        .iter()
        .find(|i| i["id"] == dependent_id)
        .expect("dependent on frontier");
    assert!(dependent["assignee"].is_null());
    assert_eq!(dependent["depends_on"], json!([blocker_id]));
    assert_eq!(dependent["tags"], json!(["beta"]));

    // Richer fields also present on non-frontier query
    let all_resp = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "tools/call",
            "params": {
                "name": "markplane_query",
                "arguments": { "epic": epic_id }
            }
        }),
    );
    let all_text = all_resp["result"]["content"][0]["text"].as_str().unwrap();
    let all_items: Vec<Value> = serde_json::from_str(all_text).unwrap();
    let claimed = all_items
        .iter()
        .find(|i| i["id"] == claimed_id)
        .expect("claimed in epic query");
    assert_eq!(claimed["assignee"], "alice");
    assert!(claimed.get("depends_on").is_some());
    assert!(claimed.get("tags").is_some());
}

// ── Tool: markplane_add / query with map (Decision tickets) ──────────────

#[test]
fn test_tool_add_and_query_map_decision_tickets() {
    let tmp = setup_project();

    let map_resp = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "kind": "map", "title": "MCP decision map" }
            }
        }),
    );
    assert!(map_resp["error"].is_null());
    let map_id = extract_id_from_response(&map_resp);

    let ok = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": {
                    "title": "Grill via MCP",
                    "map": map_id,
                    "type": "grilling"
                }
            }
        }),
    );
    assert!(ok["error"].is_null(), "{ok}");
    assert!(!ok["result"]["isError"].as_bool().unwrap_or(false), "{ok}");
    let task_id = extract_id_from_response(&ok);

    // feature type rejected for decision tickets
    let bad = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": {
                    "title": "Bad type",
                    "map": map_id,
                    "type": "feature"
                }
            }
        }),
    );
    assert_tool_error(&bad, "feature type should be rejected for map");

    // map + epic rejected
    let epic_resp = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "kind": "epic", "title": "MCP epic" }
            }
        }),
    );
    let epic_id = extract_id_from_response(&epic_resp);
    let mutex = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": {
                    "title": "Both parents",
                    "map": map_id,
                    "epic": epic_id,
                    "type": "task"
                }
            }
        }),
    );
    assert_tool_error(&mutex, "map+epic should be rejected");

    let blocked = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": {
                    "title": "Blocked decision",
                    "map": map_id,
                    "type": "research"
                }
            }
        }),
    );
    let blocked_id = extract_id_from_response(&blocked);

    send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": {
                "name": "markplane_link",
                "arguments": {
                    "from": task_id,
                    "to": blocked_id,
                    "relation": "blocks"
                }
            }
        }),
    );

    let query = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "tools/call",
            "params": {
                "name": "markplane_query",
                "arguments": { "map": map_id }
            }
        }),
    );
    assert!(query["error"].is_null());
    let text = query["result"]["content"][0]["text"].as_str().unwrap();
    let items: Vec<Value> = serde_json::from_str(text).unwrap();
    let ids: Vec<&str> = items.iter().map(|i| i["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&task_id.as_str()));
    assert!(ids.contains(&blocked_id.as_str()));

    let frontier = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "tools/call",
            "params": {
                "name": "markplane_query",
                "arguments": { "map": map_id, "frontier": true }
            }
        }),
    );
    assert!(frontier["error"].is_null());
    let ftext = frontier["result"]["content"][0]["text"].as_str().unwrap();
    let fitems: Vec<Value> = serde_json::from_str(ftext).unwrap();
    let fids: Vec<&str> = fitems.iter().map(|i| i["id"].as_str().unwrap()).collect();
    assert!(fids.contains(&task_id.as_str()));
    assert!(!fids.contains(&blocked_id.as_str()));
}

// ── Tool: markplane_resolve ──────────────────────────────────────────────

#[test]
fn test_tool_resolve_matches_cli() {
    let tmp = setup_project();
    let answer = "We chose option B.\n";
    let gist = "option B";

    // CLI path
    let map_cli = assert_cmd::cargo::cargo_bin_cmd!("markplane")
        .current_dir(tmp.path())
        .args(["map", "Resolve parity map CLI"])
        .output()
        .unwrap();
    assert!(map_cli.status.success());
    let map_cli_id = String::from_utf8_lossy(&map_cli.stdout)
        .split_whitespace()
        .find(|w| w.starts_with("MAP-"))
        .expect("CLI map ID")
        .to_string();
    // Keep another open ticket so hint is not required for parity of file effects.
    let _keep = assert_cmd::cargo::cargo_bin_cmd!("markplane")
        .current_dir(tmp.path())
        .args([
            "add",
            "Keep open CLI",
            "--map",
            &map_cli_id,
            "--type",
            "task",
        ])
        .output()
        .unwrap();
    let task_cli = assert_cmd::cargo::cargo_bin_cmd!("markplane")
        .current_dir(tmp.path())
        .args([
            "add",
            "Parity decision CLI",
            "--map",
            &map_cli_id,
            "--type",
            "grilling",
        ])
        .output()
        .unwrap();
    let cli_task_id = String::from_utf8_lossy(&task_cli.stdout)
        .split_whitespace()
        .find(|w| w.starts_with("TASK-"))
        .expect("CLI task ID")
        .to_string();

    let answer_path = tmp.path().join("answer.md");
    std::fs::write(&answer_path, answer).unwrap();
    let cli_resolve = assert_cmd::cargo::cargo_bin_cmd!("markplane")
        .current_dir(tmp.path())
        .args([
            "resolve",
            &cli_task_id,
            "--answer-file",
            answer_path.to_str().unwrap(),
            "--gist",
            gist,
        ])
        .output()
        .unwrap();
    assert!(
        cli_resolve.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&cli_resolve.stderr)
    );

    // MCP path
    let map_mcp = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 40,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": { "kind": "map", "title": "Resolve parity map MCP" }
            }
        }),
    );
    assert!(map_mcp["error"].is_null());
    let map_mcp_id = extract_id_from_response(&map_mcp);

    let _keep_mcp = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 41,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": {
                    "title": "Keep open MCP",
                    "map": map_mcp_id,
                    "type": "task"
                }
            }
        }),
    );
    let task_mcp = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "tools/call",
            "params": {
                "name": "markplane_add",
                "arguments": {
                    "title": "Parity decision MCP",
                    "map": map_mcp_id,
                    "type": "grilling"
                }
            }
        }),
    );
    assert!(task_mcp["error"].is_null());
    let mcp_task_id = extract_id_from_response(&task_mcp);

    let mcp_response = send_request(
        &tmp,
        &json!({
            "jsonrpc": "2.0",
            "id": 43,
            "method": "tools/call",
            "params": {
                "name": "markplane_resolve",
                "arguments": {
                    "id": mcp_task_id,
                    "answer": answer,
                    "gist": gist
                }
            }
        }),
    );
    assert!(
        mcp_response["error"].is_null(),
        "MCP error: {}",
        mcp_response
    );
    assert_ne!(
        mcp_response["result"]["isError"], true,
        "tool error: {}",
        mcp_response["result"]["content"][0]["text"]
    );

    let cli_task = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", cli_task_id)),
    )
    .unwrap();
    let mcp_task = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", mcp_task_id)),
    )
    .unwrap();
    assert!(cli_task.contains("status: done"));
    assert!(mcp_task.contains("status: done"));
    assert!(cli_task.contains("## Answer"));
    assert!(mcp_task.contains("## Answer"));
    assert!(cli_task.contains("We chose option B."));
    assert!(mcp_task.contains("We chose option B."));

    let cli_map = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/maps/items/{}.md", map_cli_id)),
    )
    .unwrap();
    let mcp_map = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/maps/items/{}.md", map_mcp_id)),
    )
    .unwrap();
    let cli_line = format!("- Parity decision CLI [[{}]]: {}", cli_task_id, gist);
    let mcp_line = format!("- Parity decision MCP [[{}]]: {}", mcp_task_id, gist);
    assert!(
        cli_map.contains(&cli_line),
        "CLI map missing decision line:\n{}",
        cli_map
    );
    assert!(
        mcp_map.contains(&mcp_line),
        "MCP map missing decision line:\n{}",
        mcp_map
    );
    assert!(cli_map.contains("status: active"));
    assert!(mcp_map.contains("status: active"));
}
