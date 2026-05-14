#[cfg(test)]
mod tests {
    use compiler::ParseError;
    use compiler::parse_n8n_workflow;

    #[test]
    fn test_valid_workflow() {
        let json = r#"{"name":"Test","nodes":[{"id":"1","name":"HTTP","type":"n8n-nodes-base.httpRequest","parameters":{"method":"GET","url":"https://example.com"}}],"connections":{}}"#;
        let result = parse_n8n_workflow(json);
        assert!(result.is_ok(), "Failed: {:?}", result.err());
    }

    #[test]
    fn test_empty_json() {
        let json = "{}";
        let result = parse_n8n_workflow(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_nodes() {
        let json = r#"{"name": "Test"}"#;
        let result = parse_n8n_workflow(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_node_type() {
        let json = r#"{"name":"Test","nodes":[{"id":"1","name":"Test","type":"invalid"}],"connections":{}}"#;
        let result = parse_n8n_workflow(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_circular_dependency() {
        let json = r#"{"name":"Circular","nodes":[{"id":"1","name":"A","type":"n8n-nodes-base.noOp","parameters":{}},{"id":"2","name":"B","type":"n8n-nodes-base.noOp","parameters":{}}],"connections":{"1":{"main":[{"node":"2","type":"main"}]},"2":{"main":[{"node":"1","type":"main"}]}}}"#;
        let result = parse_n8n_workflow(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_http_node() {
        let json = r#"{"name":"HTTP","nodes":[{"id":"1","name":"HTTP","type":"n8n-nodes-base.httpRequest","parameters":{"method":"GET","url":"https://example.com"}}],"connections":{}}"#;
        let result = parse_n8n_workflow(json);
        assert!(result.is_ok());
    }
}
