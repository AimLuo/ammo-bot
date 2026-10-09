pub mod context;
pub mod error;
pub mod frontmatter;
pub mod index;
pub mod links;
pub mod manifest;
pub mod models;
pub mod position;
pub mod project;
pub mod query;
pub mod references;
pub mod sections;
pub mod templates;

pub use error::{MarkplaneError, Result};
pub use frontmatter::{parse_frontmatter, parse_frontmatter_raw, write_frontmatter};
pub use links::{LinkAction, LinkRelation};
pub use models::{
    Config, DECISION_TICKET_DEFAULT_PRIORITY, DECISION_TICKET_TYPES, Effort, Epic, EpicStatus,
    IdPrefix, Map, MapStatus, MarkplaneDocument, Note, NoteStatus, Plan, PlanStatus, Priority,
    StatusCategory, Task, TaskWorkflow, WorkflowConfig, default_note_types, default_task_types,
    default_task_workflow, generate_random_id, parse_id,
};
pub use project::{
    EpicUpdate, MapUpdate, MoveDirective, NoteUpdate, Patch, PlanUpdate, Project, ResolveOutcome,
    TaskUpdate, UpdateFields, append_comment_to_body, apply_tag_changes, compose_body_after_title,
    find_blocked_items, find_frontier_items,
};
pub use query::{QueryFilter, ScanScope};
pub use references::{
    AsymmetricLink, BrokenReference, DependencyCycle, build_reference_graph, detect_cycles,
    extract_references, find_orphans, validate_reciprocal_links, validate_references,
    validate_task_statuses,
};
pub use sections::{
    REQUIRED_MAP_SECTIONS, append_line_to_section, missing_map_sections, set_answer_section,
    split_markdown_sections,
};
pub use templates::render_template;
