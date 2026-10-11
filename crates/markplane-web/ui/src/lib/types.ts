export type TaskStatus = string;
export type StatusCategory = "draft" | "backlog" | "planned" | "active" | "completed" | "cancelled";
export type EpicStatus = "now" | "next" | "later" | "done";
export type PlanStatus = "draft" | "approved" | "in-progress" | "done";
export type NoteStatus = "draft" | "active" | "archived";
export type MapStatus = "active" | "done" | "abandoned";
export type Priority = "critical" | "high" | "medium" | "low" | "someday";
export type TaskType = string;
export type Effort = "xs" | "small" | "medium" | "large" | "xl";
export type NoteType = string;
export type TicketState = "decided" | "claimed" | "frontier" | "blocked" | "out-of-scope";

export type TaskWorkflow = Record<StatusCategory, string[]>;

export interface ProjectInfo {
  name: string;
  description: string;
}

export interface ContextConfig {
  token_budget: number;
  recent_days: number;
  auto_generate: boolean;
}

export interface DispatcherStageConfig {
  model: string;
  timeout_minutes: number;
}

export interface DispatcherConfig {
  push_after_merge: boolean;
  stages: {
    implement: DispatcherStageConfig;
    fix: DispatcherStageConfig;
    merge: DispatcherStageConfig;
  };
}

export interface ProjectConfig {
  project: ProjectInfo;
  context: ContextConfig;
  documentation_paths: string[];
  task_types: string[];
  note_types: string[];
  workflows: {
    task: TaskWorkflow;
  };
  dispatcher: DispatcherConfig;
}

export interface UpdateDispatcherStageRequest {
  model?: string;
  timeout_minutes?: number;
}

export interface UpdateDispatcherRequest {
  push_after_merge?: boolean;
  stages?: {
    implement?: UpdateDispatcherStageRequest;
    fix?: UpdateDispatcherStageRequest;
    merge?: UpdateDispatcherStageRequest;
  };
}

/** Matches the backend's UpdateConfigRequest — partial at every level. */
export interface UpdateConfigRequest {
  project?: Partial<ProjectInfo>;
  context?: Partial<ContextConfig>;
  documentation_paths?: string[];
  task_types?: string[];
  note_types?: string[];
  workflows?: { task?: TaskWorkflow };
  dispatcher?: UpdateDispatcherRequest;
}

export interface DispatcherConcurrency {
  concurrency: number;
  in_use: number;
}

export interface AgentModel {
  slug: string;
  label: string;
}

export interface DispatcherLockHolder {
  pid: number;
  hostname: string;
  serve_url: string;
  config_dir: string;
  project_path: string;
  acquired_at: string;
}

export interface DispatcherLockStatus {
  status: "free" | "held_by_us" | "held_by_other" | "unavailable";
  holder?: DispatcherLockHolder;
}

export interface DispatcherActiveRun {
  task_id: string;
  title: string;
  status: string;
  stage?: string | null;
  stage_started_at?: string | null;
  queue_position?: number | null;
}

export interface DispatcherProjectStatus {
  id: string;
  name: string;
  path: string;
  scan_enabled: boolean;
  /** Present when Scan was auto-closed; cleared on manual reopen. */
  auto_close_reason?: string | null;
  lock: DispatcherLockStatus;
  active_runs: DispatcherActiveRun[];
}

export interface DispatcherStatus {
  concurrency: number;
  slots_used: number;
  running: number;
  merge_queued: number;
  projects: DispatcherProjectStatus[];
}

export interface ScanSwitch {
  enabled: boolean;
}

export interface Comment {
  date: string;
  author: string;
  body: string;
}

export interface Task {
  id: string;
  title: string;
  status: TaskStatus;
  priority: Priority;
  type: TaskType;
  effort: Effort;
  epic: string | null;
  plan: string | null;
  /** Parent Map ID when this Task is a Decision ticket. Omitted when unset. */
  map?: string | null;
  depends_on: string[];
  blocks: string[];
  related: string[];
  assignee: string | null;
  tags: string[];
  position: string | null;
  created: string;
  updated: string;
  body: string;
  /** Body without the Comments section. */
  description: string;
  comments: Comment[];
}

/** Pipeline Stage for a Dispatcher Run. */
export type Stage = "implement" | "fix" | "merge";

export type Trigger = "initial" | "resume" | "continue";

export type AttemptResult =
  | "ok"
  | "failed"
  | "timeout"
  | "interrupted"
  | "resume_mismatch";

export type RunStatus =
  | "running"
  | "merge-queued"
  | "succeeded"
  | "failed"
  | "stopped";

export interface Usage {
  inputTokens: number | null;
  outputTokens: number | null;
  cacheReadTokens: number | null;
  cacheWriteTokens: number | null;
}

/** One agent process invocation within a Run. */
export interface Attempt {
  stage: Stage;
  model: string;
  timeout_secs?: number | null;
  session_id?: string | null;
  workspace: string;
  transcript_path?: string | null;
  pid?: number | null;
  process_group_id?: number | null;
  agent_config_dir?: string | null;
  trigger: Trigger;
  started_at: string;
  ended_at?: string | null;
  exit_code?: number | null;
  verdict?: string | null;
  result?: AttemptResult | null;
  usage?: Usage | null;
  duration_api_ms?: number | null;
  stderr_tail?: string | null;
}

/** One Run of an Implementation ticket. */
export interface Run {
  seq: number;
  status: RunStatus;
  current_stage?: Stage | null;
  worktree: string;
  branch: string;
  base_sha: string;
  merge_target: string;
  merge_sha?: string | null;
  pushed: boolean;
  started_at: string;
  ended_at?: string | null;
  failure_reason?: string | null;
  worktree_removed_at?: string | null;
  branch_removed_at?: string | null;
  branch_head_at_removal?: string | null;
  /** Skip reason: uncommitted_changes | unmerged_commits | orphan_running | not_a_worktree */
  cleanup_blocked?: string | null;
  cleanup_trigger?: string | null;
  attempts: Attempt[];
}

/** Shape of `.markplane/.runs/<TASK-id>.json` / `GET /api/runs/{task_id}` when present. */
export interface RunRecord {
  runs: Run[];
}

/** Server display status from `GET /api/runs` (client may additionally derive `stopping`). */
export type RunDisplayStatus =
  | "running"
  | "merge-queued"
  | "interrupted"
  | "running elsewhere"
  | "failed"
  | "stopped"
  | "succeeded"
  | "stopping";

/** Latest-Run summary from `GET /api/runs`. */
export interface RunSummary {
  task_id: string;
  seq: number;
  status: string;
  stage?: string | null;
  stage_started_at?: string | null;
  started_at: string;
  queue_position?: number | null;
  failure_reason?: string | null;
  ended_at?: string | null;
  model?: string | null;
  timeout_secs?: number | null;
  stage_attempt_count?: number | null;
  /** Live orphan agent PID when the last attempt's process is still running. */
  orphan_pid?: number | null;
}

export interface ResumeOneResponse {
  task_id: string;
  status: string;
}

export interface ResumeAllResponse {
  resumed: string[];
  remaining: string[];
  stopped_early: boolean;
  reason?: string | null;
}

/** `GET /api/runs/{task_id}/cleanup` confirmation preview. */
export interface CleanupPreview {
  worktree: string;
  worktree_present: boolean;
  uncommitted_files: number;
  branch: string;
  branch_present: boolean;
  unmerged_commits: number;
  branch_head?: string | null;
  orphan_pid?: number | null;
  eligible: boolean;
}

export interface CleanupOneResponse {
  task_id: string;
  worktree_removed_at?: string | null;
  branch_removed_at?: string | null;
  cleanup_blocked?: string | null;
  cleanup_trigger?: string | null;
}

export type TranscriptBlock =
  | { type: "text"; text: string }
  | { type: "tool_use"; name: string; input: Record<string, unknown> };

export interface TranscriptMessage {
  idx: number;
  role: "user" | "assistant" | string;
  blocks: TranscriptBlock[];
}

/** `GET /api/runs/{task_id}/sessions/{session_id}` — ok or unavailable, never a hard error. */
export type SessionTranscript =
  | {
      status: "ok";
      path: string;
      messages: TranscriptMessage[];
      ended: string | null;
    }
  | { status: "unavailable"; path: string; reason: string };

export interface Epic {
  id: string;
  title: string;
  status: EpicStatus;
  priority: Priority;
  started: string | null;
  target: string | null;
  related: string[];
  tags: string[];
  created: string;
  updated: string;
  body: string;
  task_count: number;
  done_count: number;
  progress: number;
  status_breakdown: Record<string, number>;
}

export interface Plan {
  id: string;
  title: string;
  status: PlanStatus;
  implements: string[];
  related: string[];
  created: string;
  updated: string;
  body: string;
}

export interface Note {
  id: string;
  title: string;
  status: NoteStatus;
  type: NoteType;
  related: string[];
  tags: string[];
  created: string;
  updated: string;
  body: string;
}

export interface MapListItem {
  id: string;
  title: string;
  status: MapStatus;
  specs: string[];
  related: string[];
  tags: string[];
  created: string;
  updated: string;
  decided_count: number;
  total_count: number;
  frontier_count: number;
}

export interface MapTicket {
  id: string;
  title: string;
  type: string;
  status: string;
  assignee: string | null;
  depends_on: string[];
  position: string | null;
  question: string;
  answer: string | null;
}

export interface MapDetail {
  id: string;
  title: string;
  status: MapStatus;
  specs: string[];
  related: string[];
  tags: string[];
  created: string;
  updated: string;
  body: string;
  destination: string;
  not_yet_specified: string[];
  out_of_scope: string[];
  decided_count: number;
  total_count: number;
  frontier_count: number;
  tickets: MapTicket[];
}

export interface ProjectSummary {
  name: string;
  description: string;
  counts: {
    total: number;
    active: number;
    planned: number;
    backlog: number;
    draft: number;
    completed: number;
    blocked: number;
  };
  now_epics: Epic[];
  in_progress_tasks: Task[];
  blocked_tasks: Task[];
  recent_completions: Task[];
  next_up_tasks: Task[];
  context_summary: string | null;
  context_last_synced: string | null;
}

export interface CreateTaskRequest {
  title: string;
  type?: TaskType;
  priority?: Priority;
  effort?: Effort;
  epic?: string;
  tags?: string[];
}

export interface CreateEpicRequest {
  title: string;
  priority?: Priority;
}

export interface CreatePlanRequest {
  title: string;
  task_id?: string;
}

export interface CreateNoteRequest {
  title: string;
  type?: NoteType;
  tags?: string[];
}

export interface UpdateTaskRequest {
  title?: string;
  status?: TaskStatus;
  priority?: Priority;
  effort?: Effort;
  type?: TaskType;
  tags?: string[];
  epic?: string;
  plan?: string;
  assignee?: string;
  position?: string;
  depends_on?: string[];
  blocks?: string[];
  related?: string[];
  body?: string;
  /** Replace non-comment body; preserves on-disk Comments. */
  description?: string;
}

export interface UpdateEpicRequest {
  title?: string;
  status?: EpicStatus;
  priority?: Priority;
  tags?: string[];
  started?: string;
  target?: string;
  related?: string[];
  body?: string;
}

export interface UpdatePlanRequest {
  title?: string;
  status?: PlanStatus;
  related?: string[];
  body?: string;
}

export interface UpdateNoteRequest {
  title?: string;
  status?: NoteStatus;
  type?: NoteType;
  tags?: string[];
  related?: string[];
  body?: string;
}

export interface ApiResponse<T> {
  data: T;
}

export interface ApiListResponse<T> {
  data: T[];
  meta: { total: number };
}

export interface GraphNode {
  id: string;
  type: "task" | "epic" | "plan" | "note" | "map";
  title: string;
  status: string;
  priority?: string;
  tags?: string[];
}

export interface GraphEdge {
  source: string;
  target: string;
  relation: "blocks" | "depends_on" | "implements" | "epic" | "related";
}

export interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export interface SearchResult {
  id: string;
  entity_type: "task" | "epic" | "plan" | "note" | "map";
  title: string;
  status: string;
  priority?: string;
  snippet: string;
  score: number;
  archived: boolean;
}

export interface DocMeta {
  slug: string;
  title: string;
}

export interface DocContent {
  slug: string;
  title: string;
  content: string;
}

export type WsEvent =
  | { type: "file_changed"; entity: "task" | "epic" | "plan" | "note" | "map"; id: string; action: "created" | "modified" | "deleted" }
  | { type: "config_changed" }
  | { type: "sync_complete" }
  | { type: "doc_changed"; slug: string }
  | { type: "connected"; version: string }
  | { type: "dispatcher_changed" }
  | {
      type: "run_changed";
      task_id: string;
      seq: number;
      status: string;
      stage: string;
    };
