use markplane_core::Project;

use super::{optional_body_file, parse_comma_list};

pub fn run(
    title: String,
    tags: Option<String>,
    template: Option<String>,
    body_file: Option<String>,
) -> anyhow::Result<()> {
    let project = Project::from_current_dir()?;
    let tags = tags.map(|t| parse_comma_list(&t)).unwrap_or_default();
    let body = optional_body_file(&body_file)?;
    let map = project.create_map(&title, tags, template.as_deref(), body.as_deref())?;
    println!("Created {} — {}", map.id, map.title);
    Ok(())
}
