#[cfg(unix)]
mod common;

use assert_cmd::Command;
#[cfg(unix)]
use common::free_port;
use predicates::prelude::*;
use tempfile::TempDir;

fn cmd() -> Command {
    assert_cmd::cargo::cargo_bin_cmd!("markplane")
}

/// Run markplane inside a temp dir, initializing it first (empty, no starter content).
fn setup_project() -> TempDir {
    let tmp = TempDir::new().unwrap();
    cmd()
        .current_dir(tmp.path())
        .args(["init", "--name", "Test Project", "--empty"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized Markplane project"));
    tmp
}

/// Extract the generated ID from CLI output like "Created TASK-k7x9m — title"
fn extract_id(output: &[u8]) -> String {
    let stdout = String::from_utf8_lossy(output);
    stdout
        .split_whitespace()
        .find(|w| {
            w.starts_with("TASK-")
                || w.starts_with("EPIC-")
                || w.starts_with("PLAN-")
                || w.starts_with("NOTE-")
                || w.starts_with("MAP-")
        })
        .unwrap_or_else(|| panic!("No ID found in output: {}", stdout))
        .to_string()
}

// ── Init ─────────────────────────────────────────────────────────────────

#[test]
fn test_init_creates_structure() {
    let tmp = TempDir::new().unwrap();
    cmd()
        .current_dir(tmp.path())
        .args([
            "init",
            "--name",
            "My Project",
            "--description",
            "A test",
            "--empty",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Initialized Markplane project: My Project",
        ))
        .stdout(predicate::str::contains(".markplane/"));

    assert!(tmp.path().join(".markplane/config.yaml").is_file());
    assert!(tmp.path().join(".markplane/INDEX.md").is_file());
    assert!(tmp.path().join(".markplane/backlog").is_dir());
    assert!(tmp.path().join(".markplane/roadmap").is_dir());
    assert!(tmp.path().join(".markplane/plans").is_dir());
    assert!(tmp.path().join(".markplane/notes").is_dir());
    assert!(tmp.path().join(".markplane/maps").is_dir());
    assert!(tmp.path().join(".markplane/.context").is_dir());
}

#[test]
fn test_init_defaults_to_dir_name() {
    let tmp = TempDir::new().unwrap();
    cmd()
        .current_dir(tmp.path())
        .args(["init", "--empty"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized Markplane project"));
}

#[test]
fn test_init_already_initialized() {
    let tmp = setup_project();
    cmd()
        .current_dir(tmp.path())
        .args(["init", "--empty"])
        .assert()
        .failure();
}

#[test]
fn test_init_with_starter_content() {
    let tmp = TempDir::new().unwrap();
    cmd()
        .current_dir(tmp.path())
        .args(["init", "--name", "Seeded Project"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Seeded with starter content"))
        .stdout(predicate::str::contains("Next steps:"));

    // Tasks should exist
    cmd()
        .current_dir(tmp.path())
        .arg("ls")
        .assert()
        .success()
        .stdout(predicate::str::contains("Review and customize"))
        .stdout(predicate::str::contains("Import existing work"));

    // Epic should exist
    cmd()
        .current_dir(tmp.path())
        .args(["ls", "epics"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Project Setup"));

    // Plan should exist
    cmd()
        .current_dir(tmp.path())
        .args(["ls", "plans"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Import existing work"));

    // Note should exist
    cmd()
        .current_dir(tmp.path())
        .args(["ls", "notes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Project decisions"));

    // No broken references
    cmd()
        .current_dir(tmp.path())
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("No broken references"));
}

#[test]
fn test_init_empty() {
    let tmp = TempDir::new().unwrap();
    cmd()
        .current_dir(tmp.path())
        .args(["init", "--name", "Empty Project", "--empty"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized Markplane project"))
        .stdout(predicate::str::contains("Get started:"));

    cmd()
        .current_dir(tmp.path())
        .arg("ls")
        .assert()
        .success()
        .stdout(predicate::str::contains("No tasks found"));
}

// ── Add ──────────────────────────────────────────────────────────────────

#[test]
fn test_add_basic() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["add", "Fix login bug"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let task_id = extract_id(&output.stdout);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Created"));
    assert!(stdout.contains(&task_id));
    assert!(stdout.contains("Fix login bug"));

    assert!(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id))
            .is_file()
    );
}

#[test]
fn test_add_with_flags() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "add",
            "Add dark mode",
            "--type",
            "feature",
            "--priority",
            "high",
            "--effort",
            "large",
            "--tags",
            "ui,frontend",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let task_id = extract_id(&output.stdout);

    // Verify the file contains the right metadata
    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    assert!(content.contains("priority: high"));
    assert!(content.contains("type: feature"));
    assert!(content.contains("effort: large"));
    assert!(content.contains("ui"));
    assert!(content.contains("frontend"));
}

#[test]
fn test_add_with_epic() {
    let tmp = setup_project();
    // Create an epic first
    let epic_output = cmd()
        .current_dir(tmp.path())
        .args(["epic", "Phase 1"])
        .output()
        .unwrap();
    assert!(
        epic_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&epic_output.stderr)
    );
    let epic_id = extract_id(&epic_output.stdout);

    let output = cmd()
        .current_dir(tmp.path())
        .args(["add", "Task in epic", "--epic", &epic_id])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let task_id = extract_id(&output.stdout);

    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    assert!(content.contains(&format!("epic: {}", epic_id)));
}

#[test]
fn test_add_random_ids() {
    let tmp = setup_project();
    let out1 = cmd()
        .current_dir(tmp.path())
        .args(["add", "First"])
        .output()
        .unwrap();
    assert!(
        out1.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out1.stderr)
    );
    let id1 = extract_id(&out1.stdout);
    assert!(id1.starts_with("TASK-"));

    let out2 = cmd()
        .current_dir(tmp.path())
        .args(["add", "Second"])
        .output()
        .unwrap();
    assert!(
        out2.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out2.stderr)
    );
    let id2 = extract_id(&out2.stdout);
    assert!(id2.starts_with("TASK-"));

    let out3 = cmd()
        .current_dir(tmp.path())
        .args(["add", "Third"])
        .output()
        .unwrap();
    assert!(
        out3.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out3.stderr)
    );
    let id3 = extract_id(&out3.stdout);
    assert!(id3.starts_with("TASK-"));

    // All IDs should be unique
    assert_ne!(id1, id2);
    assert_ne!(id2, id3);
    assert_ne!(id1, id3);
}

// ── Show ─────────────────────────────────────────────────────────────────

#[test]
fn test_show_task() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["add", "Show me", "--priority", "high"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let task_id = extract_id(&output.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["show", &task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains(&task_id))
        .stdout(predicate::str::contains("Show me"))
        .stdout(predicate::str::contains("high"));
}

#[test]
fn test_show_not_found() {
    let tmp = setup_project();
    cmd()
        .current_dir(tmp.path())
        .args(["show", "TASK-999"])
        .assert()
        .failure();
}

#[test]
fn test_show_epic() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["epic", "Phase 1", "--priority", "high"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let epic_id = extract_id(&output.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["show", &epic_id])
        .assert()
        .success()
        .stdout(predicate::str::contains(&epic_id))
        .stdout(predicate::str::contains("Phase 1"));
}

// ── Ls ───────────────────────────────────────────────────────────────────

#[test]
fn test_ls_empty() {
    let tmp = setup_project();
    cmd()
        .current_dir(tmp.path())
        .arg("ls")
        .assert()
        .success()
        .stdout(predicate::str::contains("No tasks found"));
}

#[test]
fn test_ls_with_items() {
    let tmp = setup_project();
    let out1 = cmd()
        .current_dir(tmp.path())
        .args(["add", "Task A"])
        .output()
        .unwrap();
    assert!(
        out1.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out1.stderr)
    );
    let id1 = extract_id(&out1.stdout);

    let out2 = cmd()
        .current_dir(tmp.path())
        .args(["add", "Task B"])
        .output()
        .unwrap();
    assert!(
        out2.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out2.stderr)
    );
    let id2 = extract_id(&out2.stdout);

    cmd()
        .current_dir(tmp.path())
        .arg("ls")
        .assert()
        .success()
        .stdout(predicate::str::contains(&id1))
        .stdout(predicate::str::contains(&id2))
        .stdout(predicate::str::contains("Task A"))
        .stdout(predicate::str::contains("Task B"));
}

#[test]
fn test_ls_filter_status() {
    let tmp = setup_project();
    let out1 = cmd()
        .current_dir(tmp.path())
        .args(["add", "Draft item"])
        .output()
        .unwrap();
    assert!(
        out1.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out1.stderr)
    );

    let out2 = cmd()
        .current_dir(tmp.path())
        .args(["add", "Progress item"])
        .output()
        .unwrap();
    assert!(
        out2.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out2.stderr)
    );
    let id2 = extract_id(&out2.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["status", &id2, "in-progress"])
        .assert()
        .success();

    cmd()
        .current_dir(tmp.path())
        .args(["ls", "--status", "in-progress"])
        .assert()
        .success()
        .stdout(predicate::str::contains(&id2))
        .stdout(predicate::str::contains("Draft item").not());
}

#[test]
fn test_ls_epics() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["epic", "Phase 1"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let epic_id = extract_id(&output.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["ls", "epics"])
        .assert()
        .success()
        .stdout(predicate::str::contains(&epic_id))
        .stdout(predicate::str::contains("Phase 1"));
}

#[test]
fn test_ls_plans() {
    let tmp = setup_project();
    let task_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "Some task"])
        .output()
        .unwrap();
    assert!(
        task_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&task_out.stderr)
    );
    let task_id = extract_id(&task_out.stdout);

    let plan_out = cmd()
        .current_dir(tmp.path())
        .args(["plan", &task_id])
        .output()
        .unwrap();
    assert!(
        plan_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&plan_out.stderr)
    );
    let plan_id = extract_id(&plan_out.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["ls", "plans"])
        .assert()
        .success()
        .stdout(predicate::str::contains(&plan_id));
}

#[test]
fn test_ls_notes() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["note", "Research topic", "--type", "research"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let note_id = extract_id(&output.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["ls", "notes"])
        .assert()
        .success()
        .stdout(predicate::str::contains(&note_id))
        .stdout(predicate::str::contains("Research topic"));
}

// ── Status ───────────────────────────────────────────────────────────────

#[test]
fn test_status_update() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["add", "Status test"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let task_id = extract_id(&output.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["status", &task_id, "in-progress"])
        .assert()
        .success()
        .stdout(predicate::str::contains(&task_id))
        .stdout(predicate::str::contains("in-progress"));

    // Verify the change
    cmd()
        .current_dir(tmp.path())
        .args(["show", &task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("in-progress"));
}

#[test]
fn test_status_invalid() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["add", "Status test"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let task_id = extract_id(&output.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["status", &task_id, "invalid-status"])
        .assert()
        .failure();
}

// ── Sync ─────────────────────────────────────────────────────────────────

#[test]
fn test_sync() {
    let tmp = setup_project();
    cmd()
        .current_dir(tmp.path())
        .args(["add", "Sync test"])
        .assert()
        .success();

    cmd().current_dir(tmp.path()).arg("sync").assert().success();

    // INDEX.md files should be regenerated
    let root_index = std::fs::read_to_string(tmp.path().join(".markplane/INDEX.md")).unwrap();
    assert!(root_index.contains("Generated by markplane sync"));

    // Context files should exist
    assert!(tmp.path().join(".markplane/.context/summary.md").is_file());
}

#[test]
fn test_sync_with_missing_entity_dirs() {
    // On a fresh clone, directories holding only gitignored files don't exist.
    let tmp = setup_project();
    let root = tmp.path().join(".markplane");
    for dir in ["backlog", "roadmap", "plans", "notes", ".context"] {
        std::fs::remove_dir_all(root.join(dir)).unwrap();
    }

    cmd().current_dir(tmp.path()).arg("sync").assert().success();

    for dir in ["backlog", "roadmap", "plans", "notes"] {
        assert!(root.join(dir).join("INDEX.md").is_file(), "{dir}/INDEX.md");
        assert!(root.join(dir).join("items").is_dir(), "{dir}/items");
        assert!(root.join(dir).join("archive").is_dir(), "{dir}/archive");
    }
    assert!(root.join(".context/summary.md").is_file());
}

// ── Start / Done ─────────────────────────────────────────────────────────

#[test]
fn test_start_and_done() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["add", "Start/done test"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let task_id = extract_id(&output.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["start", &task_id, "--user", "alice"])
        .assert()
        .success()
        .stdout(predicate::str::contains("in-progress"))
        .stdout(predicate::str::contains("alice"));

    // Verify status and assignee
    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    assert!(content.contains("status: in-progress"));
    assert!(content.contains("assignee: alice"));

    cmd()
        .current_dir(tmp.path())
        .args(["done", &task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("done"));
}

// ── Epic / Note / Plan ───────────────────────────────────────────────────

#[test]
fn test_epic_creation() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["epic", "Phase 1", "--priority", "high"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let epic_id = extract_id(&output.stdout);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Created"));
    assert!(stdout.contains(&epic_id));
    assert!(stdout.contains("Phase 1"));

    assert!(
        tmp.path()
            .join(format!(".markplane/roadmap/items/{}.md", epic_id))
            .is_file()
    );
}

#[test]
fn test_note_creation() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "note",
            "Research caching",
            "--type",
            "research",
            "--tags",
            "cache,perf",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let note_id = extract_id(&output.stdout);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Created"));
    assert!(stdout.contains(&note_id));
    assert!(stdout.contains("Research caching"));

    assert!(
        tmp.path()
            .join(format!(".markplane/notes/items/{}.md", note_id))
            .is_file()
    );
}

#[test]
fn test_plan_creation() {
    let tmp = setup_project();
    let task_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "Dark mode"])
        .output()
        .unwrap();
    assert!(
        task_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&task_out.stderr)
    );
    let task_id = extract_id(&task_out.stdout);

    let plan_out = cmd()
        .current_dir(tmp.path())
        .args(["plan", &task_id, "--title", "Dark mode plan"])
        .output()
        .unwrap();
    assert!(
        plan_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&plan_out.stderr)
    );
    let plan_id = extract_id(&plan_out.stdout);
    let stdout = String::from_utf8_lossy(&plan_out.stdout);
    assert!(stdout.contains("Created"));
    assert!(stdout.contains(&plan_id));
    assert!(stdout.contains("Dark mode plan"));
    assert!(stdout.contains(&format!("Linked to {}", task_id)));

    // Verify task has plan linked
    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    assert!(content.contains(&plan_id));
}

// ── Promote ──────────────────────────────────────────────────────────────

#[test]
fn test_promote_note_to_task() {
    let tmp = setup_project();
    let note_out = cmd()
        .current_dir(tmp.path())
        .args(["note", "Good idea", "--type", "idea", "--tags", "cool"])
        .output()
        .unwrap();
    assert!(
        note_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&note_out.stderr)
    );
    let note_id = extract_id(&note_out.stdout);

    let promote_out = cmd()
        .current_dir(tmp.path())
        .args(["promote", &note_id, "--priority", "high"])
        .output()
        .unwrap();
    assert!(
        promote_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&promote_out.stderr)
    );
    let stdout = String::from_utf8_lossy(&promote_out.stdout);
    assert!(stdout.contains(&format!("Promoted {}", note_id)));
    // Extract the TASK ID from "Promoted NOTE-xxx → TASK-yyy — title"
    let task_id = stdout
        .split_whitespace()
        .find(|w| w.starts_with("TASK-"))
        .expect("No TASK ID found in promote output")
        .to_string();

    assert!(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id))
            .is_file()
    );
}

#[test]
fn test_promote_non_note_fails() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["add", "Not a note"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let task_id = extract_id(&output.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["promote", &task_id])
        .assert()
        .failure();
}

// ── Update ───────────────────────────────────────────────────────────────

#[test]
fn test_update_assignee() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["add", "Update assignee test"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let task_id = extract_id(&output.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["update", &task_id, "--assignee", "@daniel"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("Updated {}", task_id)));

    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    assert!(content.contains("assignee: daniel"));
}

#[test]
fn test_update_clear_assignee() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["add", "Clear assignee test"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let task_id = extract_id(&output.stdout);

    // Set assignee first
    cmd()
        .current_dir(tmp.path())
        .args(["update", &task_id, "--assignee", "daniel"])
        .assert()
        .success();

    // Clear it
    cmd()
        .current_dir(tmp.path())
        .args(["update", &task_id, "--clear-assignee"])
        .assert()
        .success();

    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    assert!(!content.contains("assignee: daniel"));
}

#[test]
fn test_update_tags() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["add", "Tag test"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let task_id = extract_id(&output.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["update", &task_id, "--add-tag", "ui,frontend"])
        .assert()
        .success();

    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    assert!(content.contains("- ui\n"));
    assert!(content.contains("- frontend\n"));

    // Remove one tag
    cmd()
        .current_dir(tmp.path())
        .args(["update", &task_id, "--remove-tag", "ui"])
        .assert()
        .success();

    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    assert!(!content.contains("- ui\n"));
    assert!(content.contains("- frontend\n"));
}

#[test]
fn test_update_effort_priority() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["add", "Effort test"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let task_id = extract_id(&output.stdout);

    cmd()
        .current_dir(tmp.path())
        .args([
            "update",
            &task_id,
            "--effort",
            "large",
            "--priority",
            "high",
        ])
        .assert()
        .success();

    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    assert!(content.contains("effort: large"));
    assert!(content.contains("priority: high"));
}

#[test]
fn test_update_title_and_type() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["add", "Title test"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let task_id = extract_id(&output.stdout);

    cmd()
        .current_dir(tmp.path())
        .args([
            "update",
            &task_id,
            "--title",
            "Renamed task",
            "--type",
            "bug",
        ])
        .assert()
        .success();

    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    assert!(content.contains("title: Renamed task"));
    assert!(content.contains("type: bug"));
}

#[test]
fn test_update_position() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["add", "Position test"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let task_id = extract_id(&output.stdout);

    // Set position
    cmd()
        .current_dir(tmp.path())
        .args(["update", &task_id, "--position", "aaa"])
        .assert()
        .success();

    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    assert!(content.contains("position: aaa"));

    // Clear position
    cmd()
        .current_dir(tmp.path())
        .args(["update", &task_id, "--clear-position"])
        .assert()
        .success();

    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    assert!(!content.contains("position: aaa"));
}

#[test]
fn test_update_epic_dates() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["epic", "Dates test"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let epic_id = extract_id(&output.stdout);

    // Set started and target dates
    cmd()
        .current_dir(tmp.path())
        .args([
            "update",
            &epic_id,
            "--started",
            "2026-02-20",
            "--target",
            "2026-06-01",
        ])
        .assert()
        .success();

    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/roadmap/items/{}.md", epic_id)),
    )
    .unwrap();
    assert!(content.contains("started: 2026-02-20"));
    assert!(content.contains("target: 2026-06-01"));

    // Clear started
    cmd()
        .current_dir(tmp.path())
        .args(["update", &epic_id, "--clear-started"])
        .assert()
        .success();

    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/roadmap/items/{}.md", epic_id)),
    )
    .unwrap();
    assert!(!content.contains("started: 2026-02-20"));
    assert!(content.contains("target: 2026-06-01"));
}

#[test]
fn test_update_note_type() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["note", "Note type test", "--type", "idea"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let note_id = extract_id(&output.stdout);

    // Change note type
    cmd()
        .current_dir(tmp.path())
        .args(["update", &note_id, "--note-type", "decision"])
        .assert()
        .success();

    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/notes/items/{}.md", note_id)),
    )
    .unwrap();
    assert!(content.contains("type: decision"));
}

#[test]
fn test_update_rejects_invalid_field() {
    let tmp = setup_project();
    // Create an epic
    let output = cmd()
        .current_dir(tmp.path())
        .args(["epic", "Test Epic"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let epic_id = extract_id(&output.stdout);

    // effort is not valid for epics
    cmd()
        .current_dir(tmp.path())
        .args(["update", &epic_id, "--effort", "large"])
        .assert()
        .failure();
}

// ── Link ─────────────────────────────────────────────────────────────────

#[test]
fn test_link_blocks() {
    let tmp = setup_project();
    let out1 = cmd()
        .current_dir(tmp.path())
        .args(["add", "Blocker"])
        .output()
        .unwrap();
    assert!(
        out1.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out1.stderr)
    );
    let id1 = extract_id(&out1.stdout);

    let out2 = cmd()
        .current_dir(tmp.path())
        .args(["add", "Blocked"])
        .output()
        .unwrap();
    assert!(
        out2.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out2.stderr)
    );
    let id2 = extract_id(&out2.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["link", &id1, &id2, "--relation", "blocks"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("{} blocks {}", id1, id2)));

    // Verify bidirectional
    let blocker = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", id1)),
    )
    .unwrap();
    assert!(blocker.contains(&id2));

    let blocked = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", id2)),
    )
    .unwrap();
    assert!(blocked.contains(&id1));
}

#[test]
fn test_link_missing_args_fails() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["add", "Lonely"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let task_id = extract_id(&output.stdout);

    // Missing TO and --relation
    cmd()
        .current_dir(tmp.path())
        .args(["link", &task_id])
        .assert()
        .failure();
}

#[test]
fn test_link_epic() {
    let tmp = setup_project();
    let task_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "A task"])
        .output()
        .unwrap();
    assert!(task_out.status.success());
    let task_id = extract_id(&task_out.stdout);

    let epic_out = cmd()
        .current_dir(tmp.path())
        .args(["epic", "An epic"])
        .output()
        .unwrap();
    assert!(epic_out.status.success());
    let epic_id = extract_id(&epic_out.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["link", &task_id, &epic_id, "--relation", "epic"])
        .assert()
        .success();

    let task_content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    assert!(task_content.contains(&epic_id));
}

#[test]
fn test_link_plan() {
    let tmp = setup_project();
    let task_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "A task"])
        .output()
        .unwrap();
    assert!(task_out.status.success());
    let task_id = extract_id(&task_out.stdout);

    // Create plan via the plan command (which now uses link_items internally)
    let plan_out = cmd()
        .current_dir(tmp.path())
        .args(["plan", &task_id])
        .output()
        .unwrap();
    assert!(
        plan_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&plan_out.stderr)
    );

    // Verify task.plan is set
    let task_content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    assert!(task_content.contains("plan: PLAN-"));
}

#[test]
fn test_link_remove() {
    let tmp = setup_project();
    let out1 = cmd()
        .current_dir(tmp.path())
        .args(["add", "A"])
        .output()
        .unwrap();
    assert!(out1.status.success());
    let id1 = extract_id(&out1.stdout);

    let out2 = cmd()
        .current_dir(tmp.path())
        .args(["add", "B"])
        .output()
        .unwrap();
    assert!(out2.status.success());
    let id2 = extract_id(&out2.stdout);

    // Add link
    cmd()
        .current_dir(tmp.path())
        .args(["link", &id1, &id2, "--relation", "blocks"])
        .assert()
        .success();

    // Remove link
    cmd()
        .current_dir(tmp.path())
        .args(["link", &id1, &id2, "--relation", "blocks", "--remove"])
        .assert()
        .success();

    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", id1)),
    )
    .unwrap();
    assert!(!content.contains(&id2));
}

#[test]
fn test_link_related_bidirectional() {
    let tmp = setup_project();
    let out1 = cmd()
        .current_dir(tmp.path())
        .args(["add", "Task A"])
        .output()
        .unwrap();
    assert!(out1.status.success());
    let id1 = extract_id(&out1.stdout);

    let out2 = cmd()
        .current_dir(tmp.path())
        .args(["add", "Task B"])
        .output()
        .unwrap();
    assert!(out2.status.success());
    let id2 = extract_id(&out2.stdout);

    // Link with related
    cmd()
        .current_dir(tmp.path())
        .args(["link", &id1, &id2, "--relation", "related"])
        .assert()
        .success();

    // Verify both files contain the reciprocal link
    let content1 = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", id1)),
    )
    .unwrap();
    let content2 = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", id2)),
    )
    .unwrap();
    assert!(
        content1.contains(&id2),
        "Task A should have related link to Task B"
    );
    assert!(
        content2.contains(&id1),
        "Task B should have related link to Task A"
    );
}

// ── Check ────────────────────────────────────────────────────────────────

#[test]
fn test_check_clean() {
    let tmp = setup_project();
    cmd()
        .current_dir(tmp.path())
        .args(["add", "Clean item"])
        .assert()
        .success();

    cmd()
        .current_dir(tmp.path())
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("No broken references"));
}

#[test]
fn test_check_asymmetric_links() {
    let tmp = setup_project();
    // Create two tasks and link them
    let out1 = cmd()
        .current_dir(tmp.path())
        .args(["add", "Blocker"])
        .output()
        .unwrap();
    assert!(out1.status.success());
    let id1 = extract_id(&out1.stdout);

    let out2 = cmd()
        .current_dir(tmp.path())
        .args(["add", "Blocked"])
        .output()
        .unwrap();
    assert!(out2.status.success());
    let id2 = extract_id(&out2.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["link", &id1, &id2, "--relation", "blocks"])
        .assert()
        .success();

    // Manually corrupt: remove depends_on from the blocked task
    let item_path = tmp
        .path()
        .join(format!(".markplane/backlog/items/{}.md", id2));
    let content = std::fs::read_to_string(&item_path).unwrap();
    let content = content.replace(&format!("depends_on:\n- {}", id1), "depends_on: []");
    std::fs::write(&item_path, content).unwrap();

    // Check should detect the asymmetric link
    cmd()
        .current_dir(tmp.path())
        .arg("check")
        .assert()
        .failure()
        .stdout(predicate::str::contains("asymmetric link"));
}

#[test]
fn test_check_fix_repairs_links() {
    let tmp = setup_project();
    let out1 = cmd()
        .current_dir(tmp.path())
        .args(["add", "Blocker"])
        .output()
        .unwrap();
    assert!(out1.status.success());
    let id1 = extract_id(&out1.stdout);

    let out2 = cmd()
        .current_dir(tmp.path())
        .args(["add", "Blocked"])
        .output()
        .unwrap();
    assert!(out2.status.success());
    let id2 = extract_id(&out2.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["link", &id1, &id2, "--relation", "blocks"])
        .assert()
        .success();

    // Manually corrupt: remove depends_on from the blocked task
    let item_path = tmp
        .path()
        .join(format!(".markplane/backlog/items/{}.md", id2));
    let content = std::fs::read_to_string(&item_path).unwrap();
    let content = content.replace(&format!("depends_on:\n- {}", id1), "depends_on: []");
    std::fs::write(&item_path, content).unwrap();

    // Fix should repair it
    cmd()
        .current_dir(tmp.path())
        .args(["check", "--fix"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Repaired"));

    // After fix, check should pass clean
    cmd()
        .current_dir(tmp.path())
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "All reciprocal links are symmetric",
        ));
}

// ── Metrics ──────────────────────────────────────────────────────────────

#[test]
fn test_metrics() {
    let tmp = setup_project();
    cmd()
        .current_dir(tmp.path())
        .args(["add", "Metrics item"])
        .assert()
        .success();

    cmd()
        .current_dir(tmp.path())
        .arg("metrics")
        .assert()
        .success()
        .stdout(predicate::str::contains("Task Status"))
        .stdout(predicate::str::contains("Total:"));
}

// ── Dashboard ────────────────────────────────────────────────────────────

#[test]
fn test_dashboard() {
    let tmp = setup_project();
    cmd()
        .current_dir(tmp.path())
        .args(["add", "Dashboard test"])
        .assert()
        .success();

    cmd()
        .current_dir(tmp.path())
        .arg("dashboard")
        .assert()
        .success()
        .stdout(predicate::str::contains("Project Dashboard"));
}

// ── No project ───────────────────────────────────────────────────────────

#[test]
fn test_commands_fail_without_init() {
    let tmp = TempDir::new().unwrap();

    cmd()
        .current_dir(tmp.path())
        .args(["add", "Should fail"])
        .assert()
        .failure();

    cmd().current_dir(tmp.path()).arg("ls").assert().failure();

    cmd()
        .current_dir(tmp.path())
        .args(["show", "TASK-001"])
        .assert()
        .failure();
}

// ── Help ─────────────────────────────────────────────────────────────────

#[test]
fn test_help() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("AI-native project management"))
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("show"))
        .stdout(predicate::str::contains("ls"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("sync"))
        .stdout(predicate::str::contains("start"))
        .stdout(predicate::str::contains("done"))
        .stdout(predicate::str::contains("promote"))
        .stdout(predicate::str::contains("plan"))
        .stdout(predicate::str::contains("epic"))
        .stdout(predicate::str::contains("note"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("link"))
        .stdout(predicate::str::contains("check"))
        .stdout(predicate::str::contains("stale"))
        .stdout(predicate::str::contains("archive"))
        .stdout(predicate::str::contains("context"))
        .stdout(predicate::str::contains("metrics"))
        .stdout(predicate::str::contains("graph"))
        .stdout(predicate::str::contains("dashboard"));
}

// ── Stale ─────────────────────────────────────────────────────────────────

#[test]
fn test_stale_no_stale_items() {
    let tmp = setup_project();
    cmd()
        .current_dir(tmp.path())
        .args(["add", "Fresh item"])
        .assert()
        .success();

    cmd()
        .current_dir(tmp.path())
        .args(["stale", "--days", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No items stale"));
}

#[test]
fn test_stale_with_old_items() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["add", "Old item"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let task_id = extract_id(&output.stdout);

    // Manually backdate the item's updated field to make it stale
    let item_path = tmp
        .path()
        .join(format!(".markplane/backlog/items/{}.md", task_id));
    let content = std::fs::read_to_string(&item_path).unwrap();
    let old_date = "2020-01-01";
    let today = chrono::Local::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    let content = content.replace(
        &format!("updated: {}", today),
        &format!("updated: {}", old_date),
    );
    std::fs::write(&item_path, content).unwrap();

    cmd()
        .current_dir(tmp.path())
        .args(["stale", "--days", "30"])
        .assert()
        .success()
        .stdout(predicate::str::contains(&task_id))
        .stdout(predicate::str::contains("Old item"));
}

// ── Archive ───────────────────────────────────────────────────────────────

#[test]
fn test_archive_nothing_to_archive() {
    let tmp = setup_project();
    cmd()
        .current_dir(tmp.path())
        .args(["add", "Draft item"])
        .assert()
        .success();

    // --all-done with only draft items should report nothing
    cmd()
        .current_dir(tmp.path())
        .args(["archive", "--all-done"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No completed items"));
}

#[test]
fn test_archive_dry_run() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["add", "To archive"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let task_id = extract_id(&output.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["status", &task_id, "done"])
        .assert()
        .success();

    cmd()
        .current_dir(tmp.path())
        .args(["archive", "--all-done", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Would archive"))
        .stdout(predicate::str::contains(&task_id));

    // File should still be in active dir (not moved)
    assert!(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id))
            .is_file()
    );
}

#[test]
fn test_archive_actual() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["add", "To archive"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let task_id = extract_id(&output.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["status", &task_id, "done"])
        .assert()
        .success();

    // Single-item archive by ID
    cmd()
        .current_dir(tmp.path())
        .args(["archive", &task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("Archived {}", task_id)));

    // File should be in archive dir
    assert!(
        tmp.path()
            .join(format!(".markplane/backlog/archive/{}.md", task_id))
            .is_file()
    );
    assert!(
        !tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id))
            .is_file()
    );
}

#[test]
fn test_archive_keep_cancelled() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["add", "To cancel"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let task_id = extract_id(&output.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["status", &task_id, "cancelled"])
        .assert()
        .success();

    // --all-done now archives cancelled tasks too
    cmd()
        .current_dir(tmp.path())
        .args(["archive", "--all-done"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Archived"));

    // File should be in archive dir
    assert!(
        tmp.path()
            .join(format!(".markplane/backlog/archive/{}.md", task_id))
            .is_file()
    );
}

#[test]
fn test_unarchive() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["add", "To archive and restore"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let task_id = extract_id(&output.stdout);

    // Archive it
    cmd()
        .current_dir(tmp.path())
        .args(["archive", &task_id])
        .assert()
        .success();

    assert!(
        tmp.path()
            .join(format!(".markplane/backlog/archive/{}.md", task_id))
            .is_file()
    );

    // Unarchive it
    cmd()
        .current_dir(tmp.path())
        .args(["unarchive", &task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("Restored {}", task_id)));

    assert!(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id))
            .is_file()
    );
    assert!(
        !tmp.path()
            .join(format!(".markplane/backlog/archive/{}.md", task_id))
            .is_file()
    );
}

#[test]
fn test_ls_archived() {
    let tmp = setup_project();
    let out1 = cmd()
        .current_dir(tmp.path())
        .args(["add", "Active item"])
        .output()
        .unwrap();
    assert!(
        out1.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out1.stderr)
    );
    let id1 = extract_id(&out1.stdout);

    let out2 = cmd()
        .current_dir(tmp.path())
        .args(["add", "Archived item"])
        .output()
        .unwrap();
    assert!(
        out2.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out2.stderr)
    );
    let id2 = extract_id(&out2.stdout);

    // Archive second item
    cmd()
        .current_dir(tmp.path())
        .args(["archive", &id2])
        .assert()
        .success();

    // Normal ls should only show active
    cmd()
        .current_dir(tmp.path())
        .arg("ls")
        .assert()
        .success()
        .stdout(predicate::str::contains(id1.as_str()))
        .stdout(predicate::str::contains(id2.as_str()).not());

    // ls --archived should only show archived
    cmd()
        .current_dir(tmp.path())
        .args(["ls", "--archived"])
        .assert()
        .success()
        .stdout(predicate::str::contains(id2.as_str()))
        .stdout(predicate::str::contains(id1.as_str()).not());
}

// ── Graph ─────────────────────────────────────────────────────────────────

#[test]
fn test_graph() {
    let tmp = setup_project();
    let out1 = cmd()
        .current_dir(tmp.path())
        .args(["add", "Blocker"])
        .output()
        .unwrap();
    assert!(
        out1.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out1.stderr)
    );
    let id1 = extract_id(&out1.stdout);

    let out2 = cmd()
        .current_dir(tmp.path())
        .args(["add", "Blocked"])
        .output()
        .unwrap();
    assert!(
        out2.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out2.stderr)
    );
    let id2 = extract_id(&out2.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["link", &id1, &id2, "--relation", "blocks"])
        .assert()
        .success();

    cmd()
        .current_dir(tmp.path())
        .args(["graph", &id1])
        .assert()
        .success()
        .stdout(predicate::str::contains(&id1))
        .stdout(predicate::str::contains(&id2));
}

// ── Context ───────────────────────────────────────────────────────────────

#[test]
fn test_context_regenerate() {
    let tmp = setup_project();
    cmd()
        .current_dir(tmp.path())
        .arg("context")
        .assert()
        .success()
        .stdout(predicate::str::contains("Context files regenerated"));

    assert!(tmp.path().join(".markplane/.context/summary.md").is_file());
}

#[test]
fn test_context_focus_active_work() {
    let tmp = setup_project();
    cmd()
        .current_dir(tmp.path())
        .args(["context", "--focus", "active-work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Active Work"));
}

#[test]
fn test_context_focus_blocked() {
    let tmp = setup_project();
    cmd()
        .current_dir(tmp.path())
        .args(["context", "--focus", "blocked"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Blocked"));
}

#[test]
fn test_context_focus_metrics() {
    let tmp = setup_project();
    cmd()
        .current_dir(tmp.path())
        .args(["context", "--focus", "metrics"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Metrics"));
}

#[test]
fn test_context_focus_summary() {
    let tmp = setup_project();
    cmd()
        .current_dir(tmp.path())
        .args(["context", "--focus", "summary"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Test Project"));
}

#[test]
fn test_context_focus_invalid() {
    let tmp = setup_project();
    cmd()
        .current_dir(tmp.path())
        .args(["context", "--focus", "bogus"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown focus area"));
}

// ── Error Cases ──────────────────────────────────────────────────────────

#[test]
fn test_add_invalid_type() {
    let tmp = setup_project();
    cmd()
        .current_dir(tmp.path())
        .args(["add", "Bad type", "--type", "invalid-type"])
        .assert()
        .failure();
}

#[test]
fn test_status_update_nonexistent_item() {
    let tmp = setup_project();
    cmd()
        .current_dir(tmp.path())
        .args(["status", "TASK-999", "in-progress"])
        .assert()
        .failure();
}

#[test]
fn test_plan_for_nonexistent_task() {
    let tmp = setup_project();
    cmd()
        .current_dir(tmp.path())
        .args(["plan", "TASK-999"])
        .assert()
        .failure();
}

#[test]
fn test_plan_for_non_task_item() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["epic", "Phase 1"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let epic_id = extract_id(&output.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["plan", &epic_id])
        .assert()
        .failure();
}

#[test]
fn test_show_invalid_id_format() {
    let tmp = setup_project();
    cmd()
        .current_dir(tmp.path())
        .args(["show", "not-a-valid-id"])
        .assert()
        .failure();
}

#[test]
fn test_add_invalid_priority() {
    let tmp = setup_project();
    cmd()
        .current_dir(tmp.path())
        .args(["add", "Bad priority", "--priority", "ultra-high"])
        .assert()
        .failure();
}

#[test]
fn test_add_invalid_effort() {
    let tmp = setup_project();
    cmd()
        .current_dir(tmp.path())
        .args(["add", "Bad effort", "--effort", "enormous"])
        .assert()
        .failure();
}

// ── Full workflow ────────────────────────────────────────────────────────

#[test]
fn test_full_workflow() {
    let tmp = setup_project();

    // Create epic
    let epic_out = cmd()
        .current_dir(tmp.path())
        .args(["epic", "Phase 1", "--priority", "high"])
        .output()
        .unwrap();
    assert!(
        epic_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&epic_out.stderr)
    );
    let epic_id = extract_id(&epic_out.stdout);

    // Create tasks
    let task1_out = cmd()
        .current_dir(tmp.path())
        .args([
            "add",
            "Build API",
            "--priority",
            "high",
            "--epic",
            &epic_id,
            "--tags",
            "api",
        ])
        .output()
        .unwrap();
    assert!(
        task1_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&task1_out.stderr)
    );
    let task1_id = extract_id(&task1_out.stdout);

    let task2_out = cmd()
        .current_dir(tmp.path())
        .args([
            "add",
            "Build UI",
            "--priority",
            "medium",
            "--epic",
            &epic_id,
            "--tags",
            "ui",
        ])
        .output()
        .unwrap();
    assert!(
        task2_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&task2_out.stderr)
    );
    let task2_id = extract_id(&task2_out.stdout);

    // Link
    cmd()
        .current_dir(tmp.path())
        .args(["link", &task2_id, &task1_id, "--relation", "depends-on"])
        .assert()
        .success();

    // Start work
    cmd()
        .current_dir(tmp.path())
        .args(["start", &task1_id, "--user", "alice"])
        .assert()
        .success();

    // Create plan
    cmd()
        .current_dir(tmp.path())
        .args(["plan", &task1_id])
        .assert()
        .success();

    // Finish
    cmd()
        .current_dir(tmp.path())
        .args(["done", &task1_id])
        .assert()
        .success();

    // Sync everything
    cmd().current_dir(tmp.path()).arg("sync").assert().success();

    // Check references
    cmd()
        .current_dir(tmp.path())
        .arg("check")
        .assert()
        .success();

    // List should show items
    cmd()
        .current_dir(tmp.path())
        .arg("ls")
        .assert()
        .success()
        .stdout(predicate::str::contains(&task1_id))
        .stdout(predicate::str::contains(&task2_id));

    // Dashboard should work
    cmd()
        .current_dir(tmp.path())
        .arg("dashboard")
        .assert()
        .success();

    // Metrics should work
    cmd()
        .current_dir(tmp.path())
        .arg("metrics")
        .assert()
        .success();
}

// ── Comments ─────────────────────────────────────────────────────────

#[test]
fn test_comment_on_task_appends_under_comments() {
    let tmp = setup_project();
    let add_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "Commentable task"])
        .output()
        .unwrap();
    assert!(add_out.status.success());
    let task_id = extract_id(&add_out.stdout);

    let body_path = tmp.path().join("comment.md");
    std::fs::write(&body_path, "First discussion point.\n").unwrap();

    let today = chrono::Local::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    let comment_out = cmd()
        .current_dir(tmp.path())
        .env("USER", "alice")
        .args([
            "comment",
            &task_id,
            "--body-file",
            body_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        comment_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&comment_out.stderr)
    );

    let file = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    let body = file.splitn(3, "---").nth(2).unwrap_or("");
    assert!(
        body.contains("## Comments"),
        "expected ## Comments section, got:\n{}",
        body
    );
    let heading = format!("### {} alice", today);
    assert!(
        body.contains(&heading),
        "expected heading {:?}, got:\n{}",
        heading,
        body
    );
    assert!(body.contains("First discussion point."));

    // show includes comments
    cmd()
        .current_dir(tmp.path())
        .args(["show", &task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("## Comments"))
        .stdout(predicate::str::contains(&heading))
        .stdout(predicate::str::contains("First discussion point."));
}

#[test]
fn test_comment_respects_markplane_author() {
    let tmp = setup_project();
    let add_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "Author env task"])
        .output()
        .unwrap();
    assert!(add_out.status.success());
    let task_id = extract_id(&add_out.stdout);

    let body_path = tmp.path().join("comment.md");
    std::fs::write(&body_path, "From the agent.\n").unwrap();

    let today = chrono::Local::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    let comment_out = cmd()
        .current_dir(tmp.path())
        .env("MARKPLANE_AUTHOR", "agent-implement")
        .env("USER", "alice")
        .args([
            "comment",
            &task_id,
            "--body-file",
            body_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        comment_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&comment_out.stderr)
    );

    let file = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    let body = file.splitn(3, "---").nth(2).unwrap_or("");
    let heading = format!("### {} agent-implement", today);
    assert!(
        body.contains(&heading),
        "expected heading {:?}, got:\n{}",
        heading,
        body
    );
    assert!(
        !body.contains(&format!("### {} alice", today)),
        "MARKPLANE_AUTHOR should win over USER; body:\n{}",
        body
    );
}

#[test]
fn test_comment_accumulates_and_works_on_all_kinds() {
    let tmp = setup_project();
    let today = chrono::Local::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    let comment_path = tmp.path().join("c.md");

    // TASK: two comments accumulate in order
    let task_id = extract_id(
        &cmd()
            .current_dir(tmp.path())
            .args(["add", "Multi-comment task"])
            .output()
            .unwrap()
            .stdout,
    );
    std::fs::write(&comment_path, "Comment one.\n").unwrap();
    cmd()
        .current_dir(tmp.path())
        .env("USER", "alice")
        .args([
            "comment",
            &task_id,
            "--body-file",
            comment_path.to_str().unwrap(),
        ])
        .assert()
        .success();
    std::fs::write(&comment_path, "Comment two.\n").unwrap();
    cmd()
        .current_dir(tmp.path())
        .env("USER", "bob")
        .args([
            "comment",
            &task_id,
            "--body-file",
            comment_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let task_file = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    let task_body = task_file.splitn(3, "---").nth(2).unwrap_or("");
    let one_pos = task_body
        .find("Comment one.")
        .expect("first comment missing");
    let two_pos = task_body
        .find("Comment two.")
        .expect("second comment missing");
    assert!(one_pos < two_pos, "comments should accumulate in order");
    assert!(task_body.contains(&format!("### {} alice", today)));
    assert!(task_body.contains(&format!("### {} bob", today)));
    assert_eq!(
        task_body.matches("## Comments").count(),
        1,
        "should have a single Comments section"
    );

    // EPIC
    let epic_id = extract_id(
        &cmd()
            .current_dir(tmp.path())
            .args(["epic", "Comment epic"])
            .output()
            .unwrap()
            .stdout,
    );
    std::fs::write(&comment_path, "Epic comment.\n").unwrap();
    cmd()
        .current_dir(tmp.path())
        .env("USER", "carol")
        .args([
            "comment",
            &epic_id,
            "--body-file",
            comment_path.to_str().unwrap(),
        ])
        .assert()
        .success();
    cmd()
        .current_dir(tmp.path())
        .args(["show", &epic_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("## Comments"))
        .stdout(predicate::str::contains(format!("### {} carol", today)))
        .stdout(predicate::str::contains("Epic comment."));

    // PLAN
    let plan_id = extract_id(
        &cmd()
            .current_dir(tmp.path())
            .args(["plan", &task_id])
            .output()
            .unwrap()
            .stdout,
    );
    std::fs::write(&comment_path, "Plan comment.\n").unwrap();
    cmd()
        .current_dir(tmp.path())
        .env("USER", "dave")
        .args([
            "comment",
            &plan_id,
            "--body-file",
            comment_path.to_str().unwrap(),
        ])
        .assert()
        .success();
    cmd()
        .current_dir(tmp.path())
        .args(["show", &plan_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("## Comments"))
        .stdout(predicate::str::contains(format!("### {} dave", today)))
        .stdout(predicate::str::contains("Plan comment."));

    // NOTE
    let note_id = extract_id(
        &cmd()
            .current_dir(tmp.path())
            .args(["note", "Comment note"])
            .output()
            .unwrap()
            .stdout,
    );
    std::fs::write(&comment_path, "Note comment.\n").unwrap();
    cmd()
        .current_dir(tmp.path())
        .env("USER", "erin")
        .args([
            "comment",
            &note_id,
            "--body-file",
            comment_path.to_str().unwrap(),
        ])
        .assert()
        .success();
    cmd()
        .current_dir(tmp.path())
        .args(["show", &note_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("## Comments"))
        .stdout(predicate::str::contains(format!("### {} erin", today)))
        .stdout(predicate::str::contains("Note comment."));
}

// ── MARKPLANE_ROOT ───────────────────────────────────────────────────

#[test]
fn test_markplane_root_reads_and_writes_specified_project() {
    let main = setup_project();
    let add_out = cmd()
        .current_dir(main.path())
        .args(["add", "Root env task"])
        .output()
        .unwrap();
    assert!(add_out.status.success());
    let task_id = extract_id(&add_out.stdout);

    // Simulate a worktree: cwd has no .markplane/, but MARKPLANE_ROOT points at main.
    let worktree = TempDir::new().unwrap();
    let body_path = worktree.path().join("comment.md");
    std::fs::write(&body_path, "Written from worktree.\n").unwrap();

    cmd()
        .current_dir(worktree.path())
        .env("MARKPLANE_ROOT", main.path())
        .args(["show", &task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Root env task"));

    let today = chrono::Local::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    cmd()
        .current_dir(worktree.path())
        .env("MARKPLANE_ROOT", main.path())
        .env("USER", "agent")
        .args([
            "comment",
            &task_id,
            "--body-file",
            body_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let file = std::fs::read_to_string(
        main.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    let body = file.splitn(3, "---").nth(2).unwrap_or("");
    assert!(
        body.contains(&format!("### {} agent", today)),
        "comment should land in main workspace; body:\n{}",
        body
    );
    assert!(body.contains("Written from worktree."));
}

#[test]
fn test_markplane_root_overrides_cwd_project() {
    let main = setup_project();
    let other = setup_project();

    let main_task = extract_id(
        &cmd()
            .current_dir(main.path())
            .args(["add", "Main only"])
            .output()
            .unwrap()
            .stdout,
    );
    let other_task = extract_id(
        &cmd()
            .current_dir(other.path())
            .args(["add", "Other only"])
            .output()
            .unwrap()
            .stdout,
    );

    // From other's cwd, MARKPLANE_ROOT=main must use main (no upward/cwd search).
    cmd()
        .current_dir(other.path())
        .env("MARKPLANE_ROOT", main.path())
        .args(["show", &main_task])
        .assert()
        .success()
        .stdout(predicate::str::contains("Main only"));

    cmd()
        .current_dir(other.path())
        .env("MARKPLANE_ROOT", main.path())
        .args(["show", &other_task])
        .assert()
        .failure()
        .stderr(predicate::str::contains(&other_task));
}

#[test]
fn test_markplane_root_unset_walks_from_cwd() {
    let tmp = setup_project();
    let task_id = extract_id(
        &cmd()
            .current_dir(tmp.path())
            .args(["add", "Nested show"])
            .output()
            .unwrap()
            .stdout,
    );

    let nested = tmp.path().join("deep/nested/cwd");
    std::fs::create_dir_all(&nested).unwrap();

    cmd()
        .current_dir(&nested)
        .env_remove("MARKPLANE_ROOT")
        .args(["show", &task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Nested show"));
}

#[test]
fn test_markplane_root_missing_markplane_errors_clearly() {
    let empty = TempDir::new().unwrap();
    let root = empty.path().to_string_lossy().to_string();

    cmd()
        .current_dir(empty.path())
        .env("MARKPLANE_ROOT", empty.path())
        .args(["show", "TASK-xxxxx"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("MARKPLANE_ROOT"))
        .stderr(predicate::str::contains(".markplane"))
        .stderr(predicate::str::contains(root.as_str()));
}

// ── Body file ────────────────────────────────────────────────────────

#[test]
fn test_add_with_body_file() {
    let tmp = setup_project();
    let body_path = tmp.path().join("body.md");
    std::fs::write(&body_path, "## Description\n\nCustom body from file.\n").unwrap();

    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "add",
            "Body file task",
            "--body-file",
            body_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let task_id = extract_id(&output.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["show", &task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Custom body from file"))
        .stdout(predicate::str::contains("## Description"));
}

#[test]
fn test_epic_plan_note_with_body_file() {
    let tmp = setup_project();
    let body_path = tmp.path().join("body.md");
    std::fs::write(&body_path, "## Objective\n\nEpic body from file.\n").unwrap();

    let epic_out = cmd()
        .current_dir(tmp.path())
        .args([
            "epic",
            "Body epic",
            "--body-file",
            body_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        epic_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&epic_out.stderr)
    );
    let epic_id = extract_id(&epic_out.stdout);
    cmd()
        .current_dir(tmp.path())
        .args(["show", &epic_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Epic body from file"));

    std::fs::write(&body_path, "## Overview\n\nPlan body from file.\n").unwrap();
    let task_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "Task for plan"])
        .output()
        .unwrap();
    assert!(task_out.status.success());
    let task_id = extract_id(&task_out.stdout);

    let plan_out = cmd()
        .current_dir(tmp.path())
        .args(["plan", &task_id, "--body-file", body_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        plan_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&plan_out.stderr)
    );
    let plan_id = extract_id(&plan_out.stdout);
    cmd()
        .current_dir(tmp.path())
        .args(["show", &plan_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Plan body from file"));

    std::fs::write(&body_path, "## Findings\n\nNote body from file.\n").unwrap();
    let note_out = cmd()
        .current_dir(tmp.path())
        .args([
            "note",
            "Body note",
            "--body-file",
            body_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        note_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&note_out.stderr)
    );
    let note_id = extract_id(&note_out.stdout);
    cmd()
        .current_dir(tmp.path())
        .args(["show", &note_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Note body from file"));
}

#[test]
fn test_add_body_file_stdin() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["add", "Stdin body task", "--body-file", "-"])
        .write_stdin("## Description\n\nBody from stdin.\n")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let task_id = extract_id(&output.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["show", &task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Body from stdin"));
}

#[test]
fn test_add_without_body_file_uses_template() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["add", "Template body task"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let task_id = extract_id(&output.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["show", &task_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("## Description"))
        .stdout(predicate::str::contains("## Acceptance Criteria"))
        .stdout(predicate::str::contains("[What needs to be done"));
}

// ── Template flag ────────────────────────────────────────────────────

#[test]
fn test_add_with_template_bug() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["add", "Login crash", "--type", "bug", "--template", "bug"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let task_id = extract_id(&output.stdout);

    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    assert!(content.contains("## Steps to Reproduce"));
}

#[test]
fn test_plan_with_template_refactor() {
    let tmp = setup_project();
    let task_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "Refactor auth"])
        .output()
        .unwrap();
    assert!(
        task_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&task_out.stderr)
    );
    let task_id = extract_id(&task_out.stdout);

    let plan_out = cmd()
        .current_dir(tmp.path())
        .args(["plan", &task_id, "--template", "refactor"])
        .output()
        .unwrap();
    assert!(
        plan_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&plan_out.stderr)
    );
    let plan_id = extract_id(&plan_out.stdout);

    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/plans/items/{}.md", plan_id)),
    )
    .unwrap();
    assert!(content.contains("## Motivation"));
    assert!(content.contains("## Current State"));
}

#[test]
fn test_note_with_template_research() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["note", "Caching study", "--template", "research"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let note_id = extract_id(&output.stdout);

    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/notes/items/{}.md", note_id)),
    )
    .unwrap();
    assert!(content.contains("## Findings"));
    assert!(content.contains("## Recommendations"));
}

// ── Standalone plan / Plan.tags / spec template / --plan ─────────────

#[test]
fn test_standalone_plan_with_tags() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "plan",
            "--title",
            "Standalone refactor",
            "--tags",
            "ready-for-agent",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let plan_id = extract_id(&output.stdout);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Created"));
    assert!(stdout.contains("Standalone refactor"));
    assert!(!stdout.contains("Linked to"));

    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/plans/items/{}.md", plan_id)),
    )
    .unwrap();
    assert!(content.contains("tags:"));
    assert!(content.contains("ready-for-agent"));
    assert!(content.contains("implements: []") || content.contains("implements:"));
}

#[test]
fn test_epic_with_tags() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["epic", "Tagged epic", "--tags", "ready-for-agent,core"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let epic_id = extract_id(&output.stdout);

    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/roadmap/items/{}.md", epic_id)),
    )
    .unwrap();
    assert!(content.contains("ready-for-agent"));
    assert!(content.contains("core"));
}

#[test]
fn test_add_and_ls_plan() {
    let tmp = setup_project();

    let plan_out = cmd()
        .current_dir(tmp.path())
        .args(["plan", "--title", "Parent plan"])
        .output()
        .unwrap();
    assert!(plan_out.status.success());
    let plan_id = extract_id(&plan_out.stdout);

    let other_plan_out = cmd()
        .current_dir(tmp.path())
        .args(["plan", "--title", "Other plan"])
        .output()
        .unwrap();
    assert!(other_plan_out.status.success());
    let other_plan_id = extract_id(&other_plan_out.stdout);

    let task_a = cmd()
        .current_dir(tmp.path())
        .args(["add", "Task A", "--plan", &plan_id])
        .output()
        .unwrap();
    assert!(
        task_a.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&task_a.stderr)
    );
    let task_a_id = extract_id(&task_a.stdout);

    let task_b = cmd()
        .current_dir(tmp.path())
        .args(["add", "Task B", "--plan", &other_plan_id])
        .output()
        .unwrap();
    assert!(task_b.status.success());
    let task_b_id = extract_id(&task_b.stdout);

    let task_a_content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_a_id)),
    )
    .unwrap();
    assert!(task_a_content.contains(&format!("plan: {plan_id}")));

    let plan_content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/plans/items/{}.md", plan_id)),
    )
    .unwrap();
    assert!(plan_content.contains(&task_a_id));

    let ls_out = cmd()
        .current_dir(tmp.path())
        .args(["ls", "--plan", &plan_id])
        .output()
        .unwrap();
    assert!(
        ls_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&ls_out.stderr)
    );
    let ls_stdout = String::from_utf8_lossy(&ls_out.stdout);
    assert!(ls_stdout.contains(&task_a_id));
    assert!(ls_stdout.contains("Task A"));
    assert!(!ls_stdout.contains(&task_b_id));
    assert!(!ls_stdout.contains("Task B"));
}

#[test]
fn test_epic_and_plan_with_spec_template() {
    let tmp = setup_project();

    let epic_out = cmd()
        .current_dir(tmp.path())
        .args(["epic", "Spec epic", "--template", "spec"])
        .output()
        .unwrap();
    assert!(
        epic_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&epic_out.stderr)
    );
    let epic_id = extract_id(&epic_out.stdout);
    let epic_content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/roadmap/items/{}.md", epic_id)),
    )
    .unwrap();
    for section in [
        "## Problem Statement",
        "## Solution",
        "## User Stories",
        "## Implementation Decisions",
        "## Testing Decisions",
        "## Out of Scope",
        "## Further Notes",
    ] {
        assert!(
            epic_content.contains(section),
            "epic missing section {section}"
        );
    }

    let plan_out = cmd()
        .current_dir(tmp.path())
        .args(["plan", "--title", "Spec plan", "--template", "spec"])
        .output()
        .unwrap();
    assert!(
        plan_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&plan_out.stderr)
    );
    let plan_id = extract_id(&plan_out.stdout);
    let plan_content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/plans/items/{}.md", plan_id)),
    )
    .unwrap();
    for section in [
        "## Problem Statement",
        "## Solution",
        "## User Stories",
        "## Implementation Decisions",
        "## Testing Decisions",
        "## Out of Scope",
        "## Further Notes",
    ] {
        assert!(
            plan_content.contains(section),
            "plan missing section {section}"
        );
    }
}

// ── Frontier ─────────────────────────────────────────────────────────────

#[test]
fn test_ls_epic_frontier_cancelled_blocker_and_claim() {
    let tmp = setup_project();

    let epic_out = cmd()
        .current_dir(tmp.path())
        .args(["epic", "Frontier epic"])
        .output()
        .unwrap();
    assert!(epic_out.status.success());
    let epic_id = extract_id(&epic_out.stdout);

    let other_epic_out = cmd()
        .current_dir(tmp.path())
        .args(["epic", "Other epic"])
        .output()
        .unwrap();
    assert!(other_epic_out.status.success());
    let other_epic_id = extract_id(&other_epic_out.stdout);

    let blocker_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "Blocker", "--epic", &epic_id])
        .output()
        .unwrap();
    assert!(blocker_out.status.success());
    let blocker_id = extract_id(&blocker_out.stdout);

    let dependent_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "Dependent", "--epic", &epic_id])
        .output()
        .unwrap();
    assert!(dependent_out.status.success());
    let dependent_id = extract_id(&dependent_out.stdout);

    let claimed_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "Claimed", "--epic", &epic_id])
        .output()
        .unwrap();
    assert!(claimed_out.status.success());
    let claimed_id = extract_id(&claimed_out.stdout);

    let outsider_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "Outsider", "--epic", &other_epic_id])
        .output()
        .unwrap();
    assert!(outsider_out.status.success());
    let outsider_id = extract_id(&outsider_out.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["link", &blocker_id, &dependent_id, "--relation", "blocks"])
        .assert()
        .success();

    cmd()
        .current_dir(tmp.path())
        .args(["start", &claimed_id, "--user", "alice"])
        .assert()
        .success();

    // While blocker is open, dependent is not on the frontier
    let blocked_frontier = cmd()
        .current_dir(tmp.path())
        .args(["ls", "--epic", &epic_id, "--frontier"])
        .output()
        .unwrap();
    assert!(blocked_frontier.status.success());
    let blocked_stdout = String::from_utf8_lossy(&blocked_frontier.stdout);
    assert!(blocked_stdout.contains(&blocker_id));
    assert!(!blocked_stdout.contains(&dependent_id));
    assert!(!blocked_stdout.contains(&claimed_id));
    assert!(!blocked_stdout.contains(&outsider_id));

    cmd()
        .current_dir(tmp.path())
        .args(["status", &blocker_id, "cancelled"])
        .assert()
        .success();

    let frontier = cmd()
        .current_dir(tmp.path())
        .args(["ls", "--epic", &epic_id, "--frontier"])
        .output()
        .unwrap();
    assert!(
        frontier.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&frontier.stderr)
    );
    let stdout = String::from_utf8_lossy(&frontier.stdout);
    assert!(stdout.contains(&dependent_id));
    assert!(stdout.contains("Dependent"));
    assert!(!stdout.contains(&blocker_id));
    assert!(!stdout.contains(&claimed_id));
    assert!(!stdout.contains(&outsider_id));

    // Position order: move claimed aside; dependent should appear before a later open task
    let later_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "Later open", "--epic", &epic_id])
        .output()
        .unwrap();
    assert!(later_out.status.success());
    let later_id = extract_id(&later_out.stdout);

    let ordered = cmd()
        .current_dir(tmp.path())
        .args(["ls", "--epic", &epic_id, "--frontier"])
        .output()
        .unwrap();
    assert!(ordered.status.success());
    let ordered_stdout = String::from_utf8_lossy(&ordered.stdout);
    let dep_pos = ordered_stdout.find(&dependent_id).unwrap();
    let later_pos = ordered_stdout.find(&later_id).unwrap();
    assert!(dep_pos < later_pos, "frontier should be sorted by position");
}

#[test]
fn test_ls_plan_frontier() {
    let tmp = setup_project();

    let plan_out = cmd()
        .current_dir(tmp.path())
        .args(["plan", "--title", "Frontier plan"])
        .output()
        .unwrap();
    assert!(plan_out.status.success());
    let plan_id = extract_id(&plan_out.stdout);

    let other_plan_out = cmd()
        .current_dir(tmp.path())
        .args(["plan", "--title", "Other plan"])
        .output()
        .unwrap();
    assert!(other_plan_out.status.success());
    let other_plan_id = extract_id(&other_plan_out.stdout);

    let ready_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "Ready", "--plan", &plan_id])
        .output()
        .unwrap();
    assert!(ready_out.status.success());
    let ready_id = extract_id(&ready_out.stdout);

    let other_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "Other plan task", "--plan", &other_plan_id])
        .output()
        .unwrap();
    assert!(other_out.status.success());
    let other_id = extract_id(&other_out.stdout);

    let ls_out = cmd()
        .current_dir(tmp.path())
        .args(["ls", "--plan", &plan_id, "--frontier"])
        .output()
        .unwrap();
    assert!(
        ls_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&ls_out.stderr)
    );
    let stdout = String::from_utf8_lossy(&ls_out.stdout);
    assert!(stdout.contains(&ready_id));
    assert!(stdout.contains("Ready"));
    assert!(!stdout.contains(&other_id));
}

// ── Map ──────────────────────────────────────────────────────────────────

#[test]
fn test_map_creates_with_five_sections() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["map", "Chart the summit"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let map_id = extract_id(&output.stdout);
    assert!(map_id.starts_with("MAP-"));

    let path = tmp
        .path()
        .join(format!(".markplane/maps/items/{}.md", map_id));
    assert!(path.is_file());
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("status: active"));
    assert!(content.contains("## Destination"));
    assert!(content.contains("## Notes"));
    assert!(content.contains("## Decisions so far"));
    assert!(content.contains("## Not yet specified"));
    assert!(content.contains("## Out of scope"));
    assert!(!content.contains("priority:"));
    assert!(!content.contains("assignee:"));
}

#[test]
fn test_map_body_file_overrides_template() {
    let tmp = setup_project();
    let body_path = tmp.path().join("map-body.md");
    std::fs::write(
        &body_path,
        "## Destination\n\nCustom dest.\n\n## Notes\n\nCustom notes.\n",
    )
    .unwrap();

    let output = cmd()
        .current_dir(tmp.path())
        .args([
            "map",
            "Custom map",
            "--body-file",
            body_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let map_id = extract_id(&output.stdout);
    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/maps/items/{}.md", map_id)),
    )
    .unwrap();
    assert!(content.contains("# Custom map"));
    assert!(content.contains("Custom dest."));
    assert!(content.contains("Custom notes."));
    assert!(!content.contains("## Decisions so far"));
}

#[test]
fn test_map_status_done_and_rejects_invalid() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["map", "Status map"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let map_id = extract_id(&output.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["status", &map_id, "done"])
        .assert()
        .success()
        .stdout(predicate::str::contains("done"));

    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/maps/items/{}.md", map_id)),
    )
    .unwrap();
    assert!(content.contains("status: done"));

    cmd()
        .current_dir(tmp.path())
        .args(["status", &map_id, "abandoned"])
        .assert()
        .success();

    cmd()
        .current_dir(tmp.path())
        .args(["status", &map_id, "in-progress"])
        .assert()
        .failure();
}

#[test]
fn test_map_ls_and_show() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["map", "Listable map"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let map_id = extract_id(&output.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["ls", "maps"])
        .assert()
        .success()
        .stdout(predicate::str::contains(&map_id))
        .stdout(predicate::str::contains("Listable map"));

    cmd()
        .current_dir(tmp.path())
        .args(["show", &map_id])
        .assert()
        .success()
        .stdout(predicate::str::contains(&map_id))
        .stdout(predicate::str::contains("Listable map"))
        .stdout(predicate::str::contains("active"))
        .stdout(predicate::str::contains("Destination"));
}

#[test]
fn test_map_check_warns_missing_sections_without_failing() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["map", "Incomplete map"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let map_id = extract_id(&output.stdout);

    let path = tmp
        .path()
        .join(format!(".markplane/maps/items/{}.md", map_id));
    let mut content = std::fs::read_to_string(&path).unwrap();
    // Strip required sections so only Destination remains in body after title
    let fm_end = content.find("\n---\n").unwrap() + 5;
    let front = &content[..fm_end];
    content = format!("{front}\n# Incomplete map\n\n## Destination\n\nOnly this.\n");
    std::fs::write(&path, &content).unwrap();

    cmd()
        .current_dir(tmp.path())
        .args(["check"])
        .assert()
        .success()
        .stdout(predicate::str::contains("missing section"))
        .stdout(predicate::str::contains(&map_id));
}

#[test]
fn test_map_wiki_link_resolves() {
    let tmp = setup_project();
    let map_out = cmd()
        .current_dir(tmp.path())
        .args(["map", "Linked map"])
        .output()
        .unwrap();
    assert!(map_out.status.success());
    let map_id = extract_id(&map_out.stdout);

    let task_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "Mentions map"])
        .output()
        .unwrap();
    assert!(task_out.status.success());
    let task_id = extract_id(&task_out.stdout);

    let task_path = tmp
        .path()
        .join(format!(".markplane/backlog/items/{}.md", task_id));
    let mut task_content = std::fs::read_to_string(&task_path).unwrap();
    task_content.push_str(&format!("\nSee [[{}]] for the route.\n", map_id));
    std::fs::write(&task_path, task_content).unwrap();

    cmd()
        .current_dir(tmp.path())
        .args(["check"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No broken references"));
}

#[test]
fn test_map_sync_generates_index() {
    let tmp = setup_project();
    let output = cmd()
        .current_dir(tmp.path())
        .args(["map", "Synced map"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let map_id = extract_id(&output.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["sync"])
        .assert()
        .success();

    let index = std::fs::read_to_string(tmp.path().join(".markplane/maps/INDEX.md")).unwrap();
    assert!(index.contains(&map_id));
    assert!(index.contains("Synced map"));
}

// ── Decision tickets (Task.map) ──────────────────────────────────────────

#[test]
fn test_add_map_accepts_wayfinder_types_rejects_feature() {
    let tmp = setup_project();
    let map_out = cmd()
        .current_dir(tmp.path())
        .args(["map", "Decision map"])
        .output()
        .unwrap();
    assert!(map_out.status.success());
    let map_id = extract_id(&map_out.stdout);

    let ok = cmd()
        .current_dir(tmp.path())
        .args(["add", "Grill this", "--map", &map_id, "--type", "grilling"])
        .output()
        .unwrap();
    assert!(
        ok.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&ok.stderr)
    );
    let task_id = extract_id(&ok.stdout);
    let content = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    assert!(content.contains(&format!("map: {}", map_id)));
    assert!(content.contains("type: grilling"));
    assert!(content.contains("priority: medium"));
    assert!(content.contains("epic: null"));
    assert!(content.contains("plan: null"));

    let bad = cmd()
        .current_dir(tmp.path())
        .args([
            "add",
            "Not a decision",
            "--map",
            &map_id,
            "--type",
            "feature",
        ])
        .output()
        .unwrap();
    assert!(
        !bad.status.success(),
        "feature type should be rejected for decision tickets"
    );
    let stderr = String::from_utf8_lossy(&bad.stderr);
    assert!(
        stderr.contains("research") || stderr.contains("grilling") || stderr.contains("type"),
        "stderr should mention valid decision types: {}",
        stderr
    );

    // prototype and task also accepted without config changes
    for ty in ["research", "prototype", "task"] {
        let out = cmd()
            .current_dir(tmp.path())
            .args([
                "add",
                &format!("Ticket {ty}"),
                "--map",
                &map_id,
                "--type",
                ty,
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "type {ty} should succeed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn test_map_mutex_with_epic_and_plan_on_add_and_link() {
    let tmp = setup_project();

    let map_out = cmd()
        .current_dir(tmp.path())
        .args(["map", "Mutex map"])
        .output()
        .unwrap();
    assert!(map_out.status.success());
    let map_id = extract_id(&map_out.stdout);

    let epic_out = cmd()
        .current_dir(tmp.path())
        .args(["epic", "Mutex epic"])
        .output()
        .unwrap();
    assert!(epic_out.status.success());
    let epic_id = extract_id(&epic_out.stdout);

    let plan_out = cmd()
        .current_dir(tmp.path())
        .args(["plan", "--title", "Mutex plan"])
        .output()
        .unwrap();
    assert!(plan_out.status.success());
    let plan_id = extract_id(&plan_out.stdout);

    // add --map + --epic rejected
    let map_epic = cmd()
        .current_dir(tmp.path())
        .args([
            "add", "Both", "--map", &map_id, "--epic", &epic_id, "--type", "grilling",
        ])
        .output()
        .unwrap();
    assert!(!map_epic.status.success());

    // add --map + --plan rejected
    let map_plan = cmd()
        .current_dir(tmp.path())
        .args([
            "add",
            "Both plan",
            "--map",
            &map_id,
            "--plan",
            &plan_id,
            "--type",
            "research",
        ])
        .output()
        .unwrap();
    assert!(!map_plan.status.success());

    // Decision ticket cannot gain epic via link
    let dec_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "Decision", "--map", &map_id, "--type", "task"])
        .output()
        .unwrap();
    assert!(dec_out.status.success());
    let dec_id = extract_id(&dec_out.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["link", &dec_id, &epic_id, "-r", "epic"])
        .assert()
        .failure();

    cmd()
        .current_dir(tmp.path())
        .args(["link", &dec_id, &plan_id, "-r", "plan"])
        .assert()
        .failure();

    // Implementation ticket cannot gain map via link
    let impl_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "Impl", "--epic", &epic_id])
        .output()
        .unwrap();
    assert!(impl_out.status.success());
    let impl_id = extract_id(&impl_out.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["link", &impl_id, &map_id, "-r", "map"])
        .assert()
        .failure();
}

#[test]
fn test_link_map_sets_and_reparents() {
    let tmp = setup_project();

    let map1_out = cmd()
        .current_dir(tmp.path())
        .args(["map", "Map one"])
        .output()
        .unwrap();
    let map1 = extract_id(&map1_out.stdout);

    let map2_out = cmd()
        .current_dir(tmp.path())
        .args(["map", "Map two"])
        .output()
        .unwrap();
    let map2 = extract_id(&map2_out.stdout);

    let dec_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "Reparent me", "--map", &map1, "--type", "grilling"])
        .output()
        .unwrap();
    assert!(dec_out.status.success());
    let dec_id = extract_id(&dec_out.stdout);

    let content1 = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", dec_id)),
    )
    .unwrap();
    assert!(content1.contains(&format!("map: {}", map1)));

    cmd()
        .current_dir(tmp.path())
        .args(["link", &dec_id, &map2, "-r", "map"])
        .assert()
        .success();

    let content2 = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", dec_id)),
    )
    .unwrap();
    assert!(content2.contains(&format!("map: {}", map2)));
    assert!(!content2.contains(&format!("map: {}", map1)));

    // Relation is Task-side only: Map frontmatter does not list child tickets
    let map2_body = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/maps/items/{}.md", map2)),
    )
    .unwrap();
    let map2_fm = map2_body.split("---").nth(1).unwrap_or("");
    assert!(!map2_fm.contains(&dec_id));
}

#[test]
fn test_ls_map_and_frontier() {
    let tmp = setup_project();

    let map_out = cmd()
        .current_dir(tmp.path())
        .args(["map", "Frontier map"])
        .output()
        .unwrap();
    let map_id = extract_id(&map_out.stdout);

    let other_map_out = cmd()
        .current_dir(tmp.path())
        .args(["map", "Other map"])
        .output()
        .unwrap();
    let other_map = extract_id(&other_map_out.stdout);

    let blocker_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "Blocker", "--map", &map_id, "--type", "research"])
        .output()
        .unwrap();
    let blocker_id = extract_id(&blocker_out.stdout);

    let dependent_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "Dependent", "--map", &map_id, "--type", "grilling"])
        .output()
        .unwrap();
    let dependent_id = extract_id(&dependent_out.stdout);

    let claimed_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "Claimed", "--map", &map_id, "--type", "task"])
        .output()
        .unwrap();
    let claimed_id = extract_id(&claimed_out.stdout);

    let outsider_out = cmd()
        .current_dir(tmp.path())
        .args([
            "add",
            "Outsider",
            "--map",
            &other_map,
            "--type",
            "prototype",
        ])
        .output()
        .unwrap();
    let outsider_id = extract_id(&outsider_out.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["link", &blocker_id, &dependent_id, "-r", "blocks"])
        .assert()
        .success();

    cmd()
        .current_dir(tmp.path())
        .args(["start", &claimed_id, "--user", "alice"])
        .assert()
        .success();

    // ls --map lists all tickets on the map
    let ls_all = cmd()
        .current_dir(tmp.path())
        .args(["ls", "--map", &map_id])
        .output()
        .unwrap();
    assert!(ls_all.status.success());
    let all_stdout = String::from_utf8_lossy(&ls_all.stdout);
    assert!(all_stdout.contains(&blocker_id));
    assert!(all_stdout.contains(&dependent_id));
    assert!(all_stdout.contains(&claimed_id));
    assert!(!all_stdout.contains(&outsider_id));

    // While blocker open, dependent not on frontier; claimed/outsider excluded
    let blocked = cmd()
        .current_dir(tmp.path())
        .args(["ls", "--map", &map_id, "--frontier"])
        .output()
        .unwrap();
    assert!(blocked.status.success());
    let blocked_stdout = String::from_utf8_lossy(&blocked.stdout);
    assert!(blocked_stdout.contains(&blocker_id));
    assert!(!blocked_stdout.contains(&dependent_id));
    assert!(!blocked_stdout.contains(&claimed_id));
    assert!(!blocked_stdout.contains(&outsider_id));

    cmd()
        .current_dir(tmp.path())
        .args(["status", &blocker_id, "cancelled"])
        .assert()
        .success();

    let frontier = cmd()
        .current_dir(tmp.path())
        .args(["ls", "--map", &map_id, "--frontier"])
        .output()
        .unwrap();
    assert!(
        frontier.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&frontier.stderr)
    );
    let stdout = String::from_utf8_lossy(&frontier.stdout);
    assert!(stdout.contains(&dependent_id));
    assert!(!stdout.contains(&blocker_id));
    assert!(!stdout.contains(&claimed_id));
    assert!(!stdout.contains(&outsider_id));

    let later_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "Later open", "--map", &map_id, "--type", "task"])
        .output()
        .unwrap();
    let later_id = extract_id(&later_out.stdout);

    let ordered = cmd()
        .current_dir(tmp.path())
        .args(["ls", "--map", &map_id, "--frontier"])
        .output()
        .unwrap();
    assert!(ordered.status.success());
    let ordered_stdout = String::from_utf8_lossy(&ordered.stdout);
    let dep_pos = ordered_stdout.find(&dependent_id).unwrap();
    let later_pos = ordered_stdout.find(&later_id).unwrap();
    assert!(dep_pos < later_pos, "frontier should be sorted by position");
}

#[test]
fn test_archive_map_cascades_decision_tickets() {
    let tmp = setup_project();

    let map_out = cmd()
        .current_dir(tmp.path())
        .args(["map", "Archive me"])
        .output()
        .unwrap();
    let map_id = extract_id(&map_out.stdout);

    let d1_out = cmd()
        .current_dir(tmp.path())
        .args([
            "add",
            "Decision one",
            "--map",
            &map_id,
            "--type",
            "research",
        ])
        .output()
        .unwrap();
    let d1 = extract_id(&d1_out.stdout);

    let d2_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "Decision two", "--map", &map_id, "--type", "task"])
        .output()
        .unwrap();
    let d2 = extract_id(&d2_out.stdout);

    // Status change alone must not archive Decision tickets
    cmd()
        .current_dir(tmp.path())
        .args(["status", &map_id, "done"])
        .assert()
        .success();

    assert!(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", d1))
            .is_file()
    );
    assert!(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", d2))
            .is_file()
    );

    cmd()
        .current_dir(tmp.path())
        .args(["archive", &map_id])
        .assert()
        .success();

    assert!(
        tmp.path()
            .join(format!(".markplane/maps/archive/{}.md", map_id))
            .is_file()
    );
    assert!(
        tmp.path()
            .join(format!(".markplane/backlog/archive/{}.md", d1))
            .is_file()
    );
    assert!(
        tmp.path()
            .join(format!(".markplane/backlog/archive/{}.md", d2))
            .is_file()
    );
    assert!(
        !tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", d1))
            .exists()
    );
    assert!(
        !tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", d2))
            .exists()
    );

    let ls = cmd()
        .current_dir(tmp.path())
        .args(["ls", "--map", &map_id])
        .output()
        .unwrap();
    assert!(ls.status.success());
    let ls_stdout = String::from_utf8_lossy(&ls.stdout);
    assert!(!ls_stdout.contains(&d1));
    assert!(!ls_stdout.contains(&d2));
}

#[test]
fn test_link_spec_maintains_map_specs() {
    let tmp = setup_project();

    let map_out = cmd()
        .current_dir(tmp.path())
        .args(["map", "Spec map"])
        .output()
        .unwrap();
    let map_id = extract_id(&map_out.stdout);

    let epic_out = cmd()
        .current_dir(tmp.path())
        .args(["epic", "Published epic"])
        .output()
        .unwrap();
    let epic_id = extract_id(&epic_out.stdout);

    let plan_out = cmd()
        .current_dir(tmp.path())
        .args(["plan", "--title", "Published plan"])
        .output()
        .unwrap();
    let plan_id = extract_id(&plan_out.stdout);

    let task_out = cmd()
        .current_dir(tmp.path())
        .args(["add", "Not a spec"])
        .output()
        .unwrap();
    let task_id = extract_id(&task_out.stdout);

    cmd()
        .current_dir(tmp.path())
        .args(["link", &map_id, &epic_id, "-r", "spec"])
        .assert()
        .success();

    let map_body = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/maps/items/{}.md", map_id)),
    )
    .unwrap();
    assert!(map_body.contains(&format!("- {}", epic_id)) || map_body.contains(&epic_id));
    let map_fm = map_body.split("---").nth(1).unwrap_or("");
    assert!(map_fm.contains("specs:"));
    assert!(map_fm.contains(&epic_id));

    // EPIC gains no reverse field
    let epic_body = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/roadmap/items/{}.md", epic_id)),
    )
    .unwrap();
    let epic_fm = epic_body.split("---").nth(1).unwrap_or("");
    assert!(!epic_fm.contains(&map_id));
    assert!(!epic_fm.contains("specs:"));

    cmd()
        .current_dir(tmp.path())
        .args(["link", &map_id, &plan_id, "-r", "spec"])
        .assert()
        .success();

    let map_body2 = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/maps/items/{}.md", map_id)),
    )
    .unwrap();
    let map_fm2 = map_body2.split("---").nth(1).unwrap_or("");
    assert!(map_fm2.contains(&epic_id));
    assert!(map_fm2.contains(&plan_id));

    // PLAN gains no reverse field
    let plan_body = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/plans/items/{}.md", plan_id)),
    )
    .unwrap();
    let plan_fm = plan_body.split("---").nth(1).unwrap_or("");
    assert!(!plan_fm.contains(&map_id));

    // Reject TASK targets
    cmd()
        .current_dir(tmp.path())
        .args(["link", &map_id, &task_id, "-r", "spec"])
        .assert()
        .failure();

    // --remove clears the specs entry
    cmd()
        .current_dir(tmp.path())
        .args(["link", &map_id, &epic_id, "-r", "spec", "--remove"])
        .assert()
        .success();

    let map_body3 = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/maps/items/{}.md", map_id)),
    )
    .unwrap();
    let map_fm3 = map_body3.split("---").nth(1).unwrap_or("");
    assert!(!map_fm3.contains(&epic_id));
    assert!(map_fm3.contains(&plan_id));
}

// ── Resolve ──────────────────────────────────────────────────────────────

#[test]
fn test_resolve_writes_answer_marks_done_appends_decision() {
    let tmp = setup_project();
    let map_id = extract_id(
        &cmd()
            .current_dir(tmp.path())
            .args(["map", "Resolve map"])
            .output()
            .unwrap()
            .stdout,
    );
    // Keep a second open ticket so the map-cleared hint is not printed.
    let _other = extract_id(
        &cmd()
            .current_dir(tmp.path())
            .args(["add", "Still open", "--map", &map_id, "--type", "task"])
            .output()
            .unwrap()
            .stdout,
    );
    let task_id = extract_id(
        &cmd()
            .current_dir(tmp.path())
            .args([
                "add",
                "Pick the path",
                "--map",
                &map_id,
                "--type",
                "grilling",
            ])
            .output()
            .unwrap()
            .stdout,
    );

    let answer_path = tmp.path().join("answer.md");
    std::fs::write(&answer_path, "We take the left fork.\n").unwrap();

    let out = cmd()
        .current_dir(tmp.path())
        .args([
            "resolve",
            &task_id,
            "--answer-file",
            answer_path.to_str().unwrap(),
            "--gist",
            "left fork",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.to_lowercase().contains("no open") && !stdout.contains("status"),
        "should not hint map done while another ticket is open: {}",
        stdout
    );

    let task_file = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    assert!(
        task_file.contains("status: done"),
        "task should be done:\n{}",
        task_file
    );
    let task_body = task_file.splitn(3, "---").nth(2).unwrap_or("");
    assert!(
        task_body.contains("## Answer"),
        "expected ## Answer:\n{}",
        task_body
    );
    assert!(
        task_body.contains("We take the left fork."),
        "expected answer content:\n{}",
        task_body
    );

    let map_file = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/maps/items/{}.md", map_id)),
    )
    .unwrap();
    assert!(
        map_file.contains("status: active"),
        "map status must stay active:\n{}",
        map_file
    );
    let expected_line = format!("- Pick the path [[{}]]: left fork", task_id);
    assert!(
        map_file.contains(&expected_line),
        "expected decision line {:?}:\n{}",
        expected_line,
        map_file
    );
}

#[test]
fn test_resolve_last_open_prints_hint_map_status_unchanged() {
    let tmp = setup_project();
    let map_id = extract_id(
        &cmd()
            .current_dir(tmp.path())
            .args(["map", "Last ticket map"])
            .output()
            .unwrap()
            .stdout,
    );
    let task_id = extract_id(
        &cmd()
            .current_dir(tmp.path())
            .args([
                "add",
                "Final decision",
                "--map",
                &map_id,
                "--type",
                "research",
            ])
            .output()
            .unwrap()
            .stdout,
    );

    let answer_path = tmp.path().join("answer.md");
    std::fs::write(&answer_path, "Ship it.\n").unwrap();

    let out = cmd()
        .current_dir(tmp.path())
        .args([
            "resolve",
            &task_id,
            "--answer-file",
            answer_path.to_str().unwrap(),
            "--gist",
            "ship it",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(&map_id),
        "hint should mention map id: {}",
        stdout
    );
    assert!(
        stdout.to_lowercase().contains("no open") || stdout.contains("status"),
        "hint should mention no open tickets / status: {}",
        stdout
    );
    assert!(
        stdout.contains("done"),
        "hint should mention marking done: {}",
        stdout
    );

    let map_file = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/maps/items/{}.md", map_id)),
    )
    .unwrap();
    assert!(
        map_file.contains("status: active"),
        "map must remain active (not auto-done):\n{}",
        map_file
    );
}

#[test]
fn test_resolve_rejects_non_decision_ticket() {
    let tmp = setup_project();
    let task_id = extract_id(
        &cmd()
            .current_dir(tmp.path())
            .args(["add", "Regular implementation task"])
            .output()
            .unwrap()
            .stdout,
    );

    let answer_path = tmp.path().join("answer.md");
    std::fs::write(&answer_path, "Nope.\n").unwrap();

    let out = cmd()
        .current_dir(tmp.path())
        .args([
            "resolve",
            &task_id,
            "--answer-file",
            answer_path.to_str().unwrap(),
            "--gist",
            "nope",
        ])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "resolve on non-Decision ticket should fail"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.to_lowercase().contains("decision") || stderr.contains("map"),
        "stderr should mention Decision ticket / map: {}",
        stderr
    );

    let task_file = std::fs::read_to_string(
        tmp.path()
            .join(format!(".markplane/backlog/items/{}.md", task_id)),
    )
    .unwrap();
    assert!(
        !task_file.contains("status: done"),
        "non-decision resolve must not mark done:\n{}",
        task_file
    );
    assert!(
        !task_file.contains("## Answer"),
        "non-decision resolve must not write Answer:\n{}",
        task_file
    );
}

#[test]
fn test_claude_md_includes_maps_paths() {
    let tmp = setup_project();
    // Seed a local CLAUDE.md so walk-up does not pick a parent repo file.
    std::fs::write(tmp.path().join("CLAUDE.md"), "# Fixture\n").unwrap();
    cmd()
        .current_dir(tmp.path())
        .args(["claude-md"])
        .assert()
        .success();

    let content = std::fs::read_to_string(tmp.path().join("CLAUDE.md")).unwrap();
    assert!(content.contains("## Project Management"));
    assert!(content.contains(".markplane/maps/INDEX.md"));
    assert!(content.contains(".context/maps.md"));
}

// ── Serve multi-project registry ─────────────────────────────────────────

#[test]
fn test_serve_registers_project_and_serves_prefixed_api() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::process::{Command as StdCommand, Stdio};
    use std::time::{Duration, Instant};

    let tmp = setup_project();
    let xdg = tempfile::TempDir::new().unwrap();

    // Pick a free port.
    let port = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };

    let bin = assert_cmd::cargo::cargo_bin!("markplane");
    let mut child = StdCommand::new(bin)
        .current_dir(tmp.path())
        .env("XDG_CONFIG_HOME", xdg.path())
        .args(["serve", "--dev", "--port", &port.to_string()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn serve");

    let registry = xdg.path().join("markplane/projects.json");
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if registry.is_file() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        registry.is_file(),
        "projects.json was not created under XDG_CONFIG_HOME"
    );

    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&registry).unwrap()).unwrap();
    let projects = json["projects"].as_array().expect("projects array");
    assert_eq!(projects.len(), 1);
    let id = projects[0]["id"].as_str().unwrap();

    // Wait until the server accepts connections.
    let mut ready = false;
    while Instant::now() < deadline {
        if let Ok(mut stream) = std::net::TcpStream::connect(("127.0.0.1", port)) {
            let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
            let req = format!(
                "GET /api/p/{id}/summary HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
            );
            if stream.write_all(req.as_bytes()).is_ok() {
                let mut buf = String::new();
                let _ = stream.read_to_string(&mut buf);
                if buf.contains("200 OK") && buf.contains("Test Project") {
                    ready = true;
                    break;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let _ = child.kill();
    let _ = child.wait();
    assert!(ready, "prefixed /api/p/<id>/summary did not become ready");
}

// ── Serve daemon (start/stop/status/logs) ────────────────────────────────

#[cfg(unix)]
fn serve_cmd(xdg: &TempDir) -> Command {
    let mut c = cmd();
    c.env("XDG_CONFIG_HOME", xdg.path());
    c
}

#[cfg(unix)]
#[test]
fn test_serve_start_status_stop() {
    let xdg = TempDir::new().unwrap();
    let port = free_port();
    let port_s = port.to_string();

    serve_cmd(&xdg)
        .args(["serve", "start", "--dev", "--port", &port_s])
        .assert()
        .success()
        .stdout(predicate::str::contains("http://localhost:"));

    serve_cmd(&xdg)
        .args(["serve", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("running").or(predicate::str::contains("Running")))
        .stdout(predicate::str::contains(&port_s))
        .stdout(predicate::str::contains("http://localhost:"));

    serve_cmd(&xdg)
        .args(["serve", "stop"])
        .assert()
        .success();

    serve_cmd(&xdg)
        .args(["serve", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("not running").or(predicate::str::contains("Not running")));
}

#[cfg(unix)]
#[test]
fn test_serve_start_when_already_running() {
    let xdg = TempDir::new().unwrap();
    let port = free_port();
    let port_s = port.to_string();

    serve_cmd(&xdg)
        .args(["serve", "start", "--dev", "--port", &port_s])
        .assert()
        .success();

    serve_cmd(&xdg)
        .args(["serve", "start", "--dev", "--port", &port_s])
        .assert()
        .success()
        .stdout(predicate::str::contains("Already running"));

    serve_cmd(&xdg).args(["serve", "stop"]).assert().success();
}

#[cfg(unix)]
#[test]
fn test_serve_start_ignores_stale_pid_file() {
    let xdg = TempDir::new().unwrap();
    let port = free_port();
    let port_s = port.to_string();
    let config = xdg.path().join("markplane");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(
        config.join("serve.pid"),
        r#"{"pid":999999,"port":1,"url":"http://localhost:1","version":"0","startedAt":"2020-01-01T00:00:00Z"}"#,
    )
    .unwrap();

    serve_cmd(&xdg)
        .args(["serve", "start", "--dev", "--port", &port_s])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started"));

    serve_cmd(&xdg)
        .args(["serve", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Running"))
        .stdout(predicate::str::contains(&port_s));

    // SIGKILL leaves the pid file behind; lock is released.
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(config.join("serve.pid")).unwrap()).unwrap();
    let pid = json["pid"].as_u64().unwrap();
    std::process::Command::new("kill")
        .args(["-9", &pid.to_string()])
        .status()
        .expect("kill -9");
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Stale file must not block a new start.
    let port2 = free_port();
    serve_cmd(&xdg)
        .args(["serve", "start", "--dev", "--port", &port2.to_string()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started"));

    serve_cmd(&xdg).args(["serve", "stop"]).assert().success();
}

#[cfg(unix)]
#[test]
fn test_serve_start_port_in_use() {
    let xdg = TempDir::new().unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let port_s = port.to_string();

    serve_cmd(&xdg)
        .args(["serve", "start", "--dev", "--port", &port_s])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("--port")
                .or(predicate::str::contains("lsof"))
                .or(predicate::str::contains("already in use"))
                .or(predicate::str::contains("failed to start")),
        );

    drop(listener);
}

#[cfg(unix)]
#[test]
fn test_serve_foreground_reuses_running_server() {
    let xdg = TempDir::new().unwrap();
    let port = free_port();
    let port_s = port.to_string();

    // Background server (no project in cwd of daemon).
    serve_cmd(&xdg)
        .args(["serve", "start", "--dev", "--port", &port_s])
        .assert()
        .success();

    let proj_a = setup_project();
    let proj_b = setup_project();

    // First foreground serve registers A and exits.
    serve_cmd(&xdg)
        .current_dir(proj_a.path())
        .args(["serve", "--dev", "--port", &port_s])
        .assert()
        .success()
        .stdout(predicate::str::contains("Registered project"));

    // Second project from another directory.
    serve_cmd(&xdg)
        .current_dir(proj_b.path())
        .args(["serve", "--dev", "--port", &port_s])
        .assert()
        .success()
        .stdout(predicate::str::contains("Registered project"));

    let registry = xdg.path().join("markplane/projects.json");
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&registry).unwrap()).unwrap();
    let projects = json["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 2, "both projects should be registered");

    serve_cmd(&xdg).args(["serve", "stop"]).assert().success();
}

#[cfg(unix)]
#[test]
fn test_serve_logs_shows_output() {
    let xdg = TempDir::new().unwrap();
    let port = free_port();
    let port_s = port.to_string();

    serve_cmd(&xdg)
        .args(["serve", "start", "--dev", "--port", &port_s])
        .assert()
        .success();

    serve_cmd(&xdg)
        .args(["serve", "logs", "-n", "50"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Markplane web UI starting"));

    serve_cmd(&xdg).args(["serve", "stop"]).assert().success();
}
