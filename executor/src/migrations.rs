use rusqlite::Connection;
use std::path::Path;

const MIGRATIONS: &[(&str, &str)] = &[
    ("001_init", "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at TEXT NOT NULL DEFAULT (datetime('now')));"),
];

pub fn run_migrations(db_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::open(db_path)?;

    conn.execute("CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at TEXT NOT NULL DEFAULT (datetime('now')))", [])?;

    let applied: Vec<i32> = {
        let mut stmt = conn.prepare("SELECT version FROM schema_migrations ORDER BY version")?;
        let rows = stmt.query_map([], |row| row.get::<_, i32>(0))?;
        let mut v = Vec::new();
        for row in rows {
            if let Ok(ver) = row {
                v.push(ver);
            }
        }
        v
    };

    for (i, (name, sql)) in MIGRATIONS.iter().enumerate() {
        let version = (i + 1) as i32;
        if applied.contains(&version) { continue; }
        println!("Migration {}: {}", version, name);
        conn.execute_batch(sql)?;
        conn.execute("INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)", rusqlite::params![version, name])?;
    }

    Ok(())
}
