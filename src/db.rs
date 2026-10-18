use rusqlite::{Connection, Result as SqlResult};
use std::path::PathBuf;

pub fn db_path() -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("com.edgeagent.app");
    std::fs::create_dir_all(&path).ok();
    path.push("edge_agent.db");
    path
}

pub fn init_db() -> SqlResult<Connection> {
    let conn = Connection::open(db_path())?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS tools (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            tool_type       TEXT NOT NULL,
            name            TEXT NOT NULL,
            endpoint        TEXT NOT NULL,
            xaddr           TEXT,
            location        TEXT,
            username        TEXT,
            password        TEXT,
            channel         INTEGER DEFAULT 1,
            brand           TEXT,
            enabled         INTEGER DEFAULT 1,
            created_at      TEXT NOT NULL
        )",
        [],
    )?;

    // 兼容旧表：添加 channel 列
    let has_channel: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('tools') WHERE name = 'channel'",
        [], |row| row.get(0),
    )?;
    if has_channel == 0 {
        log::info!("tools 表添加 channel 列...");
        conn.execute("ALTER TABLE tools ADD COLUMN channel INTEGER DEFAULT 1", [])?;
    }

    // 兼容旧表：添加 brand 列
    let has_brand: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('tools') WHERE name = 'brand'",
        [], |row| row.get(0),
    )?;
    if has_brand == 0 {
        log::info!("tools 表添加 brand 列...");
        conn.execute("ALTER TABLE tools ADD COLUMN brand TEXT", [])?;
    }

    // 兼容旧表：添加 username/password 列（旧表只有 credential_id）
    let has_cred_col: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('tools') WHERE name = 'credential_id'",
        [], |row| row.get(0),
    )?;
    if has_cred_col > 0 {
        log::info!("检测到旧版 tools 表结构，迁移凭据到工具表...");
        conn.execute("BEGIN", [])?;
        conn.execute(
            "CREATE TABLE tools_new (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                tool_type       TEXT NOT NULL,
                name            TEXT NOT NULL,
                endpoint        TEXT NOT NULL,
                xaddr           TEXT,
                location        TEXT,
                username        TEXT,
                password        TEXT,
                channel         INTEGER DEFAULT 1,
                brand           TEXT,
                enabled         INTEGER DEFAULT 1,
                created_at      TEXT NOT NULL
            )", [],
        )?;
        conn.execute(
            "INSERT INTO tools_new (id, tool_type, name, endpoint, xaddr, location, username, password, enabled, created_at)
             SELECT id, tool_type, name, endpoint, xaddr, location, '', '', enabled, created_at FROM tools",
            [],
        )?;
        conn.execute("DROP TABLE tools", [])?;
        conn.execute("ALTER TABLE tools_new RENAME TO tools", [])?;
        conn.execute("COMMIT", [])?;
        log::info!("tools 表凭据迁移完成");
    }

    conn.execute(
        "CREATE TABLE IF NOT EXISTS scenarios (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            name            TEXT NOT NULL,
            enabled         INTEGER DEFAULT 1,
            pipeline        TEXT NOT NULL,
            created_at      TEXT NOT NULL,
            updated_at      TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS task_logs (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            scenario_id     INTEGER NOT NULL,
            scenario_name   TEXT NOT NULL,
            status          TEXT NOT NULL,
            started_at      TEXT NOT NULL,
            completed_at    TEXT,
            result          TEXT,
            error_msg       TEXT
        )",
        [],
    )?;

    // 兼容旧表：从 cameras 迁移到 tools
    migrate_cameras_to_tools(&conn)?;

    // 兼容旧表：旧 scenarios 表结构迁移
    migrate_old_scenarios(&conn)?;

    Ok(conn)
}

fn migrate_cameras_to_tools(conn: &Connection) -> SqlResult<()> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='cameras'",
        [],
        |row| row.get(0),
    )?;
    if count == 0 { return Ok(()); }

    let tool_count: i64 = conn.query_row("SELECT COUNT(*) FROM tools", [], |row| row.get(0))?;
    if tool_count > 0 { return Ok(()); }

    log::info!("检测到旧版 cameras 表，自动迁移到 tools 表...");
    conn.execute(
        "INSERT INTO tools (tool_type, name, endpoint, xaddr, location, username, password, channel, brand, enabled, created_at)
         SELECT 'onvif_camera', name, endpoint, xaddr, location, '', '', 1, NULL, enabled, created_at FROM cameras",
        [],
    )?;
    conn.execute("DROP TABLE cameras", [])?;
    log::info!("cameras -> tools 迁移完成");
    Ok(())
}

fn migrate_old_scenarios(conn: &Connection) -> SqlResult<()> {
    // 检查旧字段是否存在
    let has_old_cols: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('scenarios') WHERE name IN ('trigger_type', 'camera_ids')",
        [],
        |row| row.get(0),
    )?;
    if has_old_cols == 0 { return Ok(()); }

    log::info!("检测到旧版 scenarios 表结构，清理旧字段...");
    // SQLite 不支持 DROP COLUMN，需要重建表
    conn.execute("BEGIN", [])?;
    conn.execute(
        "CREATE TABLE scenarios_new (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            name            TEXT NOT NULL,
            enabled         INTEGER DEFAULT 1,
            pipeline        TEXT NOT NULL DEFAULT '[]',
            created_at      TEXT NOT NULL,
            updated_at      TEXT NOT NULL
        )",
        [],
    )?;
    conn.execute("INSERT INTO scenarios_new (id, name, enabled, pipeline, created_at, updated_at) SELECT id, name, enabled, '[]', created_at, updated_at FROM scenarios", [])?;
    conn.execute("DROP TABLE scenarios", [])?;
    conn.execute("ALTER TABLE scenarios_new RENAME TO scenarios", [])?;
    conn.execute("COMMIT", [])?;
    log::info!("scenarios 表结构迁移完成");
    Ok(())
}
