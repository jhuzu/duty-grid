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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DutyPoint { pub id: String, pub plan_id: String, pub point_code: String, pub point_name: String, pub note: Option<String>, pub color: String, pub latitude: f64, pub longitude: f64, pub visible: bool }

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDutyPointInput { pub plan_id: String, pub point_code: String, pub point_name: String, pub note: Option<String>, pub color: String, pub latitude: f64, pub longitude: f64 }
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DutyRoute { pub id: String, pub plan_id: String, pub route_name: String, pub color: String, pub point_ids: Vec<String>, pub route_type: String, pub geometry: Option<Vec<[f64; 2]>> }
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDutyRouteInput { pub plan_id: String, pub route_name: String, pub color: String, pub point_ids: Vec<String> }
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateManualRouteInput { pub plan_id: String, pub route_name: String, pub color: String, pub geometry: Vec<[f64; 2]> }
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonRoute { pub id: String, pub route_name: String, pub color: String, pub geometry: Vec<[f64; 2]> }
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCommonRouteInput { pub route_name: String, pub color: String, pub geometry: Vec<[f64; 2]> }

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
    connection.execute_batch(include_str!("../migrations/0002_duty_points.sql")).map_err(|error| format!("無法套用勤務點位 migration：{error}"))?;
    connection.execute("INSERT OR IGNORE INTO schema_migrations(version) VALUES (?1)", [2]).map_err(|error| format!("無法記錄勤務點位 migration：{error}"))?;
    let has_point_color = connection.prepare("PRAGMA table_info(duty_points)")
        .map_err(|error| format!("無法檢查勤務點位欄位：{error}"))?
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| format!("無法讀取勤務點位欄位：{error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("無法讀取勤務點位欄位：{error}"))?
        .iter().any(|column| column == "color");
    if !has_point_color {
        connection.execute_batch(include_str!("../migrations/0003_duty_point_color.sql"))
            .map_err(|error| format!("無法套用勤務點位顏色 migration：{error}"))?;
    }
    connection.execute("INSERT OR IGNORE INTO schema_migrations(version) VALUES (?1)", [3]).map_err(|error| format!("無法記錄勤務點位顏色 migration：{error}"))?;
    connection.execute_batch(include_str!("../migrations/0004_duty_routes.sql")).map_err(|error| format!("無法套用勤務路線 migration：{error}"))?;
    connection.execute("INSERT OR IGNORE INTO schema_migrations(version) VALUES (?1)", [4]).map_err(|error| format!("無法記錄勤務路線 migration：{error}"))?;
    let has_geometry = connection.prepare("PRAGMA table_info(duty_routes)").map_err(|e| format!("無法檢查路線欄位：{e}"))?.query_map([], |r| r.get::<_, String>(1)).map_err(|e| format!("無法讀取路線欄位：{e}"))?.collect::<Result<Vec<_>, _>>().map_err(|e| format!("無法讀取路線欄位：{e}"))?.iter().any(|c| c == "geometry_json");
    if !has_geometry { connection.execute_batch(include_str!("../migrations/0005_manual_route_geometry.sql")).map_err(|e| format!("無法套用手繪路線 migration：{e}"))?; }
    connection.execute("INSERT OR IGNORE INTO schema_migrations(version) VALUES (?1)", [5]).map_err(|e| format!("無法記錄手繪路線 migration：{e}"))?;
    let duplicate_code_migration_done: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 6)", [], |row| row.get(0)).map_err(|e| format!("無法檢查點位編號 migration：{e}"))?;
    if !duplicate_code_migration_done {
        connection.execute_batch(include_str!("../migrations/0006_allow_duplicate_point_codes.sql")).map_err(|e| format!("無法允許重複點位編號：{e}"))?;
        connection.execute("INSERT INTO schema_migrations(version) VALUES (?1)", [6]).map_err(|e| format!("無法記錄點位編號 migration：{e}"))?;
    }
    let common_route_migration_done: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 7)", [], |row| row.get(0)).map_err(|e| format!("無法檢查常用路線 migration：{e}"))?;
    if !common_route_migration_done {
        connection.execute_batch(include_str!("../migrations/0007_common_routes.sql")).map_err(|e| format!("無法套用常用路線 migration：{e}"))?;
        connection.execute("INSERT INTO schema_migrations(version) VALUES (?1)", [7]).map_err(|e| format!("無法記錄常用路線 migration：{e}"))?;
    }
    Ok(())
}

pub fn list_duty_routes(path: &Path, plan_id: &str) -> Result<Vec<DutyRoute>, String> {
    let connection = open_database(path)?;
    let mut statement = connection.prepare("SELECT id, plan_id, route_name, color, route_type, geometry_json FROM duty_routes WHERE plan_id = ?1 ORDER BY created_at").map_err(|e| format!("無法讀取勤務路線：{e}"))?;
    let rows = statement.query_map([plan_id], |row| Ok((row.get::<_, String>(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get::<_, Option<String>>(5)?))).map_err(|e| format!("無法查詢勤務路線：{e}"))?;
    rows.map(|row| { let (id, plan_id, route_name, color, route_type, geometry_json) = row.map_err(|e| format!("無法讀取勤務路線：{e}"))?; let mut stops = connection.prepare("SELECT point_id FROM duty_route_stops WHERE route_id = ?1 ORDER BY stop_order").map_err(|e| format!("無法讀取路線點位：{e}"))?; let point_ids = stops.query_map([&id], |r| r.get(0)).map_err(|e| format!("無法讀取路線點位：{e}"))?.collect::<Result<Vec<String>, _>>().map_err(|e| format!("無法讀取路線點位：{e}"))?; let geometry = geometry_json.map(|json| serde_json::from_str(&json).map_err(|e| format!("無法讀取手繪路線：{e}"))).transpose()?; Ok(DutyRoute { id, plan_id, route_name, color, point_ids, route_type, geometry }) }).collect()
}

pub fn create_duty_route(path: &Path, input: CreateDutyRouteInput) -> Result<DutyRoute, String> {
    if input.route_name.trim().is_empty() || input.point_ids.len() < 2 { return Err("請輸入路線名稱，並至少選擇兩個勤務點位。".to_owned()); }
    let mut connection = open_database(path)?; let tx = connection.transaction().map_err(|e| format!("無法建立路線交易：{e}"))?;
    let id: String = tx.query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0)).map_err(|e| format!("無法建立路線識別碼：{e}"))?;
    tx.execute("INSERT INTO duty_routes(id, plan_id, route_name, color) VALUES (?1, ?2, ?3, ?4)", params![id, input.plan_id, input.route_name.trim(), input.color]).map_err(|e| format!("無法保存勤務路線：{e}"))?;
    for (index, point_id) in input.point_ids.iter().enumerate() { tx.execute("INSERT INTO duty_route_stops(route_id, point_id, stop_order) VALUES (?1, ?2, ?3)", params![id, point_id, index as i64]).map_err(|e| format!("無法保存路線點位：{e}"))?; }
    tx.commit().map_err(|e| format!("無法完成勤務路線保存：{e}"))?; Ok(DutyRoute { id, plan_id: input.plan_id, route_name: input.route_name.trim().to_owned(), color: input.color, point_ids: input.point_ids, route_type: "point_sequence".to_owned(), geometry: None })
}

pub fn create_manual_route(path: &Path, input: CreateManualRouteInput) -> Result<DutyRoute, String> { if input.route_name.trim().is_empty() || input.geometry.len() < 2 { return Err("請輸入路線名稱，並繪製至少兩個節點。".to_owned()); } let connection = open_database(path)?; let id: String = connection.query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0)).map_err(|e| format!("無法建立路線識別碼：{e}"))?; let geometry_json = serde_json::to_string(&input.geometry).map_err(|e| format!("無法保存手繪路線：{e}"))?; connection.execute("INSERT INTO duty_routes(id, plan_id, route_name, color, route_type, geometry_json) VALUES (?1,?2,?3,?4,'manual',?5)", params![id,input.plan_id,input.route_name.trim(),input.color,geometry_json]).map_err(|e| format!("無法保存手繪路線：{e}"))?; Ok(DutyRoute { id, plan_id: input.plan_id, route_name: input.route_name.trim().to_owned(), color: input.color, point_ids: vec![], route_type: "manual".to_owned(), geometry: Some(input.geometry) }) }

pub fn delete_duty_route(path: &Path, route_id: &str) -> Result<(), String> {
    let connection = open_database(path)?;
    let deleted = connection.execute("DELETE FROM duty_routes WHERE id = ?1", [route_id]).map_err(|e| format!("無法刪除勤務路線：{e}"))?;
    if deleted == 0 { return Err("找不到要刪除的勤務路線。".to_owned()); }
    Ok(())
}

pub fn update_duty_route_color(path: &Path, route_id: &str, color: &str) -> Result<(), String> {
    if !["red", "orange", "yellow", "green", "blue", "purple"].contains(&color) {
        return Err("不支援的路線顏色。".to_owned());
    }
    let connection = open_database(path)?;
    let updated = connection.execute("UPDATE duty_routes SET color = ?2 WHERE id = ?1", params![route_id, color]).map_err(|error| format!("無法更新路線顏色：{error}"))?;
    if updated == 0 { return Err("找不到要更新的勤務路線。".to_owned()); }
    Ok(())
}

pub fn update_duty_route_name(path: &Path, route_id: &str, route_name: &str) -> Result<(), String> {
    let route_name = route_name.trim();
    if route_name.is_empty() { return Err("路線名稱不可空白。".to_owned()); }
    let connection = open_database(path)?;
    let updated = connection.execute("UPDATE duty_routes SET route_name = ?2 WHERE id = ?1", params![route_id, route_name]).map_err(|error| format!("無法更新路線名稱：{error}"))?;
    if updated == 0 { return Err("找不到要更新的勤務路線。".to_owned()); }
    Ok(())
}

pub fn list_common_routes(path: &Path) -> Result<Vec<CommonRoute>, String> {
    let connection = open_database(path)?;
    let mut statement = connection.prepare("SELECT id, route_name, color, geometry_json FROM common_routes ORDER BY created_at DESC").map_err(|error| format!("無法讀取常用路線：{error}"))?;
    let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?))).map_err(|error| format!("無法查詢常用路線：{error}"))?;
    let common_routes = rows.map(|row| { let (id, route_name, color, geometry_json) = row.map_err(|error| format!("無法讀取常用路線：{error}"))?; let geometry = serde_json::from_str(&geometry_json).map_err(|error| format!("無法讀取常用路線座標：{error}"))?; Ok(CommonRoute { id, route_name, color, geometry }) }).collect();
    common_routes
}

pub fn create_common_route(path: &Path, input: CreateCommonRouteInput) -> Result<CommonRoute, String> {
    if input.route_name.trim().is_empty() || input.geometry.len() < 2 { return Err("請選擇至少含兩個節點的路線，再儲存為常用路線。".to_owned()); }
    let connection = open_database(path)?;
    let id: String = connection.query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0)).map_err(|error| format!("無法建立常用路線識別碼：{error}"))?;
    let geometry_json = serde_json::to_string(&input.geometry).map_err(|error| format!("無法保存常用路線：{error}"))?;
    connection.execute("INSERT INTO common_routes(id, route_name, color, geometry_json) VALUES (?1, ?2, ?3, ?4)", params![id, input.route_name.trim(), input.color, geometry_json]).map_err(|error| format!("無法保存常用路線：{error}"))?;
    Ok(CommonRoute { id, route_name: input.route_name.trim().to_owned(), color: input.color, geometry: input.geometry })
}

pub fn delete_common_route(path: &Path, route_id: &str) -> Result<(), String> {
    let connection = open_database(path)?;
    let deleted = connection.execute("DELETE FROM common_routes WHERE id = ?1", [route_id]).map_err(|error| format!("無法刪除常用路線：{error}"))?;
    if deleted == 0 { return Err("找不到要刪除的常用路線。".to_owned()); }
    Ok(())
}

pub fn list_duty_points(path: &Path, plan_id: &str) -> Result<Vec<DutyPoint>, String> {
    let connection = open_database(path)?;
    let mut statement = connection.prepare("SELECT id, plan_id, point_code, point_name, note, color, latitude, longitude, visible FROM duty_points WHERE plan_id = ?1 ORDER BY point_code").map_err(|e| format!("無法讀取勤務點位：{e}"))?;
    let points = statement.query_map([plan_id], |r| Ok(DutyPoint { id:r.get(0)?, plan_id:r.get(1)?, point_code:r.get(2)?, point_name:r.get(3)?, note:r.get(4)?, color:r.get(5)?, latitude:r.get(6)?, longitude:r.get(7)?, visible:r.get::<_, i64>(8)? != 0 })).map_err(|e| format!("無法查詢勤務點位：{e}"))?.collect::<Result<Vec<_>,_>>().map_err(|e| format!("無法讀取勤務點位資料：{e}"))?;
    Ok(points)
}

pub fn create_duty_point(path: &Path, input: CreateDutyPointInput) -> Result<DutyPoint, String> {
    if input.point_code.trim().is_empty() || input.point_name.trim().is_empty() { return Err("點位編號與名稱不可空白。".to_owned()); }
    let connection = open_database(path)?;
    let id: String = connection.query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0)).map_err(|e| format!("無法建立點位識別碼：{e}"))?;
    connection.execute("INSERT INTO duty_points(id, plan_id, point_code, point_name, note, color, latitude, longitude) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)", params![id,input.plan_id,input.point_code.trim(),input.point_name.trim(),input.note,input.color,input.latitude,input.longitude]).map_err(|e| format!("無法保存勤務點位：{e}"))?;
    connection.query_row("SELECT id, plan_id, point_code, point_name, note, color, latitude, longitude, visible FROM duty_points WHERE id=?1", [id], |r| Ok(DutyPoint { id:r.get(0)?, plan_id:r.get(1)?, point_code:r.get(2)?, point_name:r.get(3)?, note:r.get(4)?, color:r.get(5)?, latitude:r.get(6)?, longitude:r.get(7)?, visible:r.get::<_, i64>(8)? != 0 })).map_err(|e| format!("勤務點位已保存，但無法讀回資料：{e}"))
}

pub fn delete_duty_point(path: &Path, point_id: &str) -> Result<(), String> {
    let connection = open_database(path)?;
    let deleted = connection.execute("DELETE FROM duty_points WHERE id = ?1", [point_id]).map_err(|e| format!("無法刪除勤務點位：{e}"))?;
    if deleted == 0 { return Err("找不到要刪除的勤務點位。".to_owned()); }
    Ok(())
}

pub fn move_duty_point(path: &Path, point_id: &str, latitude: f64, longitude: f64) -> Result<(), String> {
    let connection = open_database(path)?;
    let updated = connection.execute("UPDATE duty_points SET latitude = ?2, longitude = ?3, updated_at = CURRENT_TIMESTAMP WHERE id = ?1", params![point_id, latitude, longitude]).map_err(|e| format!("無法移動勤務點位：{e}"))?;
    if updated == 0 { return Err("找不到要移動的勤務點位。".to_owned()); }
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
