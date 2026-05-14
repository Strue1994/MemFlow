use crate::error::ExecError;
use serde_json::Value;

pub fn execute_s3_upload(bucket: &str, key: &str, _body: &Value, _region: &str) -> Result<Value, ExecError> {
    tracing::info!(target: "executor.s3", bucket = %bucket, key = %key, "S3 upload (enable 'aws' feature)");
    Ok(serde_json::json!({"status": "logged", "bucket": bucket, "key": key}))
}

pub fn execute_s3_download(bucket: &str, key: &str) -> Result<Value, ExecError> {
    tracing::info!(target: "executor.s3", bucket = %bucket, key = %key, "S3 download (enable 'aws' feature)");
    Ok(serde_json::json!({"status": "logged", "bucket": bucket, "key": key}))
}
