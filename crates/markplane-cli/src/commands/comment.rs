use std::env;

use markplane_core::Project;

use super::read_body_file;

pub fn run(id: String, body_file: String) -> anyhow::Result<()> {
    let project = Project::from_current_dir()?;
    let text = read_body_file(&body_file)?;
    let author = env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_else(|_| "me".to_string());

    project.add_comment(&id, &author, &text)?;
    println!("Commented on {}", id);
    Ok(())
}
