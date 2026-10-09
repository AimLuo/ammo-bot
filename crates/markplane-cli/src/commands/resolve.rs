use markplane_core::Project;

use super::read_body_file;

pub fn run(id: String, answer_file: String, gist: String) -> anyhow::Result<()> {
    let project = Project::from_current_dir()?;
    let answer = read_body_file(&answer_file)?;

    let outcome = project.resolve_decision(&id, &answer, &gist)?;
    println!("{} → done", id);
    if outcome.map_has_no_open_tickets {
        println!(
            "Map {} has no open Decision tickets left — ask whether to mark it done: markplane status {} done",
            outcome.map_id, outcome.map_id
        );
    }
    Ok(())
}
