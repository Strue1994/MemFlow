# FIX-07: Learning Loop State Persistence

## 问题描述

Learning engine 在重启后丢失所有学习状态。需要持久化学习循环的状态。

## 当前问题

`learning-engine/src/` 中的状态存储在内存中:
- 学习到的 pattern
- 调整的参数
- 策略更新历史
- 反馈数据

重启后需要重新学习。

## 修复方案

1. 添加 SQLite 持久化存储:
   ```rust
   use rusqlite::Connection;
   
   pub struct LearningStateStore {
       conn: Connection,
   }
   
   impl LearningStateStore {
       pub fn new(path: &str) -> Result<Self> {
           let conn = Connection::open(path)?;
           conn.execute(
               "CREATE TABLE IF NOT EXISTS learning_state (
                   key TEXT PRIMARY KEY,
                   value TEXT NOT NULL,
                   updated_at INTEGER NOT NULL
               )",
               [],
           )?;
           Ok(Self { conn })
       }
       
       pub fn save(&self, key: &str, value: &str) -> Result<()> {
           let now = std::time::SystemTime::now()
               .duration_since(SystemTime::UNIX_EPOCH)
               .unwrap()
               .as_secs() as i64;
           self.conn.execute(
               "INSERT OR REPLACE INTO learning_state VALUES (?1, ?2, ?3)",
               [key, value, &now.to_string()],
           )?;
           Ok(())
       }
       
       pub fn load(&self, key: &str) -> Result<Option<String>> {
           let mut stmt = self.conn.prepare(
               "SELECT value FROM learning_state WHERE key = ?1"
           )?;
           let result = stmt.query_row([key], |row| row.get(0));
           Ok(result.ok())
       }
   }
   ```

2. 在各 learning 模块中集成存储:
   - `scheduler.rs` - 调度状态
   - `decision.rs` - 决策历史
   - `param_optimizer.rs` - 优化参数

## 影响文件

- `learning-engine/src/scheduler.rs`
- `learning-engine/src/decision.rs`
- `learning-engine/src/param_optimizer.rs`
- `learning-engine/src/strategy_updater.rs`

## 验证方法

重启 learning engine，确认状态保留。

## 优先级

MEDIUM - 功能增强