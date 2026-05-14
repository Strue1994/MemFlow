#[cfg(test)]
mod tests {
    use learning_engine::feedback::Feedback;

    #[test]
    fn test_feedback_creation() {
        let feedback = Feedback {
            workflow_id: "test_wf".to_string(),
            user_id: "user1".to_string(),
            accepted: true,
            original: r#"{"nodes":[]}"#.to_string(),
            modified: None,
            score: Some(5),
            created_at: 1234567890,
        };

        assert_eq!(feedback.workflow_id, "test_wf");
        assert_eq!(feedback.accepted, true);
    }

    #[test]
    fn test_pattern_miner() {
        use learning_engine::lib::PatternMiner;

        let miner = PatternMiner::new(2);

        assert_eq!(miner.min_frequency, 2);
    }

    #[test]
    fn test_ab_tester() {
        use learning_engine::lib::{ABTester, ExecutionLog};

        let tester = ABTester::new(5);

        assert_eq!(tester.min_samples, 5);
    }

    #[test]
    fn test_scheduler_default_config() {
        use learning_engine::scheduler::ScheduleConfig;
        use learning_engine::scheduler::ScheduleMode;

        let config = ScheduleConfig::default();

        assert!(config.enabled);
        assert!(matches!(config.mode, ScheduleMode::Interval { .. }));
    }

    #[test]
    fn test_decision_maker_default_thresholds() {
        use learning_engine::decision::DecisionThresholds;

        let thresholds = DecisionThresholds::default();

        assert_eq!(thresholds.min_success_rate, 0.95);
        assert_eq!(thresholds.max_error_rate, 0.05);
    }

    #[test]
    fn test_monitor_alert_thresholds() {
        use learning_engine::monitor::AlertThresholds;

        let thresholds = AlertThresholds::default();

        assert_eq!(thresholds.error_rate_threshold, 0.05);
        assert_eq!(thresholds.success_rate_min_percent, 95.0);
    }

    #[test]
    fn test_safety_whitelist() {
        use learning_engine::safety::{EntityType, SafetyWhitelist, WhitelistEntry};

        let whitelist = SafetyWhitelist::new();

        let entry = WhitelistEntry::new(
            EntityType::Workflow("test_wf".to_string()),
            "test_value".to_string(),
            "Test reason".to_string(),
            "system".to_string(),
        );

        assert!(!entry.is_expired());
    }
}
