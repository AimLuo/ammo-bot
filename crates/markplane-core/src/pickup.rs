//! Dispatcher pickup: filter and sort Implementation tickets ready for an agent.

use std::collections::HashSet;

use chrono::NaiveDate;

use crate::models::{MarkplaneDocument, Priority, StatusCategory, Task, TaskWorkflow};
use crate::project::find_blocked_items;
use crate::query::priority_rank;

/// Tag that marks a ticket as released for the Dispatcher.
pub const READY_FOR_AGENT_TAG: &str = "ready-for-agent";

/// Minimal fields used when sorting pickup candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickupSortKey {
    pub id: String,
    pub priority: Priority,
    pub position: Option<String>,
    pub created: NaiveDate,
}

impl PickupSortKey {
    pub fn from_task(task: &Task) -> Self {
        Self {
            id: task.id.clone(),
            priority: task.priority.clone(),
            position: task.position.clone(),
            created: task.created,
        }
    }
}

/// Compare two candidates: priority → position → created → ID.
pub fn cmp_pickup_keys(a: &PickupSortKey, b: &PickupSortKey) -> std::cmp::Ordering {
    priority_rank(&a.priority)
        .cmp(&priority_rank(&b.priority))
        .then_with(|| match (&a.position, &b.position) {
            (Some(ap), Some(bp)) => ap.cmp(bp),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        })
        .then_with(|| a.created.cmp(&b.created))
        .then_with(|| a.id.cmp(&b.id))
}

/// Whether a task is an Implementation ticket (not a Decision ticket).
pub fn is_implementation_ticket(task: &Task) -> bool {
    task.map.is_none()
}

/// Filter Implementation tickets that are pickup candidates.
///
/// Candidates must be:
/// - Implementation tickets (no `map`)
/// - status in the planned category
/// - tagged `ready-for-agent`
/// - not blocked (unresolved `depends_on`)
///
/// Results are sorted by priority → position → created → ID.
pub fn filter_and_sort_pickup_candidates<'a>(
    items: &'a [MarkplaneDocument<Task>],
    workflow: &TaskWorkflow,
) -> Vec<&'a MarkplaneDocument<Task>> {
    let planned: HashSet<&str> = workflow
        .statuses_in(StatusCategory::Planned)
        .iter()
        .map(|s| s.as_str())
        .collect();
    let blocked_ids: HashSet<&str> = find_blocked_items(items, workflow)
        .into_iter()
        .map(|d| d.frontmatter.id.as_str())
        .collect();

    let mut candidates: Vec<_> = items
        .iter()
        .filter(|doc| {
            let t = &doc.frontmatter;
            is_implementation_ticket(t)
                && planned.contains(t.status.as_str())
                && t.tags.iter().any(|tag| tag == READY_FOR_AGENT_TAG)
                && !blocked_ids.contains(t.id.as_str())
        })
        .collect();

    candidates.sort_by(|a, b| {
        cmp_pickup_keys(
            &PickupSortKey::from_task(&a.frontmatter),
            &PickupSortKey::from_task(&b.frontmatter),
        )
    });
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Effort, MarkplaneDocument, default_task_workflow};
    use chrono::NaiveDate;

    fn task(
        id: &str,
        status: &str,
        priority: Priority,
        tags: &[&str],
        position: Option<&str>,
        created: &str,
        depends_on: &[&str],
        map: Option<&str>,
    ) -> MarkplaneDocument<Task> {
        MarkplaneDocument {
            frontmatter: Task {
                id: id.into(),
                title: id.into(),
                status: status.into(),
                priority,
                item_type: "feature".into(),
                effort: Effort::Medium,
                epic: None,
                plan: None,
                map: map.map(str::to_string),
                depends_on: depends_on.iter().map(|s| (*s).to_string()).collect(),
                blocks: vec![],
                related: vec![],
                assignee: None,
                tags: tags.iter().map(|s| (*s).to_string()).collect(),
                position: position.map(str::to_string),
                created: NaiveDate::parse_from_str(created, "%Y-%m-%d").unwrap(),
                updated: NaiveDate::parse_from_str(created, "%Y-%m-%d").unwrap(),
            },
            body: String::new(),
        }
    }

    #[test]
    fn filters_planned_ready_unblocked_implementation_only() {
        let workflow = default_task_workflow();
        let items = vec![
            task(
                "TASK-aaaa1",
                "planned",
                Priority::Medium,
                &["ready-for-agent"],
                None,
                "2026-01-01",
                &[],
                None,
            ),
            task(
                "TASK-bbbb2",
                "in-progress",
                Priority::High,
                &["ready-for-agent"],
                None,
                "2026-01-01",
                &[],
                None,
            ),
            task(
                "TASK-cccc3",
                "planned",
                Priority::High,
                &[],
                None,
                "2026-01-01",
                &[],
                None,
            ),
            task(
                "TASK-dddd4",
                "planned",
                Priority::High,
                &["ready-for-agent"],
                None,
                "2026-01-01",
                &["TASK-zzzz9"],
                None,
            ),
            // blocker still open → dddd4 blocked; also include blocker itself
            task(
                "TASK-zzzz9",
                "planned",
                Priority::Low,
                &[],
                None,
                "2026-01-01",
                &[],
                None,
            ),
            task(
                "TASK-eeee5",
                "planned",
                Priority::Critical,
                &["ready-for-agent"],
                None,
                "2026-01-01",
                &[],
                Some("MAP-xxxx1"),
            ),
        ];

        let ids: Vec<_> = filter_and_sort_pickup_candidates(&items, &workflow)
            .iter()
            .map(|d| d.frontmatter.id.as_str())
            .collect();
        assert_eq!(ids, vec!["TASK-aaaa1"]);
    }

    #[test]
    fn sorts_by_priority_then_position_then_created_then_id() {
        let workflow = default_task_workflow();
        let items = vec![
            task(
                "TASK-id002",
                "planned",
                Priority::High,
                &["ready-for-agent"],
                Some("b"),
                "2026-01-02",
                &[],
                None,
            ),
            task(
                "TASK-id001",
                "planned",
                Priority::High,
                &["ready-for-agent"],
                Some("b"),
                "2026-01-02",
                &[],
                None,
            ),
            task(
                "TASK-id003",
                "planned",
                Priority::High,
                &["ready-for-agent"],
                Some("a"),
                "2026-01-03",
                &[],
                None,
            ),
            task(
                "TASK-id004",
                "planned",
                Priority::Critical,
                &["ready-for-agent"],
                Some("z"),
                "2026-01-10",
                &[],
                None,
            ),
            task(
                "TASK-id005",
                "planned",
                Priority::High,
                &["ready-for-agent"],
                Some("b"),
                "2026-01-01",
                &[],
                None,
            ),
        ];

        let ids: Vec<_> = filter_and_sort_pickup_candidates(&items, &workflow)
            .iter()
            .map(|d| d.frontmatter.id.as_str())
            .collect();
        assert_eq!(
            ids,
            vec![
                "TASK-id004", // critical first
                "TASK-id003", // high, position a
                "TASK-id005", // high, position b, created earliest
                "TASK-id001", // high, position b, same created, lower id
                "TASK-id002",
            ]
        );
    }

    #[test]
    fn closed_dependency_unblocks_candidate() {
        let workflow = default_task_workflow();
        let items = vec![
            task(
                "TASK-dep01",
                "done",
                Priority::Medium,
                &[],
                None,
                "2026-01-01",
                &[],
                None,
            ),
            task(
                "TASK-cand1",
                "planned",
                Priority::Medium,
                &["ready-for-agent"],
                None,
                "2026-01-02",
                &["TASK-dep01"],
                None,
            ),
        ];
        let ids: Vec<_> = filter_and_sort_pickup_candidates(&items, &workflow)
            .iter()
            .map(|d| d.frontmatter.id.as_str())
            .collect();
        assert_eq!(ids, vec!["TASK-cand1"]);
    }
}
