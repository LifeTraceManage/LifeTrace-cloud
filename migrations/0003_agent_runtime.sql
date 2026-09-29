CREATE TABLE agent_sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    app_id TEXT NOT NULL,
    scopes_json TEXT NOT NULL,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','archived')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_message_at TEXT
);
CREATE INDEX idx_agent_sessions_user_access_updated
ON agent_sessions(user_id,app_id,scopes_json,status,updated_at DESC);

CREATE TABLE agent_runs (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('running','completed','failed','cancelled','awaiting_approval')),
    provider TEXT NOT NULL,
    model TEXT,
    prompt TEXT NOT NULL,
    error_code TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at TEXT
);
CREATE INDEX idx_agent_runs_user_created ON agent_runs(user_id,created_at DESC);
CREATE INDEX idx_agent_runs_session_created ON agent_runs(session_id,created_at DESC);

CREATE TABLE agent_messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
    run_id TEXT REFERENCES agent_runs(id) ON DELETE SET NULL,
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('user','assistant','system','tool')),
    content TEXT NOT NULL,
    provider TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_agent_messages_session_created ON agent_messages(session_id,created_at,id);
CREATE INDEX idx_agent_messages_user_created ON agent_messages(user_id,created_at DESC);

CREATE TABLE agent_tool_calls (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES agent_runs(id) ON DELETE CASCADE,
    session_id TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    tool_name TEXT NOT NULL,
    arguments_json TEXT NOT NULL DEFAULT '{}',
    status TEXT NOT NULL CHECK (status IN ('running','completed','failed','denied','awaiting_approval')),
    result_json TEXT,
    error_message TEXT,
    requires_approval INTEGER NOT NULL DEFAULT 0,
    started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at TEXT
);
CREATE INDEX idx_agent_tool_calls_run ON agent_tool_calls(run_id,started_at);
CREATE INDEX idx_agent_tool_calls_user ON agent_tool_calls(user_id,started_at DESC);

CREATE TABLE agent_approvals (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES agent_runs(id) ON DELETE CASCADE,
    session_id TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES cloud_users(id) ON DELETE CASCADE,
    tool_call_id TEXT REFERENCES agent_tool_calls(id) ON DELETE SET NULL,
    action_name TEXT NOT NULL,
    action_json TEXT NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending','approved','rejected','expired','cancelled')),
    requested_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    decided_at TEXT,
    decided_by_session_id TEXT,
    expires_at TEXT
);
CREATE INDEX idx_agent_approvals_user_status ON agent_approvals(user_id,status,requested_at DESC);
