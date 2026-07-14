ALTER TABLE job_queue ADD COLUMN idempotency_key TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS job_queue_idempotency_key_index
    ON job_queue (kind, idempotency_key)
    WHERE idempotency_key IS NOT NULL
      AND status IN ('pending', 'processing');
