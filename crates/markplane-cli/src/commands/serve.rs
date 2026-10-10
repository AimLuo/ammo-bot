use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Extension, Path, Query, Request, State};
use axum::http::{HeaderValue, StatusCode, Uri};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, patch, post};
use hyper::body::Incoming;
use futures::SinkExt;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, broadcast};
use tower::Layer;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
#[cfg(not(feature = "embed-ui"))]
use tower_http::services::ServeDir;

use markplane_core::{
    Effort, Epic, EpicStatus, EpicUpdate, LinkAction, LinkRelation, Map, MarkplaneDocument,
    MarkplaneError, Note, NoteUpdate, Patch, Plan, PlanUpdate, Priority, Project, QueryFilter,
    ScanScope, StatusCategory, Task, TaskUpdate, TaskWorkflow, find_blocked_items,
    find_frontier_items, parse_id, split_markdown_sections,
};

use super::serve_daemon::{self, PidInfo, PidLock};
use super::serve_registry::{self, RegistryEntry};

#[cfg(feature = "embed-ui")]
use rust_embed::Embed;

#[cfg(feature = "embed-ui")]
#[derive(Embed)]
#[folder = "../markplane-web/ui/out"]
struct WebAssets;

#[cfg(feature = "embed-ui")]
#[derive(Embed)]
#[folder = "../../docs"]
struct DocAssets;

/// Per-Project runtime: loaded Project, isolated WS fan-out, and file watcher.
struct ProjectRuntime {
    project: Project,
    ws_tx: broadcast::Sender<String>,
    watcher_abort: tokio::task::AbortHandle,
}

impl ProjectRuntime {
    fn abort_watcher(&self) {
        self.watcher_abort.abort();
    }
}

/// WebSocket broadcast channel for the Project resolved by the rewrite layer.
#[derive(Clone)]
struct ProjectWs(broadcast::Sender<String>);

struct AppState {
    config_dir: PathBuf,
    port: u16,
    dev: bool,
    /// When true, CORS / WebSocket Origin checks use `https://`.
    use_tls: bool,
    /// Optional HOME override (tests); production leaves this `None`.
    home_override: Option<PathBuf>,
    runtimes: Mutex<HashMap<String, Arc<ProjectRuntime>>>,
}

enum RuntimeError {
    NotFound,
    Unavailable { path: String },
}

impl AppState {
    fn new(config_dir: PathBuf, port: u16, dev: bool, use_tls: bool) -> Arc<Self> {
        Self::with_home(config_dir, port, dev, use_tls, None)
    }

    fn with_home(
        config_dir: PathBuf,
        port: u16,
        dev: bool,
        use_tls: bool,
        home_override: Option<PathBuf>,
    ) -> Arc<Self> {
        Arc::new(Self {
            config_dir,
            port,
            dev,
            use_tls,
            home_override,
            runtimes: Mutex::new(HashMap::new()),
        })
    }

    async fn get_or_create_runtime(
        self: &Arc<Self>,
        id: &str,
    ) -> Result<Arc<ProjectRuntime>, RuntimeError> {
        {
            let guard = self.runtimes.lock().await;
            if let Some(rt) = guard.get(id) {
                return Ok(rt.clone());
            }
        }

        let entry = match serve_registry::find(&self.config_dir, id) {
            Ok(Some(e)) => e,
            Ok(None) | Err(_) => return Err(RuntimeError::NotFound),
        };

        self.establish_runtime(id, &entry).await
    }

    async fn establish_runtime(
        self: &Arc<Self>,
        id: &str,
        entry: &RegistryEntry,
    ) -> Result<Arc<ProjectRuntime>, RuntimeError> {
        let project_path = PathBuf::from(&entry.path);
        let markplane = project_path.join(".markplane");
        if !project_path.is_dir() || !markplane.is_dir() {
            self.drop_runtime(id).await;
            return Err(RuntimeError::Unavailable {
                path: entry.path.clone(),
            });
        }

        let project = Project::new(markplane);
        if let Err(e) = project.sync_all() {
            eprintln!("sync_all failed for project {id}: {e}");
            self.drop_runtime(id).await;
            return Err(RuntimeError::Unavailable {
                path: entry.path.clone(),
            });
        }

        let (ws_tx, _) = broadcast::channel::<String>(256);
        let watch_root = project.root().to_path_buf();
        let ws_tx_clone = ws_tx.clone();
        let watcher = tokio::spawn(async move {
            if let Err(e) = run_file_watcher(watch_root, ws_tx_clone).await {
                eprintln!("File watcher error: {e}");
            }
        });
        let watcher_abort = watcher.abort_handle();

        let runtime = Arc::new(ProjectRuntime {
            project,
            ws_tx,
            watcher_abort,
        });

        {
            let mut guard = self.runtimes.lock().await;
            // Another request may have won the race.
            if let Some(existing) = guard.get(id) {
                runtime.abort_watcher();
                return Ok(existing.clone());
            }
            guard.insert(id.to_string(), runtime.clone());
        }

        if let Err(e) = serve_registry::touch_last_opened(&self.config_dir, id) {
            eprintln!("failed to update lastOpened for {id}: {e}");
        }

        Ok(runtime)
    }

    async fn drop_runtime(&self, id: &str) {
        let mut guard = self.runtimes.lock().await;
        if let Some(rt) = guard.remove(id) {
            rt.abort_watcher();
        }
    }
}

// ── Documentation Manifest ───────────────────────────────────────────────

struct DocEntry {
    slug: &'static str,
    title: &'static str,
    path: &'static str,
}

const DOC_MANIFEST: &[DocEntry] = &[
    DocEntry {
        slug: "getting-started",
        title: "Getting Started",
        path: "docs/getting-started.md",
    },
    DocEntry {
        slug: "file-format",
        title: "Item Reference",
        path: "docs/file-format.md",
    },
    DocEntry {
        slug: "cli-reference",
        title: "CLI Reference",
        path: "docs/cli-reference.md",
    },
    DocEntry {
        slug: "mcp-setup",
        title: "MCP Setup",
        path: "docs/mcp-setup.md",
    },
    DocEntry {
        slug: "ai-integration",
        title: "AI Integration",
        path: "docs/ai-integration.md",
    },
    DocEntry {
        slug: "web-ui-guide",
        title: "Web UI Guide",
        path: "docs/web-ui-guide.md",
    },
];

pub async fn run(
    port: u16,
    open: bool,
    dev: bool,
    no_tls: bool,
    daemon_child: bool,
) -> anyhow::Result<()> {
    let config_dir = serve_registry::config_dir();
    fs_create_config_dir(&config_dir)?;

    let mut pid_lock = match PidLock::try_acquire(&config_dir)? {
        Some(mut lock) => {
            // Empty content while locked means "starting" (not leftover JSON).
            lock.clear()?;
            lock
        }
        None => {
            if daemon_child {
                anyhow::bail!("another markplane serve instance is already running");
            }
            // Reuse the running server: register current Project and exit.
            if let Some((_info, url)) = serve_daemon::reuse_running_server(&config_dir)? {
                println!("Server already running — opened {url}");
                // Spec: always open the browser when reusing a running server.
                let _ = open::that(&url);
                return Ok(());
            }
            anyhow::bail!("serve.pid is locked but no server info is available");
        }
    };

    let open_path = if daemon_child {
        // Daemon cwd is $HOME; do not register from there.
        "/".to_string()
    } else {
        match Project::from_current_dir() {
            Ok(project) => {
                let project_dir = project
                    .root()
                    .parent()
                    .unwrap_or(project.root())
                    .to_path_buf();
                let entry = serve_registry::register(&config_dir, &project_dir)?;
                println!(
                    "Registered project '{}' ({})",
                    entry.name,
                    project_dir.display()
                );
                format!("/p/{}/", entry.id)
            }
            Err(_) => {
                println!("No Project in current directory — opening landing page");
                "/".to_string()
            }
        }
    };

    let use_tls = !dev && !no_tls;
    let state = AppState::new(config_dir.clone(), port, dev, use_tls);
    let app = build_app(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let scheme = if use_tls { "https" } else { "http" };
    let base = format!("{scheme}://localhost:{port}");
    let url = format!("{base}{open_path}");

    if use_tls {
        let certs_dir = config_dir.join("certs");
        let ensured = super::certs::ensure_certs(&certs_dir)?;
        if ensured.renewed {
            println!(
                "Generated localhost TLS certificate at {}",
                ensured.cert_path.display()
            );
            println!("{}", super::certs::trust_instructions(&ensured.cert_path));
            let _ = std::io::stdout().flush();
        }
    }

    println!("Markplane web UI starting on {base}");
    if dev {
        println!("Running in dev mode — API only (use Next.js dev server for UI)");
    } else {
        #[cfg(feature = "embed-ui")]
        println!("Serving embedded UI from binary");
        #[cfg(not(feature = "embed-ui"))]
        println!("Serving static UI from crates/markplane-web/ui/out/");
        if no_tls {
            println!("TLS disabled (--no-tls); serving plain HTTP");
        }
    }
    println!("Open {url}");
    let _ = std::io::stdout().flush();

    // Probe the port before claiming readiness in serve.pid.
    match std::net::TcpListener::bind(addr) {
        Ok(probe) => drop(probe),
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            eprintln!("Port {port} is already in use");
            eprintln!("Try --port <other> or: lsof -iTCP:{port} -sTCP:LISTEN");
            return Err(e.into());
        }
        Err(e) => return Err(e.into()),
    }

    let info = PidInfo {
        pid: std::process::id(),
        port,
        url: base.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        started_at: chrono::Utc::now(),
    };
    pid_lock.write_info(&info)?;

    if open && !daemon_child {
        let _ = open::that(&url);
    }

    // axum-server passes hyper::Incoming; adapt to axum::Body for the boxed stack.
    let app = tower::util::MapRequestLayer::new(incoming_to_axum_body).layer(app);
    let make_service = axum::ServiceExt::into_make_service(app);

    let handle = axum_server::Handle::new();
    let shutdown_handle = handle.clone();
    tokio::spawn(async move {
        shutdown_signal().await;
        shutdown_handle.graceful_shutdown(Some(Duration::from_secs(3)));
    });

    let result = if use_tls {
        let certs_dir = config_dir.join("certs");
        let cert_path = certs_dir.join("cert.pem");
        let key_path = certs_dir.join("key.pem");
        let tls = axum_server::tls_rustls::RustlsConfig::from_pem_file(&cert_path, &key_path).await?;
        axum_server::bind_rustls(addr, tls)
            .handle(handle)
            .serve(make_service)
            .await
    } else {
        axum_server::bind(addr)
            .handle(handle)
            .serve(make_service)
            .await
    };

    let _ = pid_lock.clear();
    result?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

fn incoming_to_axum_body(req: axum::http::Request<Incoming>) -> Request {
    let (parts, body) = req.into_parts();
    Request::from_parts(parts, Body::new(body))
}

fn fs_create_config_dir(config_dir: &FsPath) -> anyhow::Result<()> {
    std::fs::create_dir_all(config_dir)?;
    Ok(())
}

/// Assemble the serve stack: prefix middleware outside the Router (before matching).
fn build_app(
    state: Arc<AppState>,
) -> tower::util::BoxCloneService<Request, Response, Infallible> {
    let router = build_router(state.clone());
    let app = middleware::from_fn_with_state(state, project_prefix_middleware).layer(router);
    tower::util::BoxCloneService::new(app)
}

fn build_router(state: Arc<AppState>) -> Router {
    let api = Router::new()
        .route("/api/projects", get(list_projects).post(add_project))
        .route(
            "/api/projects/{id}",
            patch(rename_project).delete(remove_project),
        )
        .route("/api/fs", get(list_fs))
        .route("/api/init", post(init_and_register_project))
        .route("/api/summary", get(get_summary))
        .route("/api/tasks", get(get_tasks).post(create_task))
        .route(
            "/api/tasks/{id}",
            get(get_task).patch(update_task).delete(delete_task),
        )
        .route("/api/epics", get(get_epics).post(create_epic))
        .route("/api/epics/{id}", get(get_epic).patch(update_epic))
        .route("/api/plans", get(get_plans).post(create_plan))
        .route("/api/plans/{id}", get(get_plan).patch(update_plan))
        .route("/api/notes", get(get_notes).post(create_note))
        .route("/api/notes/{id}", get(get_note).patch(update_note))
        .route("/api/maps", get(get_maps))
        .route("/api/maps/{id}", get(get_map))
        .route("/api/items/{id}/archive", post(post_archive_item))
        .route("/api/items/{id}/unarchive", post(post_unarchive_item))
        .route("/api/link", post(post_link))
        .route("/api/config", get(get_config).patch(patch_config))
        .route("/api/sync", post(post_sync))
        .route("/api/docs", get(get_docs))
        .route("/api/docs/{slug}", get(get_doc))
        .route("/api/search", get(get_search))
        .route("/api/graph", get(get_graph_all))
        .route("/api/graph/{id}", get(get_graph_focused))
        .route("/ws", get(ws_handler))
        .with_state(state.clone());

    let cors = if state.dev {
        CorsLayer::permissive()
    } else {
        let scheme = if state.use_tls { "https" } else { "http" };
        let localhost = format!("{scheme}://localhost:{}", state.port)
            .parse::<HeaderValue>()
            .unwrap();
        let loopback = format!("{scheme}://127.0.0.1:{}", state.port)
            .parse::<HeaderValue>()
            .unwrap();
        CorsLayer::new()
            .allow_origin([localhost, loopback])
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any)
    };

    if state.dev {
        // In dev mode, just serve the API — Next.js dev server proxies to us
        api.layer(cors)
            .layer(axum::extract::DefaultBodyLimit::max(2_097_152))
            .layer(CompressionLayer::new())
    } else {
        // Try embedded assets first (when compiled with --features embed-ui),
        // fall back to filesystem serving for development builds.
        #[cfg(feature = "embed-ui")]
        {
            api.fallback(get(serve_embedded))
                .layer(cors)
                .layer(axum::extract::DefaultBodyLimit::max(2_097_152))
                .layer(CompressionLayer::new())
        }
        #[cfg(not(feature = "embed-ui"))]
        {
            let ui_dir =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../markplane-web/ui/out");
            let serve_dir = ServeDir::new(&ui_dir).append_index_html_on_directories(true);
            api.fallback_service(serve_dir)
                .layer(cors)
                .layer(axum::extract::DefaultBodyLimit::max(2_097_152))
                .layer(CompressionLayer::new())
        }
    }
}

/// Known unprefixed app pages — stale bookmarks redirect to the landing page.
fn is_legacy_page_path(path: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "/dashboard",
        "/backlog",
        "/roadmap",
        "/maps",
        "/plans",
        "/notes",
        "/graph",
        "/archive",
        "/settings",
        "/docs",
    ];
    PREFIXES.iter().any(|p| path == *p || path.starts_with(&format!("{p}/")))
}

#[derive(Clone, Copy)]
enum PrefixKind {
    Api,
    Page,
}

fn parse_project_prefix(path: &str) -> Option<(String, String, PrefixKind)> {
    if let Some(rest) = path.strip_prefix("/api/p/") {
        let (id, r) = rest.split_once('/')?;
        if id.is_empty() {
            return None;
        }
        return Some((id.to_string(), r.to_string(), PrefixKind::Api));
    }
    if let Some(rest) = path.strip_prefix("/p/") {
        if rest.is_empty() {
            return None;
        }
        let (id, r) = match rest.split_once('/') {
            Some((id, r)) => (id, r),
            None => (rest, ""),
        };
        if id.is_empty() {
            return None;
        }
        return Some((id.to_string(), r.to_string(), PrefixKind::Page));
    }
    None
}

fn rewrite_path(kind: PrefixKind, rest: &str) -> String {
    match kind {
        PrefixKind::Api => {
            if rest == "ws" {
                "/ws".to_string()
            } else {
                format!("/api/{rest}")
            }
        }
        PrefixKind::Page => {
            if rest.is_empty() || rest == "/" {
                "/dashboard/".to_string()
            } else if rest.starts_with('/') {
                rest.to_string()
            } else {
                format!("/{rest}")
            }
        }
    }
}

fn set_uri_path(req: &mut Request, new_path: &str) {
    let pq = match req.uri().query() {
        Some(q) => format!("{new_path}?{q}"),
        None => new_path.to_string(),
    };
    let mut parts = req.uri().clone().into_parts();
    parts.path_and_query = Some(pq.parse().expect("path_and_query"));
    *req.uri_mut() = Uri::from_parts(parts).expect("uri");
}

async fn project_prefix_middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();

    if is_legacy_page_path(&path) {
        // Spec: unprefixed legacy pages (e.g. `/backlog/`) 302 to `/`.
        // axum::Redirect has no 302 helper (only 303/307/308).
        return (
            StatusCode::FOUND,
            [(axum::http::header::LOCATION, "/")],
        )
            .into_response();
    }

    if let Some((id, rest, kind)) = parse_project_prefix(&path) {
        match state.get_or_create_runtime(&id).await {
            Ok(rt) => {
                req.extensions_mut().insert(rt.project.clone());
                req.extensions_mut().insert(ProjectWs(rt.ws_tx.clone()));
                set_uri_path(&mut req, &rewrite_path(kind, &rest));
                next.run(req).await
            }
            Err(RuntimeError::NotFound) => match kind {
                PrefixKind::Api => error_response(
                    StatusCode::NOT_FOUND,
                    "project_not_found",
                    &format!("Project '{id}' is not in the registry"),
                )
                .into_response(),
                // Serve the SPA so the client can show a full-page recovery UI.
                PrefixKind::Page => {
                    set_uri_path(&mut req, &rewrite_path(kind, &rest));
                    next.run(req).await
                }
            },
            Err(RuntimeError::Unavailable { path }) => {
                state.drop_runtime(&id).await;
                match kind {
                    PrefixKind::Api => error_response_with_path(
                        StatusCode::CONFLICT,
                        "project_unavailable",
                        "Project path is unavailable",
                        &path,
                    )
                    .into_response(),
                    PrefixKind::Page => {
                        set_uri_path(&mut req, &rewrite_path(kind, &rest));
                        next.run(req).await
                    }
                }
            }
        }
    } else {
        next.run(req).await
    }
}

#[derive(Serialize)]
struct ProjectListItem {
    id: String,
    name: String,
    path: String,
    path_valid: bool,
    last_opened: String,
}

fn registry_entry_to_list_item(e: serve_registry::RegistryEntry) -> ProjectListItem {
    let path_valid = FsPath::new(&e.path).join(".markplane").is_dir();
    ProjectListItem {
        id: e.id,
        name: e.name,
        path: e.path,
        path_valid,
        last_opened: e.last_opened.to_rfc3339(),
    }
}

#[derive(Deserialize)]
struct FsQuery {
    path: Option<String>,
}

#[derive(Serialize)]
struct FsEntry {
    name: String,
    path: String,
    has_markplane: bool,
}

#[derive(Serialize)]
struct FsListing {
    path: String,
    parent: Option<String>,
    home: String,
    has_markplane: bool,
    entries: Vec<FsEntry>,
}

fn home_dir(state: &AppState) -> Result<PathBuf, (StatusCode, Json<ApiError>)> {
    if let Some(home) = &state.home_override {
        return Ok(home.clone());
    }
    serve_registry::user_home().ok_or_else(|| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "HOME or USERPROFILE is not set",
        )
    })
}

fn expand_fs_path(
    state: &AppState,
    raw: Option<&str>,
) -> Result<PathBuf, (StatusCode, Json<ApiError>)> {
    let home = home_dir(state)?;
    let path = match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => home.clone(),
        Some("~") => home.clone(),
        Some(p) if p.starts_with("~/") => home.join(&p[2..]),
        Some(p) => PathBuf::from(p),
    };
    Ok(path)
}

fn resolve_existing_dir(path: &FsPath) -> Result<PathBuf, (StatusCode, Json<ApiError>)> {
    let canonical = fs_canonicalize(path).map_err(|e| {
        error_response_with_path(
            StatusCode::BAD_REQUEST,
            "invalid_path",
            &format!("cannot resolve path: {e}"),
            &path.display().to_string(),
        )
    })?;
    if !canonical.is_dir() {
        return Err(error_response_with_path(
            StatusCode::BAD_REQUEST,
            "not_a_directory",
            "path is not a directory",
            &canonical.display().to_string(),
        ));
    }
    Ok(canonical)
}

fn fs_canonicalize(path: &FsPath) -> std::io::Result<PathBuf> {
    // canonicalize requires the path to exist.
    std::fs::canonicalize(path)
}

async fn list_fs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<FsQuery>,
) -> Result<Json<ApiResponse<FsListing>>, (StatusCode, Json<ApiError>)> {
    let home = home_dir(&state)?;
    let home = resolve_existing_dir(&home)?;
    let requested = expand_fs_path(&state, query.path.as_deref())?;
    let path = resolve_existing_dir(&requested)?;

    let mut entries = Vec::new();
    let read = std::fs::read_dir(&path).map_err(|e| {
        error_response_with_path(
            StatusCode::BAD_REQUEST,
            "invalid_path",
            &format!("cannot read directory: {e}"),
            &path.display().to_string(),
        )
    })?;
    for entry in read {
        let entry = entry.map_err(|e| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                &e.to_string(),
            )
        })?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }
        let meta = entry.metadata().map_err(|e| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                &e.to_string(),
            )
        })?;
        if !meta.is_dir() {
            continue;
        }
        let child = entry.path();
        let has_markplane = child.join(".markplane").is_dir();
        entries.push(FsEntry {
            name: name.into_owned(),
            path: child.to_string_lossy().into_owned(),
            has_markplane,
        });
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));

    let parent = path
        .parent()
        .map(|p| p.to_string_lossy().into_owned());
    let has_markplane = path.join(".markplane").is_dir();

    Ok(Json(ApiResponse {
        data: FsListing {
            path: path.to_string_lossy().into_owned(),
            parent,
            home: home.to_string_lossy().into_owned(),
            has_markplane,
            entries,
        },
    }))
}

#[derive(Deserialize)]
struct PathBody {
    path: String,
}

async fn add_project(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PathBody>,
) -> Result<Json<ApiResponse<ProjectListItem>>, (StatusCode, Json<ApiError>)> {
    let requested = expand_fs_path(&state, Some(&body.path))?;
    let path = resolve_existing_dir(&requested)?;
    if !path.join(".markplane").is_dir() {
        return Err(error_response_with_path(
            StatusCode::CONFLICT,
            "project_not_initialized",
            "Directory is not a Markplane project (missing .markplane/)",
            &path.display().to_string(),
        ));
    }

    let before = serve_registry::find_by_path(&state.config_dir, &path).map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &e.to_string(),
        )
    })?;
    let entry = serve_registry::register(&state.config_dir, &path).map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &e.to_string(),
        )
    })?;
    let entry = if before.is_some() {
        serve_registry::touch_last_opened(&state.config_dir, &entry.id).map_err(|e| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                &e.to_string(),
            )
        })?;
        serve_registry::find(&state.config_dir, &entry.id)
            .map_err(|e| {
                error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    &e.to_string(),
                )
            })?
            .unwrap_or(entry)
    } else {
        entry
    };

    Ok(Json(ApiResponse {
        data: registry_entry_to_list_item(entry),
    }))
}

async fn init_and_register_project(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PathBody>,
) -> Result<Json<ApiResponse<ProjectListItem>>, (StatusCode, Json<ApiError>)> {
    let requested = expand_fs_path(&state, Some(&body.path))?;
    let path = resolve_existing_dir(&requested)?;
    if path.join(".markplane").is_dir() {
        return Err(error_response_with_path(
            StatusCode::CONFLICT,
            "already_initialized",
            "Directory already contains .markplane/",
            &path.display().to_string(),
        ));
    }

    let project_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "my-project".to_string());
    let project = Project::init(path.join(".markplane"), &project_name, "").map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &e.to_string(),
        )
    })?;
    project.sync_all().map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &e.to_string(),
        )
    })?;
    // Match default `markplane init` (seed starter content).
    project.seed_starter_content().map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &e.to_string(),
        )
    })?;
    project.sync_all().map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &e.to_string(),
        )
    })?;

    let entry = serve_registry::register(&state.config_dir, &path).map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &e.to_string(),
        )
    })?;
    Ok(Json(ApiResponse {
        data: registry_entry_to_list_item(entry),
    }))
}

async fn list_projects(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiListResponse<ProjectListItem>>, (StatusCode, Json<ApiError>)> {
    let mut projects = serve_registry::list(&state.config_dir).map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &e.to_string(),
        )
    })?;
    projects.sort_by(|a, b| b.last_opened.cmp(&a.last_opened));
    let data: Vec<ProjectListItem> = projects.into_iter().map(registry_entry_to_list_item).collect();
    let total = data.len();
    Ok(Json(ApiListResponse {
        data,
        meta: ListMeta { total },
    }))
}

#[derive(Deserialize)]
struct RenameProjectRequest {
    name: String,
}

async fn remove_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<ProjectListItem>>, (StatusCode, Json<ApiError>)> {
    let removed = serve_registry::remove(&state.config_dir, &id).map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &e.to_string(),
        )
    })?;
    let Some(entry) = removed else {
        return Err(error_response(
            StatusCode::NOT_FOUND,
            "project_not_found",
            &format!("Project '{id}' is not in the registry"),
        ));
    };
    state.drop_runtime(&id).await;
    Ok(Json(ApiResponse {
        data: registry_entry_to_list_item(entry),
    }))
}

async fn rename_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<RenameProjectRequest>,
) -> Result<Json<ApiResponse<ProjectListItem>>, (StatusCode, Json<ApiError>)> {
    let renamed = serve_registry::rename(&state.config_dir, &id, &body.name).map_err(|e| {
        if e.to_string().contains("must not be empty") {
            error_response(StatusCode::BAD_REQUEST, "invalid_field", &e.to_string())
        } else {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                &e.to_string(),
            )
        }
    })?;
    let Some(entry) = renamed else {
        return Err(error_response(
            StatusCode::NOT_FOUND,
            "project_not_found",
            &format!("Project '{id}' is not in the registry"),
        ));
    };
    Ok(Json(ApiResponse {
        data: registry_entry_to_list_item(entry),
    }))
}

// ── Embedded Static File Server ──────────────────────────────────────────

#[cfg(feature = "embed-ui")]
async fn serve_embedded(uri: axum::http::Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // Try the exact path first
    if let Some(content) = WebAssets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, mime.as_ref().to_string())],
            content.data.into_owned(),
        )
            .into_response();
    }

    // For directory paths, try index.html
    let index_path = if path.is_empty() {
        "index.html".to_string()
    } else {
        format!("{}/index.html", path.trim_end_matches('/'))
    };
    if let Some(content) = WebAssets::get(&index_path) {
        return (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "text/html".to_string())],
            content.data.into_owned(),
        )
            .into_response();
    }

    // SPA fallback: serve root index.html for client-side routing
    if let Some(content) = WebAssets::get("index.html") {
        return (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "text/html".to_string())],
            content.data.into_owned(),
        )
            .into_response();
    }

    (StatusCode::NOT_FOUND, "Not found").into_response()
}

// ── File Watcher ─────────────────────────────────────────────────────────

async fn run_file_watcher(root: PathBuf, tx: broadcast::Sender<String>) -> anyhow::Result<()> {
    use notify_debouncer_mini::{DebouncedEventKind, new_debouncer};

    let (notify_tx, mut notify_rx) = tokio::sync::mpsc::channel(100);

    let mut debouncer = new_debouncer(
        Duration::from_millis(100),
        move |events: Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
            if let Ok(events) = events {
                for event in events {
                    if event.kind == DebouncedEventKind::Any {
                        let _ = notify_tx.blocking_send(event.path);
                    }
                }
            }
        },
    )?;

    debouncer
        .watcher()
        .watch(&root, notify::RecursiveMode::Recursive)?;

    // Keep debouncer alive. Docs are global markplane docs — not watched per Project.
    let _debouncer = debouncer;

    while let Some(path) = notify_rx.recv().await {
        let path_str = path.to_string_lossy().to_string();

        // Skip non-markdown files (except config.yaml)
        if !path_str.ends_with(".md") && !path_str.ends_with("config.yaml") {
            continue;
        }

        // Skip .context/ directory changes (those are generated)
        if path_str.contains(".context") {
            continue;
        }

        // Skip INDEX.md changes (those are generated by sync)
        if path_str.ends_with("INDEX.md") {
            continue;
        }

        // Determine entity type and ID from path
        let event = if path_str.ends_with("config.yaml") {
            serde_json::json!({"type": "config_changed"})
        } else if let Some(filename) = path.file_stem() {
            let id = filename.to_string_lossy();
            if let Ok((prefix, _)) = parse_id(&id) {
                let entity = match prefix {
                    markplane_core::IdPrefix::Epic => "epic",
                    markplane_core::IdPrefix::Task => "task",
                    markplane_core::IdPrefix::Plan => "plan",
                    markplane_core::IdPrefix::Note => "note",
                    markplane_core::IdPrefix::Map => "map",
                };
                serde_json::json!({
                    "type": "file_changed",
                    "entity": entity,
                    "id": id,
                    "action": "modified"
                })
            } else {
                continue;
            }
        } else {
            continue;
        };

        let msg = event.to_string();
        let _ = tx.send(msg);
    }

    Ok(())
}

// ── WebSocket Handler ────────────────────────────────────────────────────

async fn ws_handler(
    headers: axum::http::HeaderMap,
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Extension(project_ws): Extension<ProjectWs>,
) -> Result<impl IntoResponse, (StatusCode, Json<ApiError>)> {
    // Validate Origin header in non-dev mode
    if !state.dev {
        let scheme = if state.use_tls { "https" } else { "http" };
        let allowed_localhost = format!("{scheme}://localhost:{}", state.port);
        let allowed_loopback = format!("{scheme}://127.0.0.1:{}", state.port);
        match headers.get(axum::http::header::ORIGIN) {
            Some(origin) => {
                let origin_str = origin.to_str().unwrap_or("");
                if origin_str != allowed_localhost && origin_str != allowed_loopback {
                    return Err(error_response(
                        StatusCode::FORBIDDEN,
                        "forbidden",
                        "WebSocket origin not allowed",
                    ));
                }
            }
            None => {
                // No Origin header — allow (same-origin requests may omit it)
            }
        }
    }
    Ok(ws.on_upgrade(move |socket| handle_ws(socket, project_ws)))
}

async fn handle_ws(socket: WebSocket, project_ws: ProjectWs) {
    let (mut sender, mut receiver) = socket.split();

    // Send connected message
    let connected = serde_json::json!({"type": "connected", "version": "0.1.0"});
    let _ = sender
        .send(Message::Text(connected.to_string().into()))
        .await;

    let mut rx = project_ws.0.subscribe();

    // Forward broadcast events to this WebSocket client
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Consume (and ignore) incoming messages from client — keeps connection alive
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if matches!(msg, Message::Close(_)) {
                break;
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = &mut send_task => { recv_task.abort(); }
        _ = &mut recv_task => { send_task.abort(); }
    }
}

// ── API Types ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ApiResponse<T: Serialize> {
    data: T,
}

#[derive(Serialize)]
struct ApiListResponse<T: Serialize> {
    data: Vec<T>,
    meta: ListMeta,
}

#[derive(Serialize)]
struct ListMeta {
    total: usize,
}

#[derive(Serialize)]
struct ConfigResponse {
    project: ProjectInfoResponse,
    context: ContextConfigResponse,
    documentation_paths: Vec<String>,
    task_types: Vec<String>,
    note_types: Vec<String>,
    workflows: WorkflowsResponse,
}

#[derive(Serialize)]
struct ProjectInfoResponse {
    name: String,
    description: String,
}

#[derive(Serialize)]
struct ContextConfigResponse {
    token_budget: u32,
    recent_days: u32,
    auto_generate: bool,
}

#[derive(Serialize)]
struct WorkflowsResponse {
    task: std::collections::BTreeMap<String, Vec<String>>,
}

#[derive(Serialize)]
struct ApiError {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
}

fn error_response(status: StatusCode, code: &str, message: &str) -> (StatusCode, Json<ApiError>) {
    (
        status,
        Json(ApiError {
            error: ErrorDetail {
                code: code.to_string(),
                message: message.to_string(),
                path: None,
            },
        }),
    )
}

fn error_response_with_path(
    status: StatusCode,
    code: &str,
    message: &str,
    path: &str,
) -> (StatusCode, Json<ApiError>) {
    (
        status,
        Json(ApiError {
            error: ErrorDetail {
                code: code.to_string(),
                message: message.to_string(),
                path: Some(path.to_string()),
            },
        }),
    )
}

/// Map a `MarkplaneError` to an HTTP error response.
fn map_core_error(e: MarkplaneError) -> (StatusCode, Json<ApiError>) {
    match &e {
        MarkplaneError::NotFound(_) => {
            error_response(StatusCode::NOT_FOUND, "not_found", &e.to_string())
        }
        MarkplaneError::InvalidId(_) => {
            error_response(StatusCode::BAD_REQUEST, "invalid_id", &e.to_string())
        }
        MarkplaneError::InvalidStatus(_) | MarkplaneError::InvalidTransition { .. } => {
            error_response(StatusCode::BAD_REQUEST, "invalid_status", &e.to_string())
        }
        MarkplaneError::InvalidLink(_) => {
            error_response(StatusCode::BAD_REQUEST, "invalid_link", &e.to_string())
        }
        MarkplaneError::InvalidPosition(_) => error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_position",
            &e.to_string(),
        ),
        MarkplaneError::DuplicateId(_) => {
            error_response(StatusCode::CONFLICT, "duplicate_id", &e.to_string())
        }
        MarkplaneError::BrokenReference(_) => {
            error_response(StatusCode::BAD_REQUEST, "broken_reference", &e.to_string())
        }
        MarkplaneError::Config(_) | MarkplaneError::Frontmatter(_) => {
            error_response(StatusCode::BAD_REQUEST, "invalid_field", &e.to_string())
        }
        MarkplaneError::NotInitialized(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            &e.to_string(),
        ),
        MarkplaneError::Io(_) => {
            eprintln!("Internal IO error: {e}");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "An internal I/O error occurred",
            )
        }
        MarkplaneError::Yaml(_) => {
            eprintln!("Internal YAML error: {e}");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "An internal data parsing error occurred",
            )
        }
        MarkplaneError::Persist(_) => {
            eprintln!("Atomic write error: {e}");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "An internal I/O error occurred",
            )
        }
    }
}

/// Map an `anyhow::Error` to a generic HTTP error response.
/// Logs the full error server-side to avoid leaking internal details.
fn map_anyhow_error(e: anyhow::Error) -> (StatusCode, Json<ApiError>) {
    // Try to downcast to MarkplaneError for variant-specific mapping
    match e.downcast::<MarkplaneError>() {
        Ok(me) => map_core_error(me),
        Err(e) => {
            eprintln!("Internal error: {e}");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "An internal error occurred",
            )
        }
    }
}

// ── Documentation API ────────────────────────────────────────────────────

#[derive(Serialize)]
struct DocMetaResponse {
    slug: String,
    title: String,
}

#[derive(Serialize)]
struct DocContentResponse {
    slug: String,
    title: String,
    content: String,
}

/// Map a manifest path (e.g. `"docs/foo.md"`) to the embedded asset key (`"foo.md"`).
#[cfg(feature = "embed-ui")]
fn embedded_doc_key(manifest_path: &str) -> &str {
    manifest_path.strip_prefix("docs/").unwrap_or(manifest_path)
}

/// Global markplane docs root (not tied to any opened Project).
fn markplane_docs_root() -> PathBuf {
    FsPath::new(env!("CARGO_MANIFEST_DIR")).join("../../docs")
}

/// Read documentation content, preferring embedded assets over filesystem.
fn read_doc_content(path: &str) -> Option<String> {
    #[cfg(feature = "embed-ui")]
    {
        if let Some(file) = DocAssets::get(embedded_doc_key(path)) {
            return String::from_utf8(file.data.into_owned()).ok();
        }
    }
    // Manifest paths are like "docs/foo.md"; on disk under markplane_docs_root they're "foo.md".
    let rel = path.strip_prefix("docs/").unwrap_or(path);
    let file_path = markplane_docs_root().join(rel);
    std::fs::read_to_string(&file_path).ok()
}

/// Check whether a doc file is available (embedded or on disk).
fn doc_content_available(path: &str) -> bool {
    #[cfg(feature = "embed-ui")]
    {
        if DocAssets::get(embedded_doc_key(path)).is_some() {
            return true;
        }
    }
    let rel = path.strip_prefix("docs/").unwrap_or(path);
    markplane_docs_root().join(rel).is_file()
}

async fn get_docs() -> Result<Json<ApiListResponse<DocMetaResponse>>, (StatusCode, Json<ApiError>)>
{
    let docs: Vec<DocMetaResponse> = DOC_MANIFEST
        .iter()
        .filter(|entry| doc_content_available(entry.path))
        .map(|entry| DocMetaResponse {
            slug: entry.slug.to_string(),
            title: entry.title.to_string(),
        })
        .collect();
    let total = docs.len();
    Ok(Json(ApiListResponse {
        data: docs,
        meta: ListMeta { total },
    }))
}

async fn get_doc(
    Path(slug): Path<String>,
) -> Result<Json<ApiResponse<DocContentResponse>>, (StatusCode, Json<ApiError>)> {
    let entry = DOC_MANIFEST
        .iter()
        .find(|e| e.slug == slug)
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "not_found", "Document not found"))?;

    let content = read_doc_content(entry.path).ok_or_else(|| {
        error_response(
            StatusCode::NOT_FOUND,
            "not_found",
            "Document file not found",
        )
    })?;

    Ok(Json(ApiResponse {
        data: DocContentResponse {
            slug: entry.slug.to_string(),
            title: entry.title.to_string(),
            content,
        },
    }))
}

/// Diff two vectors to produce add/remove lists.
/// Converts the web UI's replacement semantics (full desired array) to core's add/remove semantics.
fn diff_vec(current: &[String], desired: &[String]) -> (Vec<String>, Vec<String>) {
    let to_add = desired
        .iter()
        .filter(|d| !current.contains(d))
        .cloned()
        .collect();
    let to_remove = current
        .iter()
        .filter(|c| !desired.contains(c))
        .cloned()
        .collect();
    (to_add, to_remove)
}

// ── Task API types ────────────────────────────────────────────────────────

#[derive(Serialize)]
struct TaskResponse {
    id: String,
    title: String,
    status: String,
    priority: String,
    #[serde(rename = "type")]
    item_type: String,
    effort: String,
    tags: Vec<String>,
    epic: Option<String>,
    plan: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    map: Option<String>,
    depends_on: Vec<String>,
    blocks: Vec<String>,
    related: Vec<String>,
    assignee: Option<String>,
    position: Option<String>,
    created: String,
    updated: String,
    body: String,
}

fn task_to_response(doc: &MarkplaneDocument<Task>) -> TaskResponse {
    let fm = &doc.frontmatter;
    TaskResponse {
        id: fm.id.clone(),
        title: fm.title.clone(),
        status: fm.status.to_string(),
        priority: fm.priority.to_string(),
        item_type: fm.item_type.to_string(),
        effort: fm.effort.to_string(),
        tags: fm.tags.clone(),
        epic: fm.epic.clone(),
        plan: fm.plan.clone(),
        map: fm.map.clone(),
        depends_on: fm.depends_on.clone(),
        blocks: fm.blocks.clone(),
        related: fm.related.clone(),
        assignee: fm.assignee.clone(),
        position: fm.position.clone(),
        created: fm.created.to_string(),
        updated: fm.updated.to_string(),
        body: doc.body.clone(),
    }
}

#[derive(Deserialize)]
struct CreateTaskRequest {
    title: String,
    #[serde(rename = "type")]
    item_type: Option<String>,
    #[serde(default = "default_priority")]
    priority: String,
    #[serde(default = "default_effort")]
    effort: String,
    #[serde(default)]
    epic: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

fn default_priority() -> String {
    "medium".to_string()
}
fn default_effort() -> String {
    "medium".to_string()
}

#[derive(Deserialize)]
struct CreateEpicRequest {
    title: String,
    #[serde(default = "default_priority")]
    priority: String,
}

#[derive(Deserialize)]
struct CreatePlanRequest {
    title: String,
    #[serde(default)]
    task_id: Option<String>,
}

#[derive(Deserialize)]
struct CreateNoteRequest {
    title: String,
    #[serde(rename = "type")]
    note_type: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Deserialize)]
struct UpdateTaskRequest {
    title: Option<String>,
    status: Option<String>,
    priority: Option<String>,
    effort: Option<String>,
    #[serde(rename = "type")]
    item_type: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    epic: Option<String>,
    plan: Option<String>,
    assignee: Option<String>,
    position: Option<String>,
    depends_on: Option<Vec<String>>,
    blocks: Option<Vec<String>>,
    #[serde(default)]
    related: Option<Vec<String>>,
    body: Option<String>,
}

// ── Epic API types ────────────────────────────────────────────────────────

#[derive(Serialize)]
struct EpicResponse {
    id: String,
    title: String,
    status: String,
    priority: String,
    started: Option<String>,
    target: Option<String>,
    tags: Vec<String>,
    related: Vec<String>,
    created: String,
    updated: String,
    body: String,
    task_count: usize,
    done_count: usize,
    progress: f64,
    status_breakdown: HashMap<String, usize>,
}

fn epic_to_response(
    doc: &MarkplaneDocument<Epic>,
    tasks: &[MarkplaneDocument<Task>],
    workflow: &markplane_core::TaskWorkflow,
) -> EpicResponse {
    let fm = &doc.frontmatter;
    let cancelled_statuses: std::collections::HashSet<&str> = workflow
        .statuses_in(StatusCategory::Cancelled)
        .iter()
        .map(|s| s.as_str())
        .collect();
    let completed_statuses: std::collections::HashSet<&str> = workflow
        .statuses_in(StatusCategory::Completed)
        .iter()
        .map(|s| s.as_str())
        .collect();

    let epic_tasks: Vec<_> = tasks
        .iter()
        .filter(|t| {
            t.frontmatter.epic.as_deref() == Some(&fm.id)
                && !cancelled_statuses.contains(t.frontmatter.status.as_str())
        })
        .collect();

    let task_count = epic_tasks.len();
    let done_count = epic_tasks
        .iter()
        .filter(|t| completed_statuses.contains(t.frontmatter.status.as_str()))
        .count();
    let progress = if task_count > 0 {
        done_count as f64 / task_count as f64
    } else {
        0.0
    };

    let mut status_breakdown = HashMap::new();
    for t in &epic_tasks {
        *status_breakdown
            .entry(t.frontmatter.status.to_string())
            .or_insert(0) += 1;
    }

    EpicResponse {
        id: fm.id.clone(),
        title: fm.title.clone(),
        status: fm.status.to_string(),
        priority: fm.priority.to_string(),
        started: fm.started.map(|d| d.to_string()),
        target: fm.target.map(|d| d.to_string()),
        tags: fm.tags.clone(),
        related: fm.related.clone(),
        created: fm.created.to_string(),
        updated: fm.updated.to_string(),
        body: doc.body.clone(),
        task_count,
        done_count,
        progress,
        status_breakdown,
    }
}

#[derive(Deserialize)]
struct UpdateEpicRequest {
    title: Option<String>,
    status: Option<String>,
    priority: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    started: Option<String>,
    target: Option<String>,
    #[serde(default)]
    related: Option<Vec<String>>,
    body: Option<String>,
}

// ── Plan API types ────────────────────────────────────────────────────────

#[derive(Serialize)]
struct PlanResponse {
    id: String,
    title: String,
    status: String,
    implements: Vec<String>,
    related: Vec<String>,
    created: String,
    updated: String,
    body: String,
}

fn plan_to_response(doc: &MarkplaneDocument<Plan>) -> PlanResponse {
    let fm = &doc.frontmatter;
    PlanResponse {
        id: fm.id.clone(),
        title: fm.title.clone(),
        status: fm.status.to_string(),
        implements: fm.implements.clone(),
        related: fm.related.clone(),
        created: fm.created.to_string(),
        updated: fm.updated.to_string(),
        body: doc.body.clone(),
    }
}

#[derive(Deserialize)]
struct UpdatePlanRequest {
    title: Option<String>,
    status: Option<String>,
    #[serde(default)]
    related: Option<Vec<String>>,
    body: Option<String>,
}

// ── Note API types ────────────────────────────────────────────────────────

#[derive(Serialize)]
struct NoteResponse {
    id: String,
    title: String,
    #[serde(rename = "type")]
    note_type: String,
    status: String,
    tags: Vec<String>,
    related: Vec<String>,
    created: String,
    updated: String,
    body: String,
}

fn note_to_response(doc: &MarkplaneDocument<Note>) -> NoteResponse {
    let fm = &doc.frontmatter;
    NoteResponse {
        id: fm.id.clone(),
        title: fm.title.clone(),
        note_type: fm.note_type.to_string(),
        status: fm.status.to_string(),
        tags: fm.tags.clone(),
        related: fm.related.clone(),
        created: fm.created.to_string(),
        updated: fm.updated.to_string(),
        body: doc.body.clone(),
    }
}

// ── Map API types ─────────────────────────────────────────────────────────

#[derive(Serialize)]
struct MapListItemResponse {
    id: String,
    title: String,
    status: String,
    specs: Vec<String>,
    related: Vec<String>,
    tags: Vec<String>,
    created: String,
    updated: String,
    decided_count: usize,
    total_count: usize,
    frontier_count: usize,
}

#[derive(Serialize)]
struct MapTicketResponse {
    id: String,
    title: String,
    #[serde(rename = "type")]
    item_type: String,
    status: String,
    assignee: Option<String>,
    depends_on: Vec<String>,
    position: Option<String>,
    question: String,
    answer: Option<String>,
}

#[derive(Serialize)]
struct MapDetailResponse {
    id: String,
    title: String,
    status: String,
    specs: Vec<String>,
    related: Vec<String>,
    tags: Vec<String>,
    created: String,
    updated: String,
    body: String,
    destination: String,
    not_yet_specified: Vec<String>,
    out_of_scope: Vec<String>,
    decided_count: usize,
    total_count: usize,
    frontier_count: usize,
    tickets: Vec<MapTicketResponse>,
}

/// Parse markdown bullet lines from a Map body section.
fn parse_section_bullets(section: &str) -> Vec<String> {
    section
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .filter_map(|l| {
            let text = l
                .strip_prefix("- ")
                .or_else(|| l.strip_prefix("* "))
                .or_else(|| l.strip_prefix("+ "))
                .unwrap_or(l)
                .trim();
            if text.is_empty() {
                return None;
            }
            // Skip template placeholders like "[Open questions…]"
            if text.starts_with('[') && text.ends_with(']') {
                return None;
            }
            Some(text.to_string())
        })
        .collect()
}

/// Extract Question / Answer from a Decision ticket body.
fn ticket_question_answer(body: &str) -> (String, Option<String>) {
    let sections = split_markdown_sections(body);
    let answer = sections
        .get("Answer")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    if let Some(q) = sections
        .get("Question")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        return (q.to_string(), answer);
    }

    // Fall back to body text before the first ## heading (minus a leading # title).
    let mut preamble = String::new();
    for line in body.lines() {
        if line.trim_start().starts_with("## ") {
            break;
        }
        if preamble.is_empty() && line.starts_with("# ") {
            continue;
        }
        if !preamble.is_empty() {
            preamble.push('\n');
        }
        preamble.push_str(line);
    }
    (preamble.trim().to_string(), answer)
}

fn map_ticket_counts(
    tickets: &[MarkplaneDocument<Task>],
    workflow: &TaskWorkflow,
) -> (usize, usize, usize) {
    let completed: HashSet<&str> = workflow
        .statuses_in(StatusCategory::Completed)
        .iter()
        .map(|s| s.as_str())
        .collect();
    let decided_count = tickets
        .iter()
        .filter(|t| completed.contains(t.frontmatter.status.as_str()))
        .count();
    let total_count = tickets.len();
    let frontier_count = find_frontier_items(tickets, workflow).len();
    (decided_count, total_count, frontier_count)
}

fn map_list_item(
    doc: &MarkplaneDocument<Map>,
    tickets: &[MarkplaneDocument<Task>],
    workflow: &TaskWorkflow,
) -> MapListItemResponse {
    let fm = &doc.frontmatter;
    let (decided_count, total_count, frontier_count) = map_ticket_counts(tickets, workflow);
    MapListItemResponse {
        id: fm.id.clone(),
        title: fm.title.clone(),
        status: fm.status.to_string(),
        specs: fm.specs.clone(),
        related: fm.related.clone(),
        tags: fm.tags.clone(),
        created: fm.created.to_string(),
        updated: fm.updated.to_string(),
        decided_count,
        total_count,
        frontier_count,
    }
}

fn map_to_detail(
    doc: &MarkplaneDocument<Map>,
    tickets: &[MarkplaneDocument<Task>],
    workflow: &TaskWorkflow,
) -> MapDetailResponse {
    let fm = &doc.frontmatter;
    let sections = split_markdown_sections(&doc.body);
    let destination = sections
        .get("Destination")
        .cloned()
        .unwrap_or_default();
    let not_yet_specified = sections
        .get("Not yet specified")
        .map(|s| parse_section_bullets(s))
        .unwrap_or_default();
    let out_of_scope = sections
        .get("Out of scope")
        .map(|s| parse_section_bullets(s))
        .unwrap_or_default();
    let (decided_count, total_count, frontier_count) = map_ticket_counts(tickets, workflow);

    let ticket_responses: Vec<MapTicketResponse> = tickets
        .iter()
        .map(|t| {
            let (question, answer) = ticket_question_answer(&t.body);
            let tfm = &t.frontmatter;
            MapTicketResponse {
                id: tfm.id.clone(),
                title: tfm.title.clone(),
                item_type: tfm.item_type.to_string(),
                status: tfm.status.to_string(),
                assignee: tfm.assignee.clone(),
                depends_on: tfm.depends_on.clone(),
                position: tfm.position.clone(),
                question,
                answer,
            }
        })
        .collect();

    MapDetailResponse {
        id: fm.id.clone(),
        title: fm.title.clone(),
        status: fm.status.to_string(),
        specs: fm.specs.clone(),
        related: fm.related.clone(),
        tags: fm.tags.clone(),
        created: fm.created.to_string(),
        updated: fm.updated.to_string(),
        body: doc.body.clone(),
        destination,
        not_yet_specified,
        out_of_scope,
        decided_count,
        total_count,
        frontier_count,
        tickets: ticket_responses,
    }
}

#[derive(Deserialize)]
struct UpdateNoteRequest {
    title: Option<String>,
    status: Option<String>,
    #[serde(rename = "type")]
    note_type: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    related: Option<Vec<String>>,
    body: Option<String>,
}

// ── Summary API types ─────────────────────────────────────────────────────

#[derive(Serialize)]
struct SummaryResponse {
    name: String,
    description: String,
    counts: SummaryCounts,
    now_epics: Vec<EpicResponse>,
    in_progress_tasks: Vec<TaskResponse>,
    blocked_tasks: Vec<TaskResponse>,
    recent_completions: Vec<TaskResponse>,
    next_up_tasks: Vec<TaskResponse>,
    context_summary: Option<String>,
    /// ISO 8601 timestamp of when .context/summary.md was last modified
    context_last_synced: Option<String>,
}

#[derive(Serialize)]
struct SummaryCounts {
    total: usize,
    active: usize,
    planned: usize,
    backlog: usize,
    draft: usize,
    completed: usize,
    blocked: usize,
}

// ── Handlers ──────────────────────────────────────────────────────────────

async fn get_summary(
    Extension(project): Extension<Project>,
) -> Result<Json<ApiResponse<SummaryResponse>>, (StatusCode, Json<ApiError>)> {
    let project = &project;

    let config = project.load_config().map_err(map_core_error)?;
    let tasks = project
        .list_tasks(&QueryFilter::default())
        .map_err(map_core_error)?;
    // All tasks (active + archived) for accurate epic progress
    let all_tasks = project
        .list_tasks(&QueryFilter {
            map: None,
            scope: ScanScope::All,
            ..Default::default()
        })
        .map_err(map_core_error)?;
    let epics = project.list_epics().map_err(map_core_error)?;

    let workflow = &config.workflows.task;
    let blocked = find_blocked_items(&tasks, workflow);

    let active_statuses: HashSet<&str> = workflow
        .statuses_in(StatusCategory::Active)
        .iter()
        .map(|s| s.as_str())
        .collect();
    let planned_statuses: HashSet<&str> = workflow
        .statuses_in(StatusCategory::Planned)
        .iter()
        .map(|s| s.as_str())
        .collect();
    let backlog_statuses: HashSet<&str> = workflow
        .statuses_in(StatusCategory::Backlog)
        .iter()
        .map(|s| s.as_str())
        .collect();
    let draft_statuses: HashSet<&str> = workflow
        .statuses_in(StatusCategory::Draft)
        .iter()
        .map(|s| s.as_str())
        .collect();
    let completed_statuses: HashSet<&str> = workflow
        .statuses_in(StatusCategory::Completed)
        .iter()
        .map(|s| s.as_str())
        .collect();

    let in_progress: Vec<_> = tasks
        .iter()
        .filter(|t| active_statuses.contains(t.frontmatter.status.as_str()))
        .collect();
    let planned_count = tasks
        .iter()
        .filter(|t| planned_statuses.contains(t.frontmatter.status.as_str()))
        .count();
    let backlog_count = tasks
        .iter()
        .filter(|t| backlog_statuses.contains(t.frontmatter.status.as_str()))
        .count();
    let draft_count = tasks
        .iter()
        .filter(|t| draft_statuses.contains(t.frontmatter.status.as_str()))
        .count();
    let done_count = tasks
        .iter()
        .filter(|t| completed_statuses.contains(t.frontmatter.status.as_str()))
        .count();

    let now_epics: Vec<_> = epics
        .iter()
        .filter(|e| e.frontmatter.status == EpicStatus::Now)
        .map(|e| epic_to_response(e, &all_tasks, workflow))
        .collect();

    let in_progress_tasks: Vec<_> = in_progress.iter().map(|t| task_to_response(t)).collect();
    let blocked_tasks: Vec<_> = blocked.iter().map(|t| task_to_response(t)).collect();

    // Recent completions: done items sorted by updated date (most recent first)
    let mut recent_done: Vec<_> = tasks
        .iter()
        .filter(|t| completed_statuses.contains(t.frontmatter.status.as_str()))
        .collect();
    recent_done.sort_by(|a, b| b.frontmatter.updated.cmp(&a.frontmatter.updated));
    let recent_completions: Vec<_> = recent_done
        .iter()
        .take(5)
        .map(|t| task_to_response(t))
        .collect();

    // Next up: top 5 planned/backlog tasks (already priority-sorted by list_tasks)
    let next_up_tasks: Vec<_> = tasks
        .iter()
        .filter(|t| {
            planned_statuses.contains(t.frontmatter.status.as_str())
                || backlog_statuses.contains(t.frontmatter.status.as_str())
        })
        .take(5)
        .map(task_to_response)
        .collect();

    // Read .context/summary.md if it exists
    let context_path = project.root().join(".context").join("summary.md");
    let context_summary = std::fs::read_to_string(&context_path).ok();
    let context_last_synced = std::fs::metadata(&context_path)
        .and_then(|m| m.modified())
        .ok()
        .map(|t| {
            let dt: chrono::DateTime<chrono::Utc> = t.into();
            dt.to_rfc3339()
        });

    let summary = SummaryResponse {
        name: config.project.name,
        description: config.project.description,
        counts: SummaryCounts {
            total: tasks.len(),
            active: in_progress.len(),
            planned: planned_count,
            backlog: backlog_count,
            draft: draft_count,
            completed: done_count,
            blocked: blocked.len(),
        },
        now_epics,
        in_progress_tasks,
        blocked_tasks,
        recent_completions,
        next_up_tasks,
        context_summary,
        context_last_synced,
    };

    Ok(Json(ApiResponse { data: summary }))
}

async fn get_config(
    Extension(project): Extension<Project>,
) -> Result<Json<ApiResponse<ConfigResponse>>, (StatusCode, Json<ApiError>)> {
    let config = project.load_config().map_err(map_core_error)?;

    Ok(Json(ApiResponse {
        data: config_to_response(config),
    }))
}

fn config_to_response(config: markplane_core::Config) -> ConfigResponse {
    let task_workflow_map = config
        .workflows
        .task
        .0
        .iter()
        .map(|(cat, statuses)| (cat.to_string(), statuses.clone()))
        .collect();
    ConfigResponse {
        project: ProjectInfoResponse {
            name: config.project.name,
            description: config.project.description,
        },
        context: ContextConfigResponse {
            token_budget: config.context.token_budget,
            recent_days: config.context.recent_days,
            auto_generate: config.context.auto_generate,
        },
        documentation_paths: config.documentation_paths,
        task_types: config.task_types,
        note_types: config.note_types,
        workflows: WorkflowsResponse {
            task: task_workflow_map,
        },
    }
}

#[derive(Deserialize)]
struct UpdateConfigRequest {
    project: Option<UpdateProjectRequest>,
    context: Option<UpdateContextRequest>,
    documentation_paths: Option<Vec<String>>,
    task_types: Option<Vec<String>>,
    note_types: Option<Vec<String>>,
    workflows: Option<UpdateWorkflowsRequest>,
}

#[derive(Deserialize)]
struct UpdateProjectRequest {
    name: Option<String>,
    description: Option<String>,
}

#[derive(Deserialize)]
struct UpdateContextRequest {
    token_budget: Option<u32>,
    recent_days: Option<u32>,
    auto_generate: Option<bool>,
}

#[derive(Deserialize)]
struct UpdateWorkflowsRequest {
    task: Option<std::collections::BTreeMap<String, Vec<String>>>,
}

async fn patch_config(
    Extension(project): Extension<Project>,
    Json(body): Json<UpdateConfigRequest>,
) -> Result<Json<ApiResponse<ConfigResponse>>, (StatusCode, Json<ApiError>)> {
    let mut config = project.load_config().map_err(map_core_error)?;

    // Helper: validate a type list — trim, lowercase, reject empty/duplicate, require ≥1
    fn validate_types(
        raw: Vec<String>,
        field_name: &str,
    ) -> Result<Vec<String>, (StatusCode, Json<ApiError>)> {
        let cleaned: Vec<String> = raw
            .into_iter()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        if cleaned.is_empty() {
            return Err(error_response(
                StatusCode::BAD_REQUEST,
                "validation_error",
                &format!("{field_name} must have at least one entry"),
            ));
        }
        let mut seen = HashSet::new();
        let deduped: Vec<String> = cleaned
            .into_iter()
            .filter(|s| seen.insert(s.clone()))
            .collect();
        Ok(deduped)
    }

    if let Some(project) = body.project {
        if let Some(name) = project.name {
            let name = name.trim().to_string();
            if name.is_empty() {
                return Err(error_response(
                    StatusCode::BAD_REQUEST,
                    "validation_error",
                    "Project name cannot be empty",
                ));
            }
            if name.len() > 200 {
                return Err(error_response(
                    StatusCode::BAD_REQUEST,
                    "validation_error",
                    "Project name must be 200 characters or fewer",
                ));
            }
            config.project.name = name;
        }
        if let Some(description) = project.description {
            config.project.description = description.trim().to_string();
        }
    }

    if let Some(context) = body.context {
        if let Some(token_budget) = context.token_budget {
            if !(1..=1_000_000).contains(&token_budget) {
                return Err(error_response(
                    StatusCode::BAD_REQUEST,
                    "validation_error",
                    "Token budget must be between 1 and 1,000,000",
                ));
            }
            config.context.token_budget = token_budget;
        }
        if let Some(recent_days) = context.recent_days {
            if !(1..=365).contains(&recent_days) {
                return Err(error_response(
                    StatusCode::BAD_REQUEST,
                    "validation_error",
                    "Recent days must be between 1 and 365",
                ));
            }
            config.context.recent_days = recent_days;
        }
        if let Some(auto_generate) = context.auto_generate {
            config.context.auto_generate = auto_generate;
        }
    }

    if let Some(paths) = body.documentation_paths {
        let mut seen = HashSet::new();
        config.documentation_paths = paths
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && seen.insert(s.clone()))
            .collect();
    }

    if let Some(task_types) = body.task_types {
        config.task_types = validate_types(task_types, "task_types")?;
    }
    if let Some(note_types) = body.note_types {
        config.note_types = validate_types(note_types, "note_types")?;
    }
    if let Some(workflows) = body.workflows
        && let Some(task_workflow) = workflows.task
    {
        let mut workflow_map = std::collections::BTreeMap::new();
        for (cat_str, statuses) in task_workflow {
            let category: StatusCategory = cat_str.parse().map_err(|_| {
                error_response(
                    StatusCode::BAD_REQUEST,
                    "validation_error",
                    &format!("Unknown status category: {cat_str}"),
                )
            })?;
            let cleaned = validate_types(statuses, &format!("workflows.task.{cat_str}"))?;
            workflow_map.insert(category, cleaned);
        }
        // Ensure all 6 categories are present
        for cat in StatusCategory::ALL {
            if !workflow_map.contains_key(cat) {
                return Err(error_response(
                    StatusCode::BAD_REQUEST,
                    "validation_error",
                    &format!("Missing required status category: {cat}"),
                ));
            }
        }

        // Validate that no active tasks use statuses being removed
        let new_workflow = markplane_core::TaskWorkflow(workflow_map);
        let new_statuses: HashSet<&str> = new_workflow.all_statuses().into_iter().collect();
        let tasks = project
            .list_tasks(&QueryFilter::default())
            .map_err(map_core_error)?;
        let mut stranded: Vec<(String, String)> = Vec::new();
        for doc in &tasks {
            let status_str = doc.frontmatter.status.to_string();
            if !new_statuses.contains(status_str.as_str()) {
                stranded.push((doc.frontmatter.id.clone(), status_str));
            }
        }
        if !stranded.is_empty() {
            let details: Vec<String> = stranded
                .iter()
                .map(|(id, status)| format!("{} (status: {})", id, status))
                .collect();
            return Err(error_response(
                StatusCode::CONFLICT,
                "active_tasks_conflict",
                &format!(
                    "Cannot remove statuses used by active tasks: {}",
                    details.join(", ")
                ),
            ));
        }

        config.workflows.task = new_workflow;
    }

    project.save_config(&config).map_err(map_core_error)?;

    Ok(Json(ApiResponse {
        data: config_to_response(config),
    }))
}

#[derive(Deserialize)]
struct TaskQueryParams {
    status: Option<String>,
    priority: Option<String>,
    epic: Option<String>,
    tags: Option<String>,
    assignee: Option<String>,
    #[serde(rename = "type")]
    item_type: Option<String>,
    #[serde(default)]
    archived: bool,
}

async fn get_tasks(
    Extension(project): Extension<Project>,
    Query(params): Query<TaskQueryParams>,
) -> Result<Json<ApiListResponse<TaskResponse>>, (StatusCode, Json<ApiError>)> {
    let filter = QueryFilter {
        status: params.status.map(|s| super::parse_comma_list(&s)),
        priority: params.priority.map(|s| super::parse_comma_list(&s)),
        epic: params.epic,
        plan: None,
        map: None,
        tags: params.tags.map(|s| super::parse_comma_list(&s)),
        assignee: params.assignee,
        item_type: params.item_type.map(|s| super::parse_comma_list(&s)),
        scope: if params.archived {
            ScanScope::Archived
        } else {
            ScanScope::Active
        },
    };

    let tasks = project.list_tasks(&filter).map_err(map_core_error)?;

    let total = tasks.len();
    let data: Vec<_> = tasks.iter().map(task_to_response).collect();

    Ok(Json(ApiListResponse {
        data,
        meta: ListMeta { total },
    }))
}

async fn get_task(
    Extension(project): Extension<Project>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<TaskResponse>>, (StatusCode, Json<ApiError>)> {
    // Validate ID format
    parse_id(&id).map_err(|_| {
        error_response(
            StatusCode::BAD_REQUEST,
            "invalid_id",
            &format!("Invalid ID format: {}", id),
        )
    })?;

    let doc: MarkplaneDocument<Task> = project.read_item(&id).map_err(map_core_error)?;

    Ok(Json(ApiResponse {
        data: task_to_response(&doc),
    }))
}

async fn create_task(
    Extension(project): Extension<Project>,
    Json(body): Json<CreateTaskRequest>,
) -> Result<(StatusCode, Json<ApiResponse<TaskResponse>>), (StatusCode, Json<ApiError>)> {
    let config = project.load_config().map_err(map_core_error)?;
    let item_type = body
        .item_type
        .as_deref()
        .unwrap_or(config.default_task_type());
    let priority: Priority = body
        .priority
        .parse()
        .map_err(|e: MarkplaneError| map_core_error(e))?;
    let effort: Effort = body
        .effort
        .parse()
        .map_err(|e: MarkplaneError| map_core_error(e))?;

    let task = project
        .create_task(
            &body.title,
            item_type,
            priority,
            effort,
            body.epic,
            None,
            body.tags,
            None,
            None,
        )
        .map_err(map_core_error)?;

    // Read the created task back to get the full document with body
    let doc: MarkplaneDocument<Task> = project.read_item(&task.id).map_err(map_core_error)?;

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse {
            data: task_to_response(&doc),
        }),
    ))
}

async fn create_epic(
    Extension(project): Extension<Project>,
    Json(body): Json<CreateEpicRequest>,
) -> Result<(StatusCode, Json<ApiResponse<EpicResponse>>), (StatusCode, Json<ApiError>)> {
    let priority: Priority = body
        .priority
        .parse()
        .map_err(|e: MarkplaneError| map_core_error(e))?;

    let epic = project
        .create_epic(&body.title, priority, vec![], None, None)
        .map_err(map_core_error)?;

    let doc: MarkplaneDocument<Epic> = project.read_item(&epic.id).map_err(map_core_error)?;

    // New epic has no tasks yet
    let tasks: Vec<MarkplaneDocument<Task>> = vec![];
    let config = project.load_config().map_err(map_core_error)?;
    Ok((
        StatusCode::CREATED,
        Json(ApiResponse {
            data: epic_to_response(&doc, &tasks, &config.workflows.task),
        }),
    ))
}

async fn create_plan(
    Extension(project): Extension<Project>,
    Json(body): Json<CreatePlanRequest>,
) -> Result<(StatusCode, Json<ApiResponse<PlanResponse>>), (StatusCode, Json<ApiError>)> {
    // Validate task_id if provided
    if let Some(ref task_id) = body.task_id {
        let (prefix, _) = parse_id(task_id).map_err(|_| {
            error_response(
                StatusCode::BAD_REQUEST,
                "invalid_id",
                &format!("Invalid task ID format: {}", task_id),
            )
        })?;
        if prefix != markplane_core::IdPrefix::Task {
            return Err(error_response(
                StatusCode::BAD_REQUEST,
                "invalid_id",
                &format!("task_id must be a TASK item, got {}", task_id),
            ));
        }
        project.item_path(task_id).map_err(|_| {
            error_response(
                StatusCode::NOT_FOUND,
                "not_found",
                &format!("Task {} not found", task_id),
            )
        })?;
    }

    let implements = match &body.task_id {
        Some(id) => vec![id.clone()],
        None => vec![],
    };

    let plan = project
        .create_plan(&body.title, implements, vec![], None, None)
        .map_err(map_core_error)?;

    // If task_id provided, link the plan back to the task
    if let Some(ref task_id) = body.task_id
        && let Err(e) =
            project
                .link_items(task_id, &plan.id, LinkRelation::Plan, LinkAction::Add)
    {
        // Roll back: delete the plan file
        let _ = std::fs::remove_file(project.item_path(&plan.id).unwrap_or_default());
        return Err(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "link_error",
            &format!("Failed to link plan {} to task {}: {}", plan.id, task_id, e),
        ));
    }

    let doc: MarkplaneDocument<Plan> = project.read_item(&plan.id).map_err(map_core_error)?;

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse {
            data: plan_to_response(&doc),
        }),
    ))
}

async fn create_note(
    Extension(project): Extension<Project>,
    Json(body): Json<CreateNoteRequest>,
) -> Result<(StatusCode, Json<ApiResponse<NoteResponse>>), (StatusCode, Json<ApiError>)> {
    let config = project.load_config().map_err(map_core_error)?;
    let note_type = body
        .note_type
        .as_deref()
        .unwrap_or(config.default_note_type());
    let note = project
        .create_note(&body.title, note_type, body.tags, None, None)
        .map_err(map_core_error)?;

    let doc: MarkplaneDocument<Note> = project.read_item(&note.id).map_err(map_core_error)?;

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse {
            data: note_to_response(&doc),
        }),
    ))
}

async fn update_task(
    Extension(project): Extension<Project>,
    Path(id): Path<String>,
    Json(body): Json<UpdateTaskRequest>,
) -> Result<Json<ApiResponse<TaskResponse>>, (StatusCode, Json<ApiError>)> {
    parse_id(&id).map_err(|_| {
        error_response(
            StatusCode::BAD_REQUEST,
            "invalid_id",
            &format!("Invalid ID format: {}", id),
        )
    })?;

    // Read current state for diffing tags and links
    let current: MarkplaneDocument<Task> = project.read_item(&id).map_err(map_core_error)?;
    let fm = &current.frontmatter;

    // ── Properties → TaskUpdate ──────────────────────────────────────
    let (add_tags, remove_tags) = if let Some(ref desired_tags) = body.tags {
        diff_vec(&fm.tags, desired_tags)
    } else {
        (vec![], vec![])
    };

    let assignee = match &body.assignee {
        None => Patch::Unchanged,
        Some(s) if s.is_empty() => Patch::Clear,
        Some(s) => Patch::Set(s.clone()),
    };
    let position = match &body.position {
        None => Patch::Unchanged,
        Some(s) if s.is_empty() => Patch::Clear,
        Some(s) => Patch::Set(s.clone()),
    };

    let has_changes = body.title.is_some()
        || body.status.is_some()
        || body.priority.is_some()
        || body.effort.is_some()
        || body.item_type.is_some()
        || !add_tags.is_empty()
        || !remove_tags.is_empty()
        || !matches!(assignee, Patch::Unchanged)
        || !matches!(position, Patch::Unchanged)
        || body.body.is_some();
    if has_changes {
        project
            .update_task(
                &id,
                &TaskUpdate {
                    title: body.title,
                    status: body.status,
                    priority: body.priority,
                    effort: body.effort,
                    item_type: body.item_type,
                    assignee,
                    position,
                    add_tags,
                    remove_tags,
                    body: body.body,
                },
            )
            .map_err(map_core_error)?;
    }

    // ── Links → link_items() per change ──────────────────────────────

    // Epic (scalar): Add overwrites, Remove clears
    if let Some(ref desired_epic) = body.epic {
        let desired = if desired_epic.is_empty() {
            None
        } else {
            Some(desired_epic.as_str())
        };
        let current_epic = fm.epic.as_deref();
        if desired != current_epic {
            match desired {
                Some(new) => {
                    // Add overwrites the epic field directly
                    project
                        .link_items(&id, new, LinkRelation::Epic, LinkAction::Add)
                        .map_err(map_core_error)?;
                }
                None => {
                    // Clear: remove the current epic link
                    if let Some(old) = current_epic {
                        project
                            .link_items(&id, old, LinkRelation::Epic, LinkAction::Remove)
                            .map_err(map_core_error)?;
                    }
                }
            }
        }
    }

    // Plan (scalar): Add handles old plan cleanup internally
    if let Some(ref desired_plan) = body.plan {
        let desired = if desired_plan.is_empty() {
            None
        } else {
            Some(desired_plan.as_str())
        };
        let current_plan = fm.plan.as_deref();
        if desired != current_plan {
            match desired {
                Some(new) => {
                    // Add cleans up old plan's implements list and sets new plan
                    project
                        .link_items(&id, new, LinkRelation::Plan, LinkAction::Add)
                        .map_err(map_core_error)?;
                }
                None => {
                    // Clear: remove the current plan link
                    if let Some(old) = current_plan {
                        project
                            .link_items(&id, old, LinkRelation::Plan, LinkAction::Remove)
                            .map_err(map_core_error)?;
                    }
                }
            }
        }
    }

    // depends_on (array): diff
    if let Some(ref desired_deps) = body.depends_on {
        let (to_add, to_remove) = diff_vec(&fm.depends_on, desired_deps);
        // Remove first to avoid transient invalid states
        for dep in &to_remove {
            project
                .link_items(&id, dep, LinkRelation::DependsOn, LinkAction::Remove)
                .map_err(map_core_error)?;
        }
        for dep in &to_add {
            project
                .link_items(&id, dep, LinkRelation::DependsOn, LinkAction::Add)
                .map_err(map_core_error)?;
        }
    }

    // blocks (array): diff
    if let Some(ref desired_blocks) = body.blocks {
        let (to_add, to_remove) = diff_vec(&fm.blocks, desired_blocks);
        for blk in &to_remove {
            project
                .link_items(&id, blk, LinkRelation::Blocks, LinkAction::Remove)
                .map_err(map_core_error)?;
        }
        for blk in &to_add {
            project
                .link_items(&id, blk, LinkRelation::Blocks, LinkAction::Add)
                .map_err(map_core_error)?;
        }
    }

    // related (array): diff
    if let Some(ref desired_related) = body.related {
        let (to_add, to_remove) = diff_vec(&fm.related, desired_related);
        for rel in &to_remove {
            project
                .link_items(&id, rel, LinkRelation::Related, LinkAction::Remove)
                .map_err(map_core_error)?;
        }
        for rel in &to_add {
            project
                .link_items(&id, rel, LinkRelation::Related, LinkAction::Add)
                .map_err(map_core_error)?;
        }
    }

    // Re-read for response
    let doc: MarkplaneDocument<Task> = project.read_item(&id).map_err(map_core_error)?;
    Ok(Json(ApiResponse {
        data: task_to_response(&doc),
    }))
}

async fn delete_task(
    Extension(project): Extension<Project>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<TaskResponse>>, (StatusCode, Json<ApiError>)> {
    parse_id(&id).map_err(|_| {
        error_response(
            StatusCode::BAD_REQUEST,
            "invalid_id",
            &format!("Invalid ID format: {}", id),
        )
    })?;

    let doc: MarkplaneDocument<Task> = project.read_item(&id).map_err(map_core_error)?;

    let response = task_to_response(&doc);

    project.archive_item(&id).map_err(map_core_error)?;

    Ok(Json(ApiResponse { data: response }))
}

// ── Archive / Unarchive ──────────────────────────────────────────────────

#[derive(Serialize)]
struct ArchiveResponse {
    success: bool,
    id: String,
}

async fn post_archive_item(
    Extension(project): Extension<Project>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<ArchiveResponse>>, (StatusCode, Json<ApiError>)> {
    parse_id(&id).map_err(|_| {
        error_response(
            StatusCode::BAD_REQUEST,
            "invalid_id",
            &format!("Invalid ID format: {}", id),
        )
    })?;

    project.archive_item(&id).map_err(map_core_error)?;

    Ok(Json(ApiResponse {
        data: ArchiveResponse { success: true, id },
    }))
}

async fn post_unarchive_item(
    Extension(project): Extension<Project>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<ArchiveResponse>>, (StatusCode, Json<ApiError>)> {
    parse_id(&id).map_err(|_| {
        error_response(
            StatusCode::BAD_REQUEST,
            "invalid_id",
            &format!("Invalid ID format: {}", id),
        )
    })?;

    project.unarchive_item(&id).map_err(map_core_error)?;

    Ok(Json(ApiResponse {
        data: ArchiveResponse { success: true, id },
    }))
}

#[derive(Deserialize)]
struct ArchivedQueryParam {
    #[serde(default)]
    archived: bool,
}

async fn get_epics(
    Extension(project): Extension<Project>,
    Query(params): Query<ArchivedQueryParam>,
) -> Result<Json<ApiListResponse<EpicResponse>>, (StatusCode, Json<ApiError>)> {
    let epics = project
        .list_epics_filtered(params.archived)
        .map_err(map_core_error)?;
    let tasks = project
        .list_tasks(&QueryFilter {
            map: None,
            scope: ScanScope::All,
            ..Default::default()
        })
        .map_err(map_core_error)?;

    let config = project.load_config().map_err(map_core_error)?;
    let total = epics.len();
    let data: Vec<_> = epics
        .iter()
        .map(|e| epic_to_response(e, &tasks, &config.workflows.task))
        .collect();

    Ok(Json(ApiListResponse {
        data,
        meta: ListMeta { total },
    }))
}

async fn get_epic(
    Extension(project): Extension<Project>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<EpicResponse>>, (StatusCode, Json<ApiError>)> {
    parse_id(&id).map_err(|_| {
        error_response(
            StatusCode::BAD_REQUEST,
            "invalid_id",
            &format!("Invalid ID format: {}", id),
        )
    })?;

    let doc: MarkplaneDocument<Epic> = project.read_item(&id).map_err(map_core_error)?;

    let tasks = project
        .list_tasks(&QueryFilter {
            map: None,
            scope: ScanScope::All,
            ..Default::default()
        })
        .map_err(map_core_error)?;
    let config = project.load_config().map_err(map_core_error)?;

    Ok(Json(ApiResponse {
        data: epic_to_response(&doc, &tasks, &config.workflows.task),
    }))
}

async fn get_plans(
    Extension(project): Extension<Project>,
    Query(params): Query<ArchivedQueryParam>,
) -> Result<Json<ApiListResponse<PlanResponse>>, (StatusCode, Json<ApiError>)> {
    let plans = project
        .list_plans_filtered(params.archived)
        .map_err(map_core_error)?;

    let total = plans.len();
    let data: Vec<_> = plans.iter().map(plan_to_response).collect();

    Ok(Json(ApiListResponse {
        data,
        meta: ListMeta { total },
    }))
}

async fn get_plan(
    Extension(project): Extension<Project>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<PlanResponse>>, (StatusCode, Json<ApiError>)> {
    parse_id(&id).map_err(|_| {
        error_response(
            StatusCode::BAD_REQUEST,
            "invalid_id",
            &format!("Invalid ID format: {}", id),
        )
    })?;

    let doc: MarkplaneDocument<Plan> = project.read_item(&id).map_err(map_core_error)?;

    Ok(Json(ApiResponse {
        data: plan_to_response(&doc),
    }))
}

async fn get_notes(
    Extension(project): Extension<Project>,
    Query(params): Query<ArchivedQueryParam>,
) -> Result<Json<ApiListResponse<NoteResponse>>, (StatusCode, Json<ApiError>)> {
    let notes = project
        .list_notes_filtered(params.archived)
        .map_err(map_core_error)?;

    let total = notes.len();
    let data: Vec<_> = notes.iter().map(note_to_response).collect();

    Ok(Json(ApiListResponse {
        data,
        meta: ListMeta { total },
    }))
}

async fn get_note(
    Extension(project): Extension<Project>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<NoteResponse>>, (StatusCode, Json<ApiError>)> {
    parse_id(&id).map_err(|_| {
        error_response(
            StatusCode::BAD_REQUEST,
            "invalid_id",
            &format!("Invalid ID format: {}", id),
        )
    })?;

    let doc: MarkplaneDocument<Note> = project.read_item(&id).map_err(map_core_error)?;

    Ok(Json(ApiResponse {
        data: note_to_response(&doc),
    }))
}

async fn get_maps(
    Extension(project): Extension<Project>,
    Query(params): Query<ArchivedQueryParam>,
) -> Result<Json<ApiListResponse<MapListItemResponse>>, (StatusCode, Json<ApiError>)> {
    let maps = project
        .list_maps_filtered(params.archived)
        .map_err(map_core_error)?;
    let config = project.load_config().map_err(map_core_error)?;
    let workflow = &config.workflows.task;

    let mut data = Vec::with_capacity(maps.len());
    for doc in &maps {
        let tickets = project
            .list_tasks(&QueryFilter {
                map: Some(doc.frontmatter.id.clone()),
                ..Default::default()
            })
            .map_err(map_core_error)?;
        data.push(map_list_item(doc, &tickets, workflow));
    }

    let total = data.len();
    Ok(Json(ApiListResponse {
        data,
        meta: ListMeta { total },
    }))
}

async fn get_map(
    Extension(project): Extension<Project>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<MapDetailResponse>>, (StatusCode, Json<ApiError>)> {
    parse_id(&id).map_err(|_| {
        error_response(
            StatusCode::BAD_REQUEST,
            "invalid_id",
            &format!("Invalid ID format: {}", id),
        )
    })?;

    let doc: MarkplaneDocument<Map> = project.read_item(&id).map_err(map_core_error)?;
    let tickets = project
        .list_tasks(&QueryFilter {
            map: Some(id.clone()),
            ..Default::default()
        })
        .map_err(map_core_error)?;
    let config = project.load_config().map_err(map_core_error)?;

    Ok(Json(ApiResponse {
        data: map_to_detail(&doc, &tickets, &config.workflows.task),
    }))
}

async fn update_epic(
    Extension(project): Extension<Project>,
    Path(id): Path<String>,
    Json(body): Json<UpdateEpicRequest>,
) -> Result<Json<ApiResponse<EpicResponse>>, (StatusCode, Json<ApiError>)> {
    parse_id(&id).map_err(|_| {
        error_response(
            StatusCode::BAD_REQUEST,
            "invalid_id",
            &format!("Invalid ID format: {}", id),
        )
    })?;

    // Read current state for diffing tags and links
    let current: MarkplaneDocument<Epic> = project.read_item(&id).map_err(map_core_error)?;
    let fm = &current.frontmatter;

    // Diff tags if provided
    let (add_tags, remove_tags) = if let Some(ref desired_tags) = body.tags {
        diff_vec(&fm.tags, desired_tags)
    } else {
        (vec![], vec![])
    };

    // Parse date fields to Patch<NaiveDate>
    let started = match body.started {
        None => Patch::Unchanged,
        Some(ref s) if s.is_empty() => Patch::Clear,
        Some(ref s) => Patch::Set(chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(
            |_| {
                error_response(
                    StatusCode::BAD_REQUEST,
                    "invalid_date",
                    "Invalid date format, expected YYYY-MM-DD",
                )
            },
        )?),
    };
    let target = match body.target {
        None => Patch::Unchanged,
        Some(ref s) if s.is_empty() => Patch::Clear,
        Some(ref s) => Patch::Set(chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(
            |_| {
                error_response(
                    StatusCode::BAD_REQUEST,
                    "invalid_date",
                    "Invalid date format, expected YYYY-MM-DD",
                )
            },
        )?),
    };

    // Properties + body
    let has_changes = body.title.is_some()
        || body.status.is_some()
        || body.priority.is_some()
        || !add_tags.is_empty()
        || !remove_tags.is_empty()
        || !matches!(started, Patch::Unchanged)
        || !matches!(target, Patch::Unchanged)
        || body.body.is_some();
    if has_changes {
        project
            .update_epic(
                &id,
                &EpicUpdate {
                    title: body.title,
                    status: body.status,
                    priority: body.priority,
                    add_tags,
                    remove_tags,
                    started,
                    target,
                    body: body.body,
                },
            )
            .map_err(map_core_error)?;
    }

    // ── Links → link_items() per change ──────────────────────────────

    // related (array): diff
    if let Some(ref desired_related) = body.related {
        let (to_add, to_remove) = diff_vec(&fm.related, desired_related);
        for rel in &to_remove {
            project
                .link_items(&id, rel, LinkRelation::Related, LinkAction::Remove)
                .map_err(map_core_error)?;
        }
        for rel in &to_add {
            project
                .link_items(&id, rel, LinkRelation::Related, LinkAction::Add)
                .map_err(map_core_error)?;
        }
    }

    // Re-read for response
    let doc: MarkplaneDocument<Epic> = project.read_item(&id).map_err(map_core_error)?;
    let tasks = project
        .list_tasks(&QueryFilter {
            map: None,
            scope: ScanScope::All,
            ..Default::default()
        })
        .map_err(map_core_error)?;
    let config = project.load_config().map_err(map_core_error)?;

    Ok(Json(ApiResponse {
        data: epic_to_response(&doc, &tasks, &config.workflows.task),
    }))
}

async fn update_plan(
    Extension(project): Extension<Project>,
    Path(id): Path<String>,
    Json(body): Json<UpdatePlanRequest>,
) -> Result<Json<ApiResponse<PlanResponse>>, (StatusCode, Json<ApiError>)> {
    parse_id(&id).map_err(|_| {
        error_response(
            StatusCode::BAD_REQUEST,
            "invalid_id",
            &format!("Invalid ID format: {}", id),
        )
    })?;

    // Read current state for link diffing
    let current: MarkplaneDocument<Plan> = project.read_item(&id).map_err(map_core_error)?;

    // Properties + body
    let has_changes = body.title.is_some() || body.status.is_some() || body.body.is_some();
    if has_changes {
        project
            .update_plan(
                &id,
                &PlanUpdate {
                    title: body.title,
                    status: body.status,
                    body: body.body,
                    ..Default::default()
                },
            )
            .map_err(map_core_error)?;
    }

    // ── Links → link_items() per change ──────────────────────────────

    // related (array): diff
    if let Some(ref desired_related) = body.related {
        let (to_add, to_remove) = diff_vec(&current.frontmatter.related, desired_related);
        for rel in &to_remove {
            project
                .link_items(&id, rel, LinkRelation::Related, LinkAction::Remove)
                .map_err(map_core_error)?;
        }
        for rel in &to_add {
            project
                .link_items(&id, rel, LinkRelation::Related, LinkAction::Add)
                .map_err(map_core_error)?;
        }
    }

    // Re-read for response
    let doc: MarkplaneDocument<Plan> = project.read_item(&id).map_err(map_core_error)?;
    Ok(Json(ApiResponse {
        data: plan_to_response(&doc),
    }))
}

async fn update_note(
    Extension(project): Extension<Project>,
    Path(id): Path<String>,
    Json(body): Json<UpdateNoteRequest>,
) -> Result<Json<ApiResponse<NoteResponse>>, (StatusCode, Json<ApiError>)> {
    parse_id(&id).map_err(|_| {
        error_response(
            StatusCode::BAD_REQUEST,
            "invalid_id",
            &format!("Invalid ID format: {}", id),
        )
    })?;

    // Read current state for diffing tags and links
    let current: MarkplaneDocument<Note> = project.read_item(&id).map_err(map_core_error)?;
    let fm = &current.frontmatter;

    // ── Properties → NoteUpdate ──────────────────────────────────────
    let (add_tags, remove_tags) = if let Some(ref desired_tags) = body.tags {
        diff_vec(&fm.tags, desired_tags)
    } else {
        (vec![], vec![])
    };

    let has_changes = body.title.is_some()
        || body.status.is_some()
        || body.note_type.is_some()
        || !add_tags.is_empty()
        || !remove_tags.is_empty()
        || body.body.is_some();
    if has_changes {
        project
            .update_note(
                &id,
                &NoteUpdate {
                    title: body.title,
                    status: body.status,
                    note_type: body.note_type,
                    add_tags,
                    remove_tags,
                    body: body.body,
                },
            )
            .map_err(map_core_error)?;
    }

    // ── Links → link_items() per change ──────────────────────────────

    // related (array): diff
    if let Some(ref desired_related) = body.related {
        let (to_add, to_remove) = diff_vec(&fm.related, desired_related);
        for rel in &to_remove {
            project
                .link_items(&id, rel, LinkRelation::Related, LinkAction::Remove)
                .map_err(map_core_error)?;
        }
        for rel in &to_add {
            project
                .link_items(&id, rel, LinkRelation::Related, LinkAction::Add)
                .map_err(map_core_error)?;
        }
    }

    // Re-read for response
    let doc: MarkplaneDocument<Note> = project.read_item(&id).map_err(map_core_error)?;
    Ok(Json(ApiResponse {
        data: note_to_response(&doc),
    }))
}

async fn post_sync(
    Extension(project): Extension<Project>,
    Extension(project_ws): Extension<ProjectWs>,
) -> Result<Json<ApiResponse<SyncResponse>>, (StatusCode, Json<ApiError>)> {
    project.sync_all().map_err(map_core_error)?;

    // Broadcast sync_complete only to this Project's WebSocket clients
    let event = serde_json::json!({"type": "sync_complete"});
    let _ = project_ws.0.send(event.to_string());

    Ok(Json(ApiResponse {
        data: SyncResponse {
            success: true,
            message: "Sync complete".to_string(),
        },
    }))
}

#[derive(Serialize)]
struct SyncResponse {
    success: bool,
    message: String,
}

// ── Link ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct LinkRequest {
    from: String,
    to: String,
    relation: String,
    #[serde(default)]
    remove: bool,
}

#[derive(Serialize)]
struct LinkResponse {
    from: String,
    to: String,
    relation: String,
    action: String,
}

async fn post_link(
    Extension(project): Extension<Project>,
    Json(req): Json<LinkRequest>,
) -> Result<Json<ApiResponse<LinkResponse>>, (StatusCode, Json<ApiError>)> {
    let relation: LinkRelation = req
        .relation
        .parse()
        .map_err(|e: MarkplaneError| map_core_error(e))?;
    let action = if req.remove {
        LinkAction::Remove
    } else {
        LinkAction::Add
    };

    project
        .link_items(&req.from, &req.to, relation, action)
        .map_err(map_core_error)?;

    let action_str = if req.remove { "removed" } else { "added" };

    Ok(Json(ApiResponse {
        data: LinkResponse {
            from: req.from,
            to: req.to,
            relation: relation.to_string(),
            action: action_str.to_string(),
        },
    }))
}

// ── Search ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct SearchParams {
    q: String,
    #[serde(default)]
    include_archived: bool,
}

#[derive(Serialize)]
struct SearchResultResponse {
    id: String,
    entity_type: String,
    title: String,
    status: String,
    priority: Option<String>,
    snippet: String,
    score: f64,
    archived: bool,
}

async fn get_search(
    Extension(project): Extension<Project>,
    Query(params): Query<SearchParams>,
) -> Result<Json<ApiListResponse<SearchResultResponse>>, (StatusCode, Json<ApiError>)> {
    let query = params.q.to_lowercase();
    if query.len() < 2 || query.len() > 500 {
        return Ok(Json(ApiListResponse {
            data: vec![],
            meta: ListMeta { total: 0 },
        }));
    }

    let mut results: Vec<SearchResultResponse> = Vec::new();

    // Search tasks
    let mut tasks = project
        .list_tasks(&QueryFilter::default())
        .map_err(map_core_error)?;
    let archived_task_ids: HashSet<String> = if params.include_archived {
        let archived = project
            .list_tasks(&QueryFilter {
                map: None,
                scope: ScanScope::Archived,
                ..Default::default()
            })
            .map_err(map_core_error)?;
        let ids = archived.iter().map(|d| d.frontmatter.id.clone()).collect();
        tasks.extend(archived);
        ids
    } else {
        HashSet::new()
    };
    for doc in &tasks {
        let fm = &doc.frontmatter;
        let id_match = fm.id.to_lowercase().contains(&query);
        let title_match = fm.title.to_lowercase().contains(&query);
        let tag_match = fm.tags.iter().any(|t| t.to_lowercase().contains(&query));
        let assignee_match = fm
            .assignee
            .as_ref()
            .is_some_and(|a| a.to_lowercase().contains(&query));
        let body_match = doc.body.to_lowercase().contains(&query);
        if id_match || title_match || tag_match || assignee_match || body_match {
            let score = if id_match {
                3.0
            } else if title_match {
                2.0
            } else if tag_match || assignee_match {
                1.5
            } else {
                1.0
            };
            let snippet = extract_snippet(&doc.body, &query);
            results.push(SearchResultResponse {
                id: fm.id.clone(),
                entity_type: "task".to_string(),
                title: fm.title.clone(),
                status: fm.status.to_string(),
                priority: Some(fm.priority.to_string()),
                snippet,
                score,
                archived: archived_task_ids.contains(&fm.id),
            });
        }
    }

    // Search epics
    let mut epics = project.list_epics().map_err(map_core_error)?;
    let archived_epic_ids: HashSet<String> = if params.include_archived {
        let archived = project
            .list_epics_filtered(true)
            .map_err(map_core_error)?;
        let ids = archived.iter().map(|d| d.frontmatter.id.clone()).collect();
        epics.extend(archived);
        ids
    } else {
        HashSet::new()
    };
    for doc in &epics {
        let fm = &doc.frontmatter;
        let id_match = fm.id.to_lowercase().contains(&query);
        let title_match = fm.title.to_lowercase().contains(&query);
        let tag_match = fm.tags.iter().any(|t| t.to_lowercase().contains(&query));
        let body_match = doc.body.to_lowercase().contains(&query);
        if id_match || title_match || tag_match || body_match {
            let score = if id_match {
                3.0
            } else if title_match {
                2.0
            } else if tag_match {
                1.5
            } else {
                1.0
            };
            let snippet = extract_snippet(&doc.body, &query);
            results.push(SearchResultResponse {
                id: fm.id.clone(),
                entity_type: "epic".to_string(),
                title: fm.title.clone(),
                status: fm.status.to_string(),
                priority: Some(fm.priority.to_string()),
                snippet,
                score,
                archived: archived_epic_ids.contains(&fm.id),
            });
        }
    }

    // Search plans
    let mut plans = project.list_plans().map_err(map_core_error)?;
    let archived_plan_ids: HashSet<String> = if params.include_archived {
        let archived = project
            .list_plans_filtered(true)
            .map_err(map_core_error)?;
        let ids = archived.iter().map(|d| d.frontmatter.id.clone()).collect();
        plans.extend(archived);
        ids
    } else {
        HashSet::new()
    };
    for doc in &plans {
        let fm = &doc.frontmatter;
        let id_match = fm.id.to_lowercase().contains(&query);
        let title_match = fm.title.to_lowercase().contains(&query);
        let body_match = doc.body.to_lowercase().contains(&query);
        if id_match || title_match || body_match {
            let score = if id_match {
                3.0
            } else if title_match {
                2.0
            } else {
                1.0
            };
            let snippet = extract_snippet(&doc.body, &query);
            results.push(SearchResultResponse {
                id: fm.id.clone(),
                entity_type: "plan".to_string(),
                title: fm.title.clone(),
                status: fm.status.to_string(),
                priority: None,
                snippet,
                score,
                archived: archived_plan_ids.contains(&fm.id),
            });
        }
    }

    // Search notes
    let mut notes = project.list_notes().map_err(map_core_error)?;
    let archived_note_ids: HashSet<String> = if params.include_archived {
        let archived = project
            .list_notes_filtered(true)
            .map_err(map_core_error)?;
        let ids = archived.iter().map(|d| d.frontmatter.id.clone()).collect();
        notes.extend(archived);
        ids
    } else {
        HashSet::new()
    };
    for doc in &notes {
        let fm = &doc.frontmatter;
        let id_match = fm.id.to_lowercase().contains(&query);
        let title_match = fm.title.to_lowercase().contains(&query);
        let tag_match = fm.tags.iter().any(|t| t.to_lowercase().contains(&query));
        let body_match = doc.body.to_lowercase().contains(&query);
        if id_match || title_match || tag_match || body_match {
            let score = if id_match {
                3.0
            } else if title_match {
                2.0
            } else if tag_match {
                1.5
            } else {
                1.0
            };
            let snippet = extract_snippet(&doc.body, &query);
            results.push(SearchResultResponse {
                id: fm.id.clone(),
                entity_type: "note".to_string(),
                title: fm.title.clone(),
                status: fm.status.to_string(),
                priority: None,
                snippet,
                score,
                archived: archived_note_ids.contains(&fm.id),
            });
        }
    }

    // Search maps
    let mut maps = project.list_maps().map_err(map_core_error)?;
    let archived_map_ids: HashSet<String> = if params.include_archived {
        let archived = project
            .list_maps_filtered(true)
            .map_err(map_core_error)?;
        let ids = archived.iter().map(|d| d.frontmatter.id.clone()).collect();
        maps.extend(archived);
        ids
    } else {
        HashSet::new()
    };
    for doc in &maps {
        let fm = &doc.frontmatter;
        let id_match = fm.id.to_lowercase().contains(&query);
        let title_match = fm.title.to_lowercase().contains(&query);
        let tag_match = fm.tags.iter().any(|t| t.to_lowercase().contains(&query));
        let body_match = doc.body.to_lowercase().contains(&query);
        if id_match || title_match || tag_match || body_match {
            let score = if id_match {
                3.0
            } else if title_match {
                2.0
            } else if tag_match {
                1.5
            } else {
                1.0
            };
            let snippet = extract_snippet(&doc.body, &query);
            results.push(SearchResultResponse {
                id: fm.id.clone(),
                entity_type: "map".to_string(),
                title: fm.title.clone(),
                status: fm.status.to_string(),
                priority: None,
                snippet,
                score,
                archived: archived_map_ids.contains(&fm.id),
            });
        }
    }

    // Sort by score descending, then by ID
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });

    let total = results.len();
    Ok(Json(ApiListResponse {
        data: results,
        meta: ListMeta { total },
    }))
}

/// Extract a text snippet around the first match of `query` in `body`.
fn extract_snippet(body: &str, query: &str) -> String {
    let lower = body.to_lowercase();
    if let Some(pos) = lower.find(query) {
        let start = pos.saturating_sub(40);
        let end = (pos + query.len() + 80).min(body.len());
        // Find safe UTF-8 boundaries
        let start = body.floor_char_boundary(start);
        let end = body.ceil_char_boundary(end);
        let mut snippet = body[start..end].to_string();
        // Replace newlines with spaces for cleaner display
        snippet = snippet.replace('\n', " ");
        if start > 0 {
            snippet.insert_str(0, "...");
        }
        if end < body.len() {
            snippet.push_str("...");
        }
        snippet
    } else {
        // No body match — return first ~120 chars of body
        let end = (120).min(body.len());
        let end = body.ceil_char_boundary(end);
        let mut snippet = body[..end].replace('\n', " ");
        if end < body.len() {
            snippet.push_str("...");
        }
        snippet
    }
}

// ── Graph ────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct GraphNodeResponse {
    id: String,
    #[serde(rename = "type")]
    node_type: String,
    title: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
}

#[derive(Serialize)]
struct GraphEdgeResponse {
    source: String,
    target: String,
    relation: String,
}

#[derive(Serialize)]
struct GraphResponse {
    nodes: Vec<GraphNodeResponse>,
    edges: Vec<GraphEdgeResponse>,
}

async fn get_graph_all(
    Extension(project): Extension<Project>,
) -> Result<Json<ApiResponse<GraphResponse>>, (StatusCode, Json<ApiError>)> {
    let graph = build_graph(&project, None).map_err(map_anyhow_error)?;
    Ok(Json(ApiResponse { data: graph }))
}

async fn get_graph_focused(
    Extension(project): Extension<Project>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<GraphResponse>>, (StatusCode, Json<ApiError>)> {
    parse_id(&id).map_err(|_| {
        error_response(
            StatusCode::BAD_REQUEST,
            "invalid_id",
            &format!("Invalid ID format: {}", id),
        )
    })?;
    let graph = build_graph(&project, Some(&id)).map_err(map_anyhow_error)?;
    Ok(Json(ApiResponse { data: graph }))
}

fn build_graph(project: &Project, focus_id: Option<&str>) -> anyhow::Result<GraphResponse> {
    use std::collections::HashSet;

    let mut nodes_map: HashMap<String, GraphNodeResponse> = HashMap::new();
    let mut edges: Vec<GraphEdgeResponse> = Vec::new();

    // Collect tasks
    let tasks = project.list_tasks(&QueryFilter::default())?;
    for doc in &tasks {
        let fm = &doc.frontmatter;
        nodes_map.insert(
            fm.id.clone(),
            GraphNodeResponse {
                id: fm.id.clone(),
                node_type: "task".to_string(),
                title: fm.title.clone(),
                status: fm.status.to_string(),
                priority: Some(fm.priority.to_string()),
                tags: fm.tags.clone(),
            },
        );
        // Only emit blocks edges (skip depends_on — blocks already covers the
        // same relationship from the other side)
        for blk in &fm.blocks {
            edges.push(GraphEdgeResponse {
                source: fm.id.clone(),
                target: blk.clone(),
                relation: "blocks".to_string(),
            });
        }
        if let Some(epic) = &fm.epic {
            edges.push(GraphEdgeResponse {
                source: epic.clone(),
                target: fm.id.clone(),
                relation: "epic".to_string(),
            });
        }
        // Skip task.plan edges — plan.implements covers the same relationship
        for rel in &fm.related {
            edges.push(GraphEdgeResponse {
                source: fm.id.clone(),
                target: rel.clone(),
                relation: "related".to_string(),
            });
        }
    }

    // Collect epics
    let epics = project.list_epics()?;
    for doc in &epics {
        let fm = &doc.frontmatter;
        nodes_map.insert(
            fm.id.clone(),
            GraphNodeResponse {
                id: fm.id.clone(),
                node_type: "epic".to_string(),
                title: fm.title.clone(),
                status: fm.status.to_string(),
                priority: Some(fm.priority.to_string()),
                tags: fm.tags.clone(),
            },
        );
        for rel in &fm.related {
            edges.push(GraphEdgeResponse {
                source: fm.id.clone(),
                target: rel.clone(),
                relation: "related".to_string(),
            });
        }
    }

    // Collect plans
    let plans = project.list_plans()?;
    for doc in &plans {
        let fm = &doc.frontmatter;
        nodes_map.insert(
            fm.id.clone(),
            GraphNodeResponse {
                id: fm.id.clone(),
                node_type: "plan".to_string(),
                title: fm.title.clone(),
                status: fm.status.to_string(),
                priority: None,
                tags: vec![],
            },
        );
        for imp in &fm.implements {
            edges.push(GraphEdgeResponse {
                source: fm.id.clone(),
                target: imp.clone(),
                relation: "implements".to_string(),
            });
        }
        for rel in &fm.related {
            edges.push(GraphEdgeResponse {
                source: fm.id.clone(),
                target: rel.clone(),
                relation: "related".to_string(),
            });
        }
    }

    // Collect notes
    let notes = project.list_notes()?;
    for doc in &notes {
        let fm = &doc.frontmatter;
        nodes_map.insert(
            fm.id.clone(),
            GraphNodeResponse {
                id: fm.id.clone(),
                node_type: "note".to_string(),
                title: fm.title.clone(),
                status: fm.status.to_string(),
                priority: None,
                tags: vec![],
            },
        );
        for rel in &fm.related {
            edges.push(GraphEdgeResponse {
                source: fm.id.clone(),
                target: rel.clone(),
                relation: "related".to_string(),
            });
        }
    }

    // Collect maps
    let maps = project.list_maps()?;
    for doc in &maps {
        let fm = &doc.frontmatter;
        nodes_map.insert(
            fm.id.clone(),
            GraphNodeResponse {
                id: fm.id.clone(),
                node_type: "map".to_string(),
                title: fm.title.clone(),
                status: fm.status.to_string(),
                priority: None,
                tags: fm.tags.clone(),
            },
        );
        for rel in &fm.related {
            edges.push(GraphEdgeResponse {
                source: fm.id.clone(),
                target: rel.clone(),
                relation: "related".to_string(),
            });
        }
    }

    // Only keep edges where both source and target exist as nodes
    edges.retain(|e| nodes_map.contains_key(&e.source) && nodes_map.contains_key(&e.target));

    // Deduplicate symmetric related edges (keep source < target)
    let mut seen_related: HashSet<(String, String)> = HashSet::new();
    edges.retain(|e| {
        if e.relation == "related" {
            let key = if e.source < e.target {
                (e.source.clone(), e.target.clone())
            } else {
                (e.target.clone(), e.source.clone())
            };
            seen_related.insert(key)
        } else {
            true
        }
    });

    // If focused on a specific ID, filter to only reachable nodes within 2 hops
    if let Some(focus) = focus_id {
        let mut reachable: HashSet<String> = HashSet::new();
        reachable.insert(focus.to_string());

        // BFS 2 hops (both directions)
        for _ in 0..2 {
            let current: Vec<String> = reachable.iter().cloned().collect();
            for id in &current {
                for edge in &edges {
                    if edge.source == *id {
                        reachable.insert(edge.target.clone());
                    }
                    if edge.target == *id {
                        reachable.insert(edge.source.clone());
                    }
                }
            }
        }

        edges.retain(|e| reachable.contains(&e.source) && reachable.contains(&e.target));
        nodes_map.retain(|id, _| reachable.contains(id));
    }

    let nodes: Vec<GraphNodeResponse> = nodes_map.into_values().collect();
    Ok(GraphResponse { nodes, edges })
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::body::Body;
    use http_body_util::BodyExt;
    use std::fs;
    use tempfile::TempDir;
    use tower::ServiceExt;

    struct TestEnv {
        _tmp: TempDir,
        config_dir: PathBuf,
        project_dir: PathBuf,
        project_id: String,
        state: Arc<AppState>,
        app: tower::util::BoxCloneService<Request, Response, Infallible>,
    }

    fn setup_env(name: &str) -> TestEnv {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        fs::create_dir_all(&config_dir).unwrap();
        let project_dir = tmp.path().join(name);
        fs::create_dir_all(&project_dir).unwrap();
        Project::init(
            project_dir.join(".markplane"),
            name,
            "in-process serve tests",
        )
        .unwrap();
        let entry = serve_registry::register(&config_dir, &project_dir).unwrap();
        let state = AppState::new(config_dir.clone(), 4200, true, false);
        let app = build_app(state.clone());
        TestEnv {
            _tmp: tmp,
            config_dir,
            project_dir,
            project_id: entry.id,
            state,
            app,
        }
    }

    async fn oneshot_json(
        app: &tower::util::BoxCloneService<Request, Response, Infallible>,
        method: &str,
        uri: &str,
    ) -> (StatusCode, serde_json::Value) {
        oneshot_json_body(app, method, uri, None).await
    }

    async fn oneshot_json_body(
        app: &tower::util::BoxCloneService<Request, Response, Infallible>,
        method: &str,
        uri: &str,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        let req_body = if let Some(json) = body {
            builder = builder.header("content-type", "application/json");
            Body::from(serde_json::to_vec(&json).unwrap())
        } else {
            Body::empty()
        };
        let response = app
            .clone()
            .oneshot(builder.body(req_body).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value =
            serde_json::from_slice(&bytes).unwrap_or(serde_json::json!({}));
        (status, json)
    }

    #[tokio::test]
    async fn fs_lists_home_subdirs_hides_dotdirs_marks_markplane() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let config_dir = tmp.path().join("config");
        fs::create_dir_all(&config_dir).unwrap();
        fs::create_dir_all(home.join("visible")).unwrap();
        fs::create_dir_all(home.join(".hidden")).unwrap();
        fs::create_dir_all(home.join("a-project/.markplane")).unwrap();
        fs::write(home.join("file.txt"), "not a dir").unwrap();

        let state = AppState::with_home(config_dir, 4200, true, false, Some(home.clone()));
        let app = build_app(state);
        let (status, json) = oneshot_json(&app, "GET", "/api/fs").await;

        assert_eq!(status, StatusCode::OK);
        let data = &json["data"];
        assert_eq!(
            data["path"].as_str().unwrap(),
            home.canonicalize().unwrap().to_str().unwrap()
        );
        assert_eq!(
            data["home"].as_str().unwrap(),
            home.canonicalize().unwrap().to_str().unwrap()
        );
        assert_eq!(data["has_markplane"], false);

        let entries = data["entries"].as_array().unwrap();
        let names: Vec<&str> = entries
            .iter()
            .map(|e| e["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"visible"));
        assert!(names.contains(&"a-project"));
        assert!(!names.contains(&".hidden"));
        assert!(!names.contains(&"file.txt"));

        let project = entries
            .iter()
            .find(|e| e["name"] == "a-project")
            .unwrap();
        assert_eq!(project["has_markplane"], true);
        let visible = entries.iter().find(|e| e["name"] == "visible").unwrap();
        assert_eq!(visible["has_markplane"], false);
    }

    #[tokio::test]
    async fn fs_tilde_path_resolves_under_home() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let nested = home.join("docs/work");
        fs::create_dir_all(&nested).unwrap();
        let config_dir = tmp.path().join("config");
        fs::create_dir_all(&config_dir).unwrap();

        let state = AppState::with_home(config_dir, 4200, true, false, Some(home.clone()));
        let app = build_app(state);
        let (status, json) = oneshot_json(&app, "GET", "/api/fs?path=~/docs").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            json["data"]["path"].as_str().unwrap(),
            home.join("docs").canonicalize().unwrap().to_str().unwrap()
        );
        let entries = json["data"]["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["name"], "work");
        assert!(json["data"]["parent"].as_str().is_some());
    }

    #[tokio::test]
    async fn add_project_registers_and_symlink_dedupes_updating_last_opened() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        fs::create_dir_all(&config_dir).unwrap();
        let project_dir = tmp.path().join("real-proj");
        fs::create_dir_all(&project_dir).unwrap();
        Project::init(project_dir.join(".markplane"), "Real", "").unwrap();

        let state = AppState::new(config_dir.clone(), 4200, true, false);
        let app = build_app(state);

        let (status, json) = oneshot_json_body(
            &app,
            "POST",
            "/api/projects",
            Some(serde_json::json!({ "path": project_dir.to_string_lossy() })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let id = json["data"]["id"].as_str().unwrap().to_string();
        let first_opened = json["data"]["last_opened"].as_str().unwrap().to_string();

        std::thread::sleep(std::time::Duration::from_millis(15));
        let link = tmp.path().join("alias-proj");
        std::os::unix::fs::symlink(&project_dir, &link).unwrap();

        let (status, json) = oneshot_json_body(
            &app,
            "POST",
            "/api/projects",
            Some(serde_json::json!({ "path": link.to_string_lossy() })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["data"]["id"], id);
        assert!(json["data"]["last_opened"].as_str().unwrap() > first_opened.as_str());
        assert_eq!(serve_registry::list(&config_dir).unwrap().len(), 1);
    }

    #[tokio::test]
    async fn add_project_uninitialized_returns_conflict() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        fs::create_dir_all(&config_dir).unwrap();
        let bare = tmp.path().join("bare");
        fs::create_dir_all(&bare).unwrap();

        let state = AppState::new(config_dir, 4200, true, false);
        let app = build_app(state);
        let (status, json) = oneshot_json_body(
            &app,
            "POST",
            "/api/projects",
            Some(serde_json::json!({ "path": bare.to_string_lossy() })),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(json["error"]["code"], "project_not_initialized");
        assert!(
            json["error"]["path"]
                .as_str()
                .unwrap()
                .contains("bare")
        );
    }

    #[tokio::test]
    async fn init_api_creates_markplane_and_registers() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        fs::create_dir_all(&config_dir).unwrap();
        let bare = tmp.path().join("fresh-app");
        fs::create_dir_all(&bare).unwrap();

        let state = AppState::new(config_dir.clone(), 4200, true, false);
        let app = build_app(state);
        let (status, json) = oneshot_json_body(
            &app,
            "POST",
            "/api/init",
            Some(serde_json::json!({ "path": bare.to_string_lossy() })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["data"]["id"], "fresh-app");
        assert!(bare.join(".markplane/config.yaml").is_file());
        assert!(bare.join(".markplane/backlog").is_dir());
        // Default CLI init seeds starter content.
        let backlog = bare.join(".markplane/backlog/items");
        assert!(
            backlog.is_dir()
                && fs::read_dir(&backlog).unwrap().next().is_some(),
            "expected seeded starter tasks"
        );
        assert_eq!(serve_registry::list(&config_dir).unwrap().len(), 1);
    }

    #[tokio::test]
    async fn list_projects_orders_by_last_opened_and_marks_path_validity() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        fs::create_dir_all(&config_dir).unwrap();

        let older = tmp.path().join("older-proj");
        let newer = tmp.path().join("newer-proj");
        fs::create_dir_all(&older).unwrap();
        fs::create_dir_all(&newer).unwrap();
        Project::init(older.join(".markplane"), "Older", "").unwrap();
        Project::init(newer.join(".markplane"), "Newer", "").unwrap();
        let id_older = serve_registry::register(&config_dir, &older).unwrap().id;
        let id_newer = serve_registry::register(&config_dir, &newer).unwrap().id;
        serve_registry::touch_last_opened(&config_dir, &id_older).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        serve_registry::touch_last_opened(&config_dir, &id_newer).unwrap();

        // Invalidate newer project's path after registration.
        fs::remove_dir_all(newer.join(".markplane")).unwrap();

        let state = AppState::new(config_dir, 4200, true, false);
        let app = build_app(state);
        let (status, json) = oneshot_json(&app, "GET", "/api/projects").await;
        assert_eq!(status, StatusCode::OK);
        let data = json["data"].as_array().unwrap();
        assert_eq!(data.len(), 2);
        assert_eq!(data[0]["id"], id_newer);
        assert_eq!(data[0]["path_valid"], false);
        assert_eq!(data[1]["id"], id_older);
        assert_eq!(data[1]["path_valid"], true);
    }

    #[tokio::test]
    async fn remove_project_drops_registry_entry_and_runtime_without_deleting_files() {
        let env = setup_env("remove-me");
        let marker = env.project_dir.join("keep-me.txt");
        fs::write(&marker, "safe").unwrap();

        // Establish runtime so remove must release it.
        let (status, _) = oneshot_json(
            &env.app,
            "GET",
            &format!("/api/p/{}/summary", env.project_id),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(env.state.runtimes.lock().await.contains_key(&env.project_id));

        let (status, json) = oneshot_json(
            &env.app,
            "DELETE",
            &format!("/api/projects/{}", env.project_id),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["data"]["id"], env.project_id);

        let listed = serve_registry::list(&env.config_dir).unwrap();
        assert!(listed.iter().all(|e| e.id != env.project_id));
        assert!(!env.state.runtimes.lock().await.contains_key(&env.project_id));
        assert_eq!(fs::read_to_string(&marker).unwrap(), "safe");
        assert!(env.project_dir.join(".markplane").is_dir());

        let (status, json) = oneshot_json(
            &env.app,
            "GET",
            &format!("/api/p/{}/summary", env.project_id),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(json["error"]["code"], "project_not_found");
    }

    #[tokio::test]
    async fn rename_project_updates_display_name_keeps_id_and_url() {
        let env = setup_env("rename-me");
        let (status, json) = oneshot_json_body(
            &env.app,
            "PATCH",
            &format!("/api/projects/{}", env.project_id),
            Some(serde_json::json!({ "name": "Friendly Name" })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["data"]["id"], env.project_id);
        assert_eq!(json["data"]["name"], "Friendly Name");

        let entry = serve_registry::find(&env.config_dir, &env.project_id)
            .unwrap()
            .unwrap();
        assert_eq!(entry.name, "Friendly Name");
        assert_eq!(entry.id, env.project_id);

        let (status, list) = oneshot_json(&env.app, "GET", "/api/projects").await;
        assert_eq!(status, StatusCode::OK);
        let item = list["data"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == env.project_id)
            .unwrap();
        assert_eq!(item["name"], "Friendly Name");
    }

    #[tokio::test]
    async fn summary_via_project_prefix() {
        let env = setup_env("serve-test");
        let uri = format!("/api/p/{}/summary", env.project_id);
        let (status, json) = oneshot_json(&env.app, "GET", &uri).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["data"]["name"], "serve-test");
    }

    #[tokio::test]
    async fn tasks_and_ws_channels_isolated_across_two_projects() {
        let tmp = TempDir::new().unwrap();
        let config_dir = tmp.path().join("config");
        fs::create_dir_all(&config_dir).unwrap();

        let dir_a = tmp.path().join("proj-a");
        let dir_b = tmp.path().join("proj-b");
        fs::create_dir_all(&dir_a).unwrap();
        fs::create_dir_all(&dir_b).unwrap();
        let proj_a = Project::init(dir_a.join(".markplane"), "A", "").unwrap();
        let proj_b = Project::init(dir_b.join(".markplane"), "B", "").unwrap();
        proj_a
            .create_task(
                "Task A only",
                "feature",
                Priority::Medium,
                Effort::Small,
                None,
                None,
                vec![],
                None,
                None,
            )
            .unwrap();
        proj_b
            .create_task(
                "Task B only",
                "feature",
                Priority::Medium,
                Effort::Small,
                None,
                None,
                vec![],
                None,
                None,
            )
            .unwrap();
        let id_a = serve_registry::register(&config_dir, &dir_a).unwrap().id;
        let id_b = serve_registry::register(&config_dir, &dir_b).unwrap().id;

        let state = AppState::new(config_dir, 4200, true, false);
        let app = build_app(state.clone());

        let (status, list_a) =
            oneshot_json(&app, "GET", &format!("/api/p/{id_a}/tasks")).await;
        assert_eq!(status, StatusCode::OK);
        let titles_a: Vec<&str> = list_a["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["title"].as_str().unwrap())
            .collect();
        assert!(titles_a.contains(&"Task A only"));
        assert!(!titles_a.contains(&"Task B only"));

        let (status, list_b) =
            oneshot_json(&app, "GET", &format!("/api/p/{id_b}/tasks")).await;
        assert_eq!(status, StatusCode::OK);
        let titles_b: Vec<&str> = list_b["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["title"].as_str().unwrap())
            .collect();
        assert!(titles_b.contains(&"Task B only"));
        assert!(!titles_b.contains(&"Task A only"));

        let (tx_a, tx_b) = {
            let guard = state.runtimes.lock().await;
            (
                guard.get(&id_a).unwrap().ws_tx.clone(),
                guard.get(&id_b).unwrap().ws_tx.clone(),
            )
        };
        let mut rx_a = tx_a.subscribe();
        let mut rx_b = tx_b.subscribe();
        tx_a.send(r#"{"type":"sync_complete"}"#.to_string())
            .unwrap();
        assert!(rx_a.try_recv().unwrap().contains("sync_complete"));
        assert!(
            rx_b.try_recv().is_err(),
            "project B must not see A's WebSocket events"
        );
    }

    #[tokio::test]
    async fn unknown_project_id_returns_404_project_not_found() {
        let env = setup_env("missing-id");
        let (status, json) =
            oneshot_json(&env.app, "GET", "/api/p/does-not-exist/summary").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(json["error"]["code"], "project_not_found");
    }

    #[tokio::test]
    async fn unavailable_project_path_returns_409() {
        let env = setup_env("gone-path");
        // Remove the project directory after registration.
        fs::remove_dir_all(&env.project_dir).unwrap();
        let (status, json) = oneshot_json(
            &env.app,
            "GET",
            &format!("/api/p/{}/summary", env.project_id),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(json["error"]["code"], "project_unavailable");
        assert!(json["error"]["path"].as_str().unwrap().contains("gone-path"));
    }

    #[tokio::test]
    async fn docs_available_without_project_prefix() {
        let env = setup_env("docs-global");
        let (status, json) = oneshot_json(&env.app, "GET", "/api/docs").await;
        assert_eq!(status, StatusCode::OK);
        assert!(!json["data"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn legacy_backlog_path_redirects_to_landing() {
        let env = setup_env("legacy");
        let response = env
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/backlog/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response.headers().get("location").unwrap(),
            "/"
        );
    }

    #[tokio::test]
    async fn websocket_handshake_via_project_prefix() {
        let env = setup_env("ws-proj");
        let uri = format!("/api/p/{}/ws", env.project_id);
        let response = env
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&uri)
                    .header("connection", "upgrade")
                    .header("upgrade", "websocket")
                    .header("sec-websocket-version", "13")
                    .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UPGRADE_REQUIRED);
    }

    #[tokio::test]
    async fn config_via_project_prefix() {
        let env = setup_env("cfg-proj");
        let (status, json) = oneshot_json(
            &env.app,
            "GET",
            &format!("/api/p/{}/config", env.project_id),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["data"]["project"]["name"], "cfg-proj");
    }

    #[test]
    fn test_diff_vec_no_change() {
        let current = vec!["a".to_string(), "b".to_string()];
        let desired = vec!["a".to_string(), "b".to_string()];
        let (add, remove) = diff_vec(&current, &desired);
        assert!(add.is_empty());
        assert!(remove.is_empty());
    }

    #[test]
    fn test_diff_vec_additions_only() {
        let current = vec!["a".to_string()];
        let desired = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let (add, remove) = diff_vec(&current, &desired);
        assert_eq!(add, vec!["b", "c"]);
        assert!(remove.is_empty());
    }

    #[test]
    fn test_diff_vec_removals_only() {
        let current = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let desired = vec!["a".to_string()];
        let (add, remove) = diff_vec(&current, &desired);
        assert!(add.is_empty());
        assert_eq!(remove, vec!["b", "c"]);
    }

    #[test]
    fn test_diff_vec_mixed() {
        let current = vec!["a".to_string(), "b".to_string()];
        let desired = vec!["b".to_string(), "c".to_string()];
        let (add, remove) = diff_vec(&current, &desired);
        assert_eq!(add, vec!["c"]);
        assert_eq!(remove, vec!["a"]);
    }

    #[test]
    fn test_diff_vec_empty_to_some() {
        let current: Vec<String> = vec![];
        let desired = vec!["a".to_string(), "b".to_string()];
        let (add, remove) = diff_vec(&current, &desired);
        assert_eq!(add, vec!["a", "b"]);
        assert!(remove.is_empty());
    }

    #[test]
    fn test_diff_vec_some_to_empty() {
        let current = vec!["a".to_string(), "b".to_string()];
        let desired: Vec<String> = vec![];
        let (add, remove) = diff_vec(&current, &desired);
        assert!(add.is_empty());
        assert_eq!(remove, vec!["a", "b"]);
    }

    #[test]
    fn test_parse_section_bullets() {
        let section = "- Fog A\n* Fog B\n\n[placeholder]\n+ Fog C\nplain\n";
        assert_eq!(
            parse_section_bullets(section),
            vec!["Fog A", "Fog B", "Fog C", "plain"]
        );
    }

    #[test]
    fn test_ticket_question_answer_from_sections() {
        let body = "# Title\n\n## Question\n\nWhat next?\n\n## Answer\n\nGo left.\n";
        let (q, a) = ticket_question_answer(body);
        assert_eq!(q, "What next?");
        assert_eq!(a.as_deref(), Some("Go left."));
    }

    #[test]
    fn test_ticket_question_answer_preamble_fallback() {
        let body = "# Title\n\nShould we fork?\n\n## Comments\n\n### 2026-01-01 alice\n\nHi\n";
        let (q, a) = ticket_question_answer(body);
        assert_eq!(q, "Should we fork?");
        assert!(a.is_none());
    }
}
