-- ClickHouse Materialized Views for Query Optimization
-- Run this SQL file to create materialized views for learning engine analytics

-- Daily workflow statistics materialized view
CREATE MATERIALIZED VIEW IF NOT EXISTS workflow_daily_stats
ENGINE = SummingMergeTree()
ORDER BY (workflow_id, date)
PARTITION BY toYYYYMM(date)
AS SELECT
    toDate(timestamp) AS date,
    workflow_id,
    count() AS total_calls,
    sum(success) AS success_calls,
    sum(error) AS error_calls,
    avg(duration_ms) AS avg_duration_ms,
    quantile(0.95)(duration_ms) AS p95_duration_ms,
    quantile(0.99)(duration_ms) AS p99_duration_ms,
    sum(input_tokens) AS input_tokens,
    sum(output_tokens) AS output_tokens,
    sum(cost_usd) AS cost_usd
FROM execution_logs
GROUP BY date, workflow_id;

-- Hourly workflow statistics for more granular analysis
CREATE MATERIALIZED VIEW IF NOT EXISTS workflow_hourly_stats
ENGINE = SummingMergeTree()
ORDER BY (workflow_id, hour)
AS SELECT
    toStartOfHour(toDateTime(timestamp)) AS hour,
    workflow_id,
    count() AS total_calls,
    sum(success) AS success_calls,
    avg(duration_ms) AS avg_duration_ms,
    sum(input_tokens) AS input_tokens,
    sum(output_tokens) AS output_tokens
FROM execution_logs
GROUP BY hour, workflow_id;

-- Weekly version performance comparison
CREATE MATERIALIZED VIEW IF NOT EXISTS version_performance_weekly
ENGINE = SummingMergeTree()
ORDER BY (workflow_id, version, week)
AS SELECT
    toStartOfWeek(toDate(timestamp)) AS week,
    workflow_id,
    version,
    count() AS total_executions,
    sum(success) AS successful_executions,
    avg(duration_ms) AS avg_duration_ms,
    sum(input_tokens) AS total_input_tokens,
    sum(output_tokens) AS total_output_tokens
FROM execution_logs
GROUP BY week, workflow_id, version;

-- Error pattern aggregation for root cause analysis
CREATE MATERIALIZED VIEW IF NOT EXISTS error_patterns_daily
ENGINE = SummingMergeTree()
ORDER BY (workflow_id, error_type, date)
AS SELECT
    toDate(timestamp) AS date,
    workflow_id,
    error_type,
    count() AS occurrence_count,
    uniqExact(workflow_run_id) AS unique_runs
FROM execution_errors
GROUP BY date, workflow_id, error_type;

-- Indexes for faster querying
ALTER TABLE execution_logs ADD INDEX idx_workflow_id workflow_id TYPE bloom_filter GRANULARITY 1;
ALTER TABLE execution_logs ADD INDEX idx_timestamp timestamp TYPE minmax GRANULARITY 4;
ALTER TABLE execution_logs ADD INDEX idx_version version TYPE set(100) GRANULARITY 4;

-- Example queries using materialized views

-- Get daily success rate (fast via materialized view)
-- SELECT 
--     date,
--     workflow_id,
--     success_calls / total_calls AS success_rate
-- FROM workflow_daily_stats
-- WHERE date >= today() - INTERVAL 7 DAY;

-- Get P95 latency trend
-- SELECT 
--     date,
--     workflow_id,
--     p95_duration_ms
-- FROM workflow_daily_stats
-- ORDER BY date DESC
-- LIMIT 100;

-- Get error distribution
-- SELECT 
--     error_type,
--     sum(occurrence_count) AS total
-- FROM error_patterns_daily
-- WHERE date >= today() - INTERVAL 30 DAY
-- GROUP BY error_type
-- ORDER BY total DESC;