use markplane_core::{Effort, LinkAction, LinkRelation, Priority, Project};

use super::{parse_comma_list, read_body_file};

#[allow(clippy::too_many_arguments)]
pub fn run(
    title: String,
    item_type: Option<String>,
    priority: String,
    effort: String,
    epic: Option<String>,
    plan: Option<String>,
    map: Option<String>,
    tags: Option<String>,
    template: Option<String>,
    body_file: Option<String>,
) -> anyhow::Result<()> {
    let project = Project::from_current_dir()?;
    let config = project.load_config()?;
    // Decision tickets default to wayfinder type "task" when --type is omitted.
    let item_type = item_type.as_deref().unwrap_or(if map.is_some() {
        "task"
    } else {
        config.default_task_type()
    });

    let priority: Priority = priority.parse()?;
    let effort: Effort = effort.parse()?;
    let tags = tags.map(|t| parse_comma_list(&t)).unwrap_or_default();
    let body = match body_file {
        Some(path) => Some(read_body_file(&path)?),
        None => None,
    };

    // Core rejects map+epic; reject map+plan before create so we fail cleanly.
    if map.is_some() && plan.is_some() {
        anyhow::bail!("A Decision ticket (map) cannot also have an epic or plan");
    }

    let item = project.create_task(
        &title,
        item_type,
        priority,
        effort,
        epic,
        map,
        tags,
        template.as_deref(),
        body.as_deref(),
    )?;

    if let Some(plan_id) = plan {
        project.link_items(&item.id, &plan_id, LinkRelation::Plan, LinkAction::Add)?;
    }

    println!("Created {} — {}", item.id, item.title);

    Ok(())
}
