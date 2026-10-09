use markplane_core::{
    IdPrefix, LinkAction, LinkRelation, MarkplaneDocument, Project, Task, parse_id,
};

use super::{parse_comma_list, read_body_file};

pub fn run(
    id: Option<String>,
    title: Option<String>,
    tags: Option<String>,
    template: Option<String>,
    body_file: Option<String>,
) -> anyhow::Result<()> {
    let project = Project::from_current_dir()?;
    let tags = tags.map(|t| parse_comma_list(&t)).unwrap_or_default();
    let body = match body_file {
        Some(path) => Some(read_body_file(&path)?),
        None => None,
    };

    match id {
        Some(task_id) => {
            let (prefix, _) = parse_id(&task_id)?;
            if prefix != IdPrefix::Task {
                anyhow::bail!("Can only create plans for tasks. Got: {}", task_id);
            }

            let doc: MarkplaneDocument<Task> = project.read_item(&task_id)?;
            let plan_title = title
                .unwrap_or_else(|| format!("Implementation plan for {}", doc.frontmatter.title));

            let plan = project.create_plan(
                &plan_title,
                vec![],
                tags,
                template.as_deref(),
                body.as_deref(),
            )?;

            project.link_items(&task_id, &plan.id, LinkRelation::Plan, LinkAction::Add)?;

            println!("Created {} — {}", plan.id, plan.title);
            println!("Linked to {}", task_id);
        }
        None => {
            let plan_title = title.ok_or_else(|| {
                anyhow::anyhow!("--title is required when creating a standalone plan")
            })?;

            let plan = project.create_plan(
                &plan_title,
                vec![],
                tags,
                template.as_deref(),
                body.as_deref(),
            )?;

            println!("Created {} — {}", plan.id, plan.title);
        }
    }

    Ok(())
}
