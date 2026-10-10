use markplane_core::{Project, default_comment_author};

use super::read_body_file;

pub fn run(id: String, body_file: String) -> anyhow::Result<()> {
    let project = Project::from_current_dir()?;
    let text = read_body_file(&body_file)?;
    let author = default_comment_author();

    project.add_comment(&id, &author, &text)?;
    println!("Commented on {}", id);
    Ok(())
}
