use markplane_core::{
    IdPrefix, LinkAction, LinkRelation, MarkplaneDocument, Project, Task, parse_id,
};

use super::read_body_file;

pub fn run(
    id: String,
    title: Option<String>,
    template: Option<String>,
    body_file: Option<String>,
) -> anyhow::Result<()> {
    let project = Project::from_current_dir()?;
    let (prefix, _) = parse_id(&id)?;

    if prefix != IdPrefix::Task {
        anyhow::bail!("Can only create plans for tasks. Got: {}", id);
    }

    let doc: MarkplaneDocument<Task> = project.read_item(&id)?;
    let plan_title =
        title.unwrap_or_else(|| format!("Implementation plan for {}", doc.frontmatter.title));
    let body = match body_file {
        Some(path) => Some(read_body_file(&path)?),
        None => None,
    };

    let plan = project.create_plan(&plan_title, vec![], template.as_deref(), body.as_deref())?;

    // Link the plan to the task via the centralized link system
    project.link_items(&id, &plan.id, LinkRelation::Plan, LinkAction::Add)?;

    println!("Created {} — {}", plan.id, plan.title);
    println!("Linked to {}", id);

    Ok(())
}
