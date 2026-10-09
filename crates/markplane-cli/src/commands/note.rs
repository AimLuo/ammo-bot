use markplane_core::Project;

use super::{optional_body_file, parse_comma_list};

pub fn run(
    title: String,
    note_type: Option<String>,
    tags: Option<String>,
    template: Option<String>,
    body_file: Option<String>,
) -> anyhow::Result<()> {
    let project = Project::from_current_dir()?;
    let config = project.load_config()?;
    let note_type = note_type.as_deref().unwrap_or(config.default_note_type());
    let tags = tags.map(|t| parse_comma_list(&t)).unwrap_or_default();
    let body = optional_body_file(&body_file)?;
    let note = project.create_note(
        &title,
        note_type,
        tags,
        template.as_deref(),
        body.as_deref(),
    )?;
    println!("Created {} — {}", note.id, note.title);
    Ok(())
}
