use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OpenFlags};
use serde::Serialize;

pub struct AppState {
    pub database_path: PathBuf,
    pub road_reference_path: PathBuf,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DutyPlan {
    pub id: String,
    pub plan_name: String,
    pub duty_date: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub description: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDutyPlanInput {
    pub plan_name: String,
    pub duty_date: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoadReference {
    pub intersection_name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub road_name: String,
    pub cross_road_name: String,
}

pub fn initialize_state(app_data_dir: PathBuf, road_reference_path: PathBuf) -> Result<AppState, String> {
    fs::create_dir_all(&app_data_dir).map_err(|error| format!("無法建立應用程式資料目錄：{error}"))?;
    let database_path = app_data_dir.join("dutygrid.db");
    migrate(&database_path)?;
    Ok(AppState { database_path, road_reference_path })
}

fn open_database(path: &Path) -> Result<Connection, String> {
    let connection = Connection::open(path).map_err(|error| format!("無法開啟本機資料庫：{error}"))?;
    connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
        .map_err(|error| format!("無法設定本機資料庫：{error}"))?;
    Ok(connection)
}

pub fn migrate(path: &Path) -> Result<(), String> {
    let connection = open_database(path)?;
    connection.execute_batch(include_str!("../migrations/0001_initial.sql"))
        .map_err(|error| format!("無法套用資料庫 migration：{error}"))?;
    connection.execute(
        "INSERT OR IGNORE INTO schema_migrations(version) VALUES (?1)",
        [1],
    ).map_err(|error| format!("無法記錄資料庫 migration：{error}"))?;
    Ok(())
}

pub fn list_duty_plans(path: &Path) -> Result<Vec<DutyPlan>, String> {
    let connection = open_database(path)?;
    let mut statement = connection.prepare(
        "SELECT id, plan_name, duty_date, start_time, end_time, description, status, created_at, updated_at
         FROM duty_plans ORDER BY updated_at DESC, created_at DESC",
    ).map_err(|error| format!("無法讀取勤務計畫：{error}"))?;
    let rows = statement.query_map([], |row| Ok(DutyPlan {
        id: row.get(0)?, plan_name: row.get(1)?, duty_date: row.get(2)?, start_time: row.get(3)?,
        end_time: row.get(4)?, description: row.get(5)?, status: row.get(6)?, created_at: row.get(7)?, updated_at: row.get(8)?,
    })).map_err(|error| format!("無法查詢勤務計畫：{error}"))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|error| format!("無法讀取勤務計畫資料：{error}"))
}

pub fn create_duty_plan(path: &Path, input: CreateDutyPlanInput) -> Result<DutyPlan, String> {
    let plan_name = input.plan_name.trim();
    if plan_name.is_empty() { return Err("勤務計畫名稱不可空白。".to_owned()); }
    let connection = open_database(path)?;
    let id: String = connection.query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
        .map_err(|error| format!("無法建立勤務計畫識別碼：{error}"))?;
    connection.execute(
        "INSERT INTO duty_plans(id, plan_name, duty_date, start_time, end_time, description)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, plan_name, input.duty_date, input.start_time, input.end_time, input.description],
    ).map_err(|error| format!("無法保存勤務計畫：{error}"))?;
    connection.query_row(
        "SELECT id, plan_name, duty_date, start_time, end_time, description, status, created_at, updated_at
         FROM duty_plans WHERE id = ?1", [id], |row| Ok(DutyPlan {
            id: row.get(0)?, plan_name: row.get(1)?, duty_date: row.get(2)?, start_time: row.get(3)?,
            end_time: row.get(4)?, description: row.get(5)?, status: row.get(6)?, created_at: row.get(7)?, updated_at: row.get(8)?,
        }),
    ).map_err(|error| format!("勤務計畫已保存，但無法讀回資料：{error}"))
}

pub fn lookup_intersection(path: &Path, road_name: &str, cross_road_name: &str) -> Result<Vec<RoadReference>, String> {
    if !path.is_file() { return Err("找不到板橋路口參考資料庫。".to_owned()); }
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("無法開啟板橋路口參考資料庫：{error}"))?;
    let mut statement = connection.prepare(
        "SELECT i.intersection_name, i.lat, i.lon, first_road.road_name, cross_road.road_name
         FROM intersections i
         JOIN intersection_roads first_road ON first_road.intersection_id = i.id
         JOIN intersection_roads cross_road ON cross_road.intersection_id = i.id
         WHERE first_road.road_name = ?1 AND cross_road.road_name = ?2
         ORDER BY i.intersection_name LIMIT 20",
    ).map_err(|error| format!("無法準備路口查詢：{error}"))?;
    let rows = statement.query_map(params![road_name.trim(), cross_road_name.trim()], |row| Ok(RoadReference {
        intersection_name: row.get(0)?, latitude: row.get(1)?, longitude: row.get(2)?, road_name: row.get(3)?, cross_road_name: row.get(4)?,
    })).map_err(|error| format!("無法查詢板橋路口資料：{error}"))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|error| format!("無法讀取板橋路口資料：{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migration_and_plan_creation_persist() {
        let path = std::env::temp_dir().join(format!("dutygrid-test-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        migrate(&path).expect("migration should succeed");
        create_duty_plan(&path, CreateDutyPlanInput { plan_name: "板橋勤務測試".to_owned(), duty_date: None, start_time: None, end_time: None, description: None }).expect("plan should be saved");
        assert_eq!(list_duty_plans(&path).expect("plans should load").len(), 1);
        let _ = std::fs::remove_file(path);
    }
}
