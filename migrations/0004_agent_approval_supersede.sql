ALTER TABLE agent_approvals
ADD COLUMN superseded_by_approval_id TEXT REFERENCES agent_approvals(id) ON DELETE SET NULL;

ALTER TABLE agent_approvals
ADD COLUMN cancellation_reason TEXT;

CREATE INDEX idx_agent_approvals_superseded_by
ON agent_approvals(superseded_by_approval_id);
