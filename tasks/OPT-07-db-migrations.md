# OPT-07: Add Database Migration System

## 目标

添加数据库迁移系统，支持 schema 版本管理。

## 当前状态

缺少正式的迁移系统。

## 实现方案

1. **创建迁移框架**
   ```rust
   use semver::Version;
   
   pub struct Migration {
       pub version: Version,
       pub name: String,
       pub up: fn(&Connection) -> Result<()>,
       pub down: fn(&Connection) -> Result<()>,
   }
   
   pub struct MigrationRunner {
       migrations: Vec<Migration>,
   }
   
   impl MigrationRunner {
       pub fn new() -> Self {
           Self { migrations: Vec::new() }
       }
       
       pub fn register(mut self, m: Migration) -> Self {
           self.migrations.push(m);
           self
       }
       
       pub fn migrate(&self, conn: &Connection) -> Result<()> {
           let current = self.get_current_version(conn)?;
           
           for m in &self.migrations {
               if m.version > current {
                   log::info!("Running migration: {}", m.name);
                   (m.up)(conn)?;
                   self.set_version(conn, &m.version)?;
               }
           }
           Ok(())
       }
       
       pub fn rollback(&self, conn: &Connection, steps: u32) -> Result<()> {
           let current = self.get_current_version(conn)?;
           
           for m in self.migrations.iter().rev() {
               if m.version <= current && steps > 0 {
                   log::info!("Rolling back: {}", m.name);
                   (m.down)(conn)?;
                   self.set_version(conn, &(m.version - Version::new(1, 0, 0)))?;
               }
           }
           Ok(())
       }
   }
   ```

2. **创建迁移文件**
   ```
   migrations/
   ├── V1__initial_schema.sql
   ├── V2__add_workflow_version.sql
   ├── V3__add_learning_data.sql
   └── V4__add_audit_logs.sql
   ```

3. **集成到 CLI**
   ```bash
   memflow db migrate        # 运行迁移
   memflow db rollback --to V1 # 回滚
   memflow db status         # 查看状态
   ```

## 影响文件

- 新建 `executor/src/migrations.rs`
- CLI 添加命令

## 验证方法

运行迁移，数据库 schema 正确。

## 优先级

MEDIUM - 功能增强