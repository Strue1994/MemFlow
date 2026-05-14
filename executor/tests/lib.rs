#[cfg(test)]
mod tests {
    use executor::environment::Environment;
    use executor::error::ExecError;

    #[test]
    fn test_environment_set_get() {
        let mut env = Environment::new();
        env.set("name", serde_json::json!("test"));
        let value = env.get("name").unwrap();
        assert_eq!(value, "test");
    }

    #[test]
    fn test_environment_not_found() {
        let env = Environment::new();
        let result = env.get("missing");
        assert!(result.is_err());
    }

    #[test]
    fn test_environment_clear() {
        let mut env = Environment::new();
        env.set("a", serde_json::json!(1));
        env.set("b", serde_json::json!(2));
        env.clear();
        assert!(env.get("a").is_err());
        assert!(env.get("b").is_err());
    }

    #[tokio::test]
    async fn test_memory_pool_acquire_release() {
        let pool = executor::memory_pool::MemoryPool::new(5);
        let env = pool.acquire().await;
        let result = env.get("test");
        assert!(result.is_err());
    }
}
