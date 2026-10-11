mod add;
mod archive;
mod certs;
mod check;
mod claude_md;
mod comment;
mod context;
mod dashboard;
mod done;
mod edit;
mod epic;
mod formatting;
mod graph;
mod init;
mod link;
mod ls;
mod map;
mod mcp;
mod metrics;
mod note;
mod plan;
mod promote;
mod resolve;
mod serve;
mod serve_daemon;
mod serve_dispatcher;
mod serve_dispatcher_continue;
mod serve_dispatcher_engine;
mod serve_dispatcher_handoff;
mod serve_dispatcher_lock;
mod serve_dispatcher_prompts;
mod serve_dispatcher_resume;
mod serve_git;
mod serve_registry;
mod show;
mod stale;
mod start;
mod status;
mod sync;
mod unarchive;
mod update;

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new .markplane/ structure
    Init {
        /// Project name (defaults to current directory name)
        #[arg(long)]
        name: Option<String>,
        /// Project description
        #[arg(long, default_value = "")]
        description: String,
        /// Skip starter content (create empty project)
        #[arg(long)]
        empty: bool,
    },

    /// Create a new task
    Add {
        /// Title of the item
        title: String,
        /// Task type (configurable in config.yaml, default: first in task_types list).
        /// With --map: research, prototype, grilling, or task (built-in; skips config).
        #[arg(long)]
        r#type: Option<String>,
        /// Priority level
        #[arg(long, default_value = "medium")]
        priority: String,
        /// Effort estimate
        #[arg(long, default_value = "medium")]
        effort: String,
        /// Parent epic ID
        #[arg(long)]
        epic: Option<String>,
        /// Parent plan ID
        #[arg(long)]
        plan: Option<String>,
        /// Parent Map ID (creates a Decision ticket)
        #[arg(long)]
        map: Option<String>,
        /// Comma-separated tags
        #[arg(long)]
        tags: Option<String>,
        /// Template name override (e.g. "bug", "default")
        #[arg(long)]
        template: Option<String>,
        /// Read body from file (or "-" for stdin); placed after the title heading
        #[arg(long, value_name = "PATH")]
        body_file: Option<String>,
    },

    /// Show details of an item
    Show {
        /// Item ID (e.g. TASK-042)
        id: String,
    },

    /// Append a comment under ## Comments
    Comment {
        /// Item ID (e.g. TASK-042, EPIC-001, PLAN-003, NOTE-007)
        id: String,
        /// Read comment body from file (or "-" for stdin)
        #[arg(long, value_name = "PATH")]
        body_file: String,
    },

    /// Resolve a Decision ticket (write ## Answer, mark done, append Map decision)
    Resolve {
        /// Decision ticket ID (TASK with a map)
        id: String,
        /// Read answer body from file (or "-" for stdin)
        #[arg(long, value_name = "PATH")]
        answer_file: String,
        /// One-line gist for the Map Decisions so far entry
        #[arg(long)]
        gist: String,
    },

    /// Open an item in $EDITOR
    Edit {
        /// Item ID (e.g. TASK-042)
        id: String,
    },

    /// List items
    Ls {
        /// Item kind to list: tasks (default), epics, plans, notes, maps
        #[command(subcommand)]
        kind: Option<LsKind>,

        /// Filter by status (comma-separated)
        #[arg(long)]
        status: Option<String>,
        /// Filter by priority (comma-separated)
        #[arg(long)]
        priority: Option<String>,
        /// Filter by epic ID
        #[arg(long)]
        epic: Option<String>,
        /// Filter by plan ID
        #[arg(long)]
        plan: Option<String>,
        /// Filter by map ID
        #[arg(long)]
        map: Option<String>,
        /// Filter by tags (comma-separated)
        #[arg(long)]
        tags: Option<String>,
        /// Filter by assignee
        #[arg(long)]
        assignee: Option<String>,
        /// Filter by item type (comma-separated)
        #[arg(long)]
        r#type: Option<String>,
        /// List archived items instead of active ones
        #[arg(long)]
        archived: bool,
        /// Show only frontier tasks (open, unblocked, unassigned), sorted by position
        #[arg(long)]
        frontier: bool,
    },

    /// Update the status of an item
    Status {
        /// Item ID (e.g. TASK-042)
        id: String,
        /// New status value
        new_status: String,
    },

    /// Regenerate INDEX.md files and .context/ summaries
    Sync {
        /// Also normalize position keys (rewrites source files)
        #[arg(long)]
        normalize: bool,
    },

    /// Start working on an item (sets status to in-progress and assigns to you)
    Start {
        /// Item ID (e.g. TASK-042)
        id: String,
        /// Assignee name (defaults to $USER or "me")
        #[arg(long)]
        user: Option<String>,
    },

    /// Mark an item as done
    Done {
        /// Item ID (e.g. TASK-042)
        id: String,
    },

    /// Promote a note to a task
    Promote {
        /// Note ID (e.g. NOTE-007)
        id: String,
        /// Priority for the new task
        #[arg(long, default_value = "medium")]
        priority: String,
        /// Effort estimate for the new task
        #[arg(long, default_value = "medium")]
        effort: String,
    },

    /// Create a plan (standalone, or linked to a task)
    Plan {
        /// Task ID to link (omit for a standalone plan)
        id: Option<String>,
        /// Plan title (required for standalone; defaults to "Implementation plan for <item title>" when linked)
        #[arg(long)]
        title: Option<String>,
        /// Comma-separated tags
        #[arg(long)]
        tags: Option<String>,
        /// Template name override (e.g. "refactor", "implementation", "spec")
        #[arg(long)]
        template: Option<String>,
        /// Read body from file (or "-" for stdin); placed after the title heading
        #[arg(long, value_name = "PATH")]
        body_file: Option<String>,
    },

    /// Create a new epic
    Epic {
        /// Epic title
        title: String,
        /// Priority level
        #[arg(long, default_value = "medium")]
        priority: String,
        /// Comma-separated tags
        #[arg(long)]
        tags: Option<String>,
        /// Template name override (e.g. "spec")
        #[arg(long)]
        template: Option<String>,
        /// Read body from file (or "-" for stdin); placed after the title heading
        #[arg(long, value_name = "PATH")]
        body_file: Option<String>,
    },

    /// Create a new note
    Note {
        /// Note title
        title: String,
        /// Note type (configurable in config.yaml, default: first in note_types list)
        #[arg(long)]
        r#type: Option<String>,
        /// Comma-separated tags
        #[arg(long)]
        tags: Option<String>,
        /// Template name override (e.g. "research", "analysis")
        #[arg(long)]
        template: Option<String>,
        /// Read body from file (or "-" for stdin); placed after the title heading
        #[arg(long, value_name = "PATH")]
        body_file: Option<String>,
    },

    /// Create a new map
    Map {
        /// Map title
        title: String,
        /// Comma-separated tags
        #[arg(long)]
        tags: Option<String>,
        /// Template name override
        #[arg(long)]
        template: Option<String>,
        /// Read body from file (or "-" for stdin); placed after the title heading
        #[arg(long, value_name = "PATH")]
        body_file: Option<String>,
    },

    /// Update properties on any item
    Update {
        /// Item ID (e.g. TASK-042, EPIC-001, PLAN-003, NOTE-007, MAP-x)
        id: String,
        /// New title
        #[arg(long)]
        title: Option<String>,
        /// New status
        #[arg(long)]
        status: Option<String>,
        /// New priority (tasks and epics only)
        #[arg(long)]
        priority: Option<String>,
        /// New effort size (tasks only)
        #[arg(long)]
        effort: Option<String>,
        /// New item type (tasks only: feature, bug, enhancement, chore, research, spike)
        #[arg(long)]
        r#type: Option<String>,
        /// Set assignee (tasks only)
        #[arg(long)]
        assignee: Option<String>,
        /// Clear assignee
        #[arg(long)]
        clear_assignee: bool,
        /// Set position key (tasks only)
        #[arg(long)]
        position: Option<String>,
        /// Clear position
        #[arg(long)]
        clear_position: bool,
        /// Comma-separated tags to add
        #[arg(long)]
        add_tag: Option<String>,
        /// Comma-separated tags to remove
        #[arg(long)]
        remove_tag: Option<String>,
        /// Set started date (epics only, YYYY-MM-DD)
        #[arg(long)]
        started: Option<String>,
        /// Clear started date
        #[arg(long)]
        clear_started: bool,
        /// Set target date (epics only, YYYY-MM-DD)
        #[arg(long)]
        target: Option<String>,
        /// Clear target date
        #[arg(long)]
        clear_target: bool,
        /// New note type (notes only, configurable in config.yaml)
        #[arg(long)]
        note_type: Option<String>,
    },

    /// Link two items with a relationship
    Link {
        /// Source item ID
        from: String,
        /// Target item ID
        to: String,
        /// Relationship type: blocks, depends-on, epic, plan, implements, map, spec, related
        #[arg(long, short = 'r')]
        relation: String,
        /// Remove the link instead of adding it
        #[arg(long)]
        remove: bool,
    },

    /// Validate cross-references and find broken links
    Check {
        /// Also show orphan items (no incoming references)
        #[arg(long)]
        orphans: bool,
        /// Repair asymmetric reciprocal links
        #[arg(long)]
        fix: bool,
    },

    /// List items not updated in N days
    Stale {
        /// Number of days threshold
        #[arg(long, default_value = "30")]
        days: u32,
    },

    /// Archive an item or all completed items
    Archive {
        /// Item ID to archive (e.g. TASK-042)
        id: Option<String>,
        /// Archive all completed items across all types
        #[arg(long)]
        all_done: bool,
        /// Preview what would be archived
        #[arg(long)]
        dry_run: bool,
    },

    /// Restore an archived item back to active
    Unarchive {
        /// Item ID to restore (e.g. TASK-042)
        id: String,
    },

    /// Regenerate .context/ files
    Context {
        /// Focus on a specific context view (active-work, blocked, metrics, summary)
        #[arg(long)]
        focus: Option<String>,
    },

    /// Show project metrics and statistics
    Metrics,

    /// Show dependency graph for an item
    Graph {
        /// Item ID (e.g. TASK-042)
        id: String,
        /// Maximum depth to traverse
        #[arg(long, default_value = "3")]
        depth: u32,
    },

    /// Output CLAUDE.md integration snippet
    ClaudeMd,

    /// Show project dashboard overview
    Dashboard,

    /// Start the web UI server
    Serve {
        #[command(subcommand)]
        action: Option<ServeAction>,

        /// Port to listen on
        #[arg(long, default_value = "4200", global = true)]
        port: u16,
        /// Open browser automatically
        #[arg(long, global = true)]
        open: bool,
        /// Dev mode: API only over HTTP (no static files, no TLS)
        #[arg(long, global = true)]
        dev: bool,
        /// Disable TLS and serve plain HTTP (escape hatch)
        #[arg(long, global = true)]
        no_tls: bool,
        /// Internal: re-exec'd background child (hidden)
        #[arg(long, hide = true, global = true)]
        daemon_child: bool,
    },

    /// Run the MCP (Model Context Protocol) server over stdio
    Mcp {
        /// Path to the project directory (defaults to current directory)
        #[arg(long)]
        project: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum LsKind {
    /// List epics
    Epics,
    /// List plans
    Plans,
    /// List notes
    Notes,
    /// List maps
    Maps,
}

#[derive(Subcommand)]
pub enum ServeAction {
    /// Run the server in the background
    Start,
    /// Stop the background server
    Stop,
    /// Show whether the server is running
    Status,
    /// Show background server logs
    Logs {
        /// Number of lines to show
        #[arg(short = 'n', long, default_value = "100")]
        lines: usize,
        /// Follow the log file
        #[arg(short = 'f', long)]
        follow: bool,
    },
}

pub fn execute(cmd: Commands) -> anyhow::Result<()> {
    match cmd {
        Commands::Init {
            name,
            description,
            empty,
        } => init::run(name, description, empty),
        Commands::Add {
            title,
            r#type,
            priority,
            effort,
            epic,
            plan,
            map,
            tags,
            template,
            body_file,
        } => add::run(
            title, r#type, priority, effort, epic, plan, map, tags, template, body_file,
        ),
        Commands::Show { id } => show::run(id),
        Commands::Comment { id, body_file } => comment::run(id, body_file),
        Commands::Resolve {
            id,
            answer_file,
            gist,
        } => resolve::run(id, answer_file, gist),
        Commands::Edit { id } => edit::run(id),
        Commands::Ls {
            kind,
            status,
            priority,
            epic,
            plan,
            map,
            tags,
            assignee,
            r#type,
            archived,
            frontier,
        } => ls::run(
            kind, status, priority, epic, plan, map, tags, assignee, r#type, archived, frontier,
        ),
        Commands::Status { id, new_status } => status::run(id, new_status),
        Commands::Sync { normalize } => sync::run(normalize),
        Commands::Start { id, user } => start::run(id, user),
        Commands::Done { id } => done::run(id),
        Commands::Promote {
            id,
            priority,
            effort,
        } => promote::run(id, priority, effort),
        Commands::Plan {
            id,
            title,
            tags,
            template,
            body_file,
        } => plan::run(id, title, tags, template, body_file),
        Commands::Epic {
            title,
            priority,
            tags,
            template,
            body_file,
        } => epic::run(title, priority, tags, template, body_file),
        Commands::Note {
            title,
            r#type,
            tags,
            template,
            body_file,
        } => note::run(title, r#type, tags, template, body_file),
        Commands::Map {
            title,
            tags,
            template,
            body_file,
        } => map::run(title, tags, template, body_file),
        Commands::Update {
            id,
            title,
            status,
            priority,
            effort,
            r#type,
            assignee,
            clear_assignee,
            position,
            clear_position,
            add_tag,
            remove_tag,
            started,
            clear_started,
            target,
            clear_target,
            note_type,
        } => update::run(
            id,
            title,
            status,
            priority,
            effort,
            r#type,
            assignee,
            clear_assignee,
            position,
            clear_position,
            add_tag,
            remove_tag,
            started,
            clear_started,
            target,
            clear_target,
            note_type,
        ),
        Commands::Link {
            from,
            to,
            relation,
            remove,
        } => link::run(from, to, relation, remove),
        Commands::Check { orphans, fix } => check::run(orphans, fix),
        Commands::Stale { days } => stale::run(days),
        Commands::Archive {
            id,
            all_done,
            dry_run,
        } => archive::run(id, all_done, dry_run),
        Commands::Unarchive { id } => unarchive::run(id),
        Commands::Context { focus } => context::run(focus),
        Commands::Metrics => metrics::run(),
        Commands::Graph { id, depth } => graph::run(id, depth),
        Commands::ClaudeMd => claude_md::run(),
        Commands::Dashboard => dashboard::run(),
        Commands::Serve {
            action,
            port,
            open,
            dev,
            no_tls,
            daemon_child,
        } => match action {
            Some(ServeAction::Start) => serve_daemon::start(port, dev, no_tls),
            Some(ServeAction::Stop) => serve_daemon::stop(),
            Some(ServeAction::Status) => serve_daemon::status(),
            Some(ServeAction::Logs { lines, follow }) => serve_daemon::logs(lines, follow),
            None => {
                let rt = tokio::runtime::Runtime::new()?;
                let result = rt.block_on(serve::run(port, open, dev, no_tls, daemon_child));
                rt.shutdown_timeout(std::time::Duration::from_secs(3));
                result
            }
        }
        Commands::Mcp { project } => mcp::run(project),
    }
}

/// Split a comma-separated string into a Vec of trimmed strings.
pub fn parse_comma_list(s: &str) -> Vec<String> {
    s.split(',')
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect()
}

/// Read body content from a file path, or stdin when path is `-`.
pub fn read_body_file(path: &str) -> anyhow::Result<String> {
    if path == "-" {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        Ok(buf)
    } else {
        Ok(std::fs::read_to_string(path)?)
    }
}

/// Resolve `--body-file` when present; `None` leaves the template body.
pub fn optional_body_file(body_file: &Option<String>) -> anyhow::Result<Option<String>> {
    match body_file {
        Some(path) => Ok(Some(read_body_file(path)?)),
        None => Ok(None),
    }
}

