use markplane_core::{Priority, Project};

use super::{optional_body_file, parse_comma_list};

pub fn run(
    title: String,
    priority: String,
    tags: Option<String>,
    template: Option<String>,
    body_file: Option<String>,
) -> anyhow::Result<()> {
    let project = Project::from_current_dir()?;
    let priority: Priority = priority.parse()?;
    let tags = tags.map(|t| parse_comma_list(&t)).unwrap_or_default();
    let body = optional_body_file(&body_file)?;
    let epic = project.create_epic(&title, priority, tags, template.as_deref(), body.as_deref())?;
    println!("Created {} — {}", epic.id, epic.title);
    Ok(())
}
