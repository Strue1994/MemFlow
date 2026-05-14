use crate::error::ExecError;
use serde_json::Value;

pub fn execute_github_create_issue(owner: &str, repo: &str, title: &str, body: &str, labels: &Option<Vec<String>>, token: &str) -> Result<Value, ExecError> {
    let client = reqwest::blocking::Client::new();
    let mut payload = serde_json::json!({"title": title, "body": body});
    if let Some(l) = labels { payload["labels"] = serde_json::json!(l); }
    let resp = client.post(&format!("https://api.github.com/repos/{}/{}/issues", owner, repo))
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "memflow")
        .json(&payload).send()
        .map_err(|e| ExecError::HttpError(format!("GitHub: {}", e)))?;
    if !resp.status().is_success() { return Err(ExecError::HttpError(format!("GitHub HTTP {}", resp.status()))); }
    let r: Value = resp.json().map_err(|e| ExecError::HttpError(format!("JSON: {}", e)))?;
    Ok(serde_json::json!({"status": "created", "url": r["html_url"], "number": r["number"]}))
}
