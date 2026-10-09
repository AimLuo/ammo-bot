use markplane_core::{Priority, Project};

use super::read_body_file;

pub fn run(title: String, priority: String, body_file: Option<String>) -> anyhow::Result<()> {
    let project = Project::from_current_dir()?;
    let priority: Priority = priority.parse()?;
    let body = match body_file {
        Some(path) => Some(read_body_file(&path)?),
        None => None,
    };
    let epic = project.create_epic(&title, priority, None, body.as_deref())?;
    println!("Created {} — {}", epic.id, epic.title);
    Ok(())
}
