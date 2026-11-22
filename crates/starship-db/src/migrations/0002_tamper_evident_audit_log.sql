-- Migration 0002: Tamper-Evident Hash Chained Audit Log (GAP-CLOSURE-PLAN.md §10.8)

ALTER TABLE audit_log ADD COLUMN prev_hash TEXT NOT NULL DEFAULT '0000000000000000000000000000000000000000000000000000000000000000';
ALTER TABLE audit_log ADD COLUMN entry_hash TEXT NOT NULL DEFAULT '0000000000000000000000000000000000000000000000000000000000000000';

-- Immutable Triggers preventing unauthorized tampering (GAP-CLOSURE-PLAN.md §10.8)
CREATE TRIGGER IF NOT EXISTS prevent_audit_log_update
BEFORE UPDATE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'audit_log is append-only and tamper-evident: UPDATE prohibited');
END;

CREATE TRIGGER IF NOT EXISTS prevent_audit_log_delete
BEFORE DELETE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'audit_log is append-only and tamper-evident: DELETE prohibited');
END;
