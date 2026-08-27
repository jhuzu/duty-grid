use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use calamine::{Reader, Xlsx};
use rusqlite::{params, Connection};
use serde::Serialize;

pub struct AppState {
    pub database_path: PathBuf,
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
pub struct DutyPoint { pub id: String, pub plan_id: String, pub point_code: String, pub point_name: String, pub note: Option<String>, pub color: String, pub point_type: String, pub latitude: f64, pub longitude: f64, pub visible: bool }

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDutyPointInput { pub plan_id: String, pub point_code: String, pub point_name: String, pub note: Option<String>, pub color: String, pub point_type: String, pub latitude: f64, pub longitude: f64 }
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DutyRoute { pub id: String, pub plan_id: String, pub route_name: String, pub color: String, pub point_ids: Vec<String>, pub route_type: String, pub geometry: Option<Vec<[f64; 2]>>, pub line_style: String }
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDutyRouteInput { pub plan_id: String, pub route_name: String, pub color: String, pub point_ids: Vec<String>, pub line_style: String }
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateManualRouteInput { pub plan_id: String, pub route_name: String, pub color: String, pub geometry: Vec<[f64; 2]> }
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonRoute { pub id: String, pub route_name: String, pub color: String, pub geometry: Vec<[f64; 2]> }
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCommonRouteInput { pub route_name: String, pub color: String, pub geometry: Vec<[f64; 2]> }
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Personnel { pub id: String, pub personnel_code: String, pub radio_code: String, pub name: String, pub title: String, pub unit: String, pub phone: String, pub is_sample: bool }
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonnelAssignment { pub id: String, pub plan_id: String, pub personnel_id: String, pub duty_point_id: Option<String>, pub assigned_unit: String, pub assigned_title: String }
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePersonnelAssignmentInput { pub plan_id: String, pub personnel_id: String, pub duty_point_id: Option<String>, pub assigned_unit: String, pub assigned_title: String }
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPersonnelInput { pub file_name: String, pub file_data: Vec<u8> }
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPersonnelResult { pub total_rows: usize, pub accepted_rows: usize, pub rejected_rows: usize }
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentEquipment { pub plan_id: String, pub duty_point_id: String, pub selected_items: Vec<String> }
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDeploymentEquipmentInput { pub plan_id: String, pub duty_point_id: String, pub selected_items: Vec<String> }
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceState { pub plan_id: String, pub active_nav: String, pub selected_point_id: Option<String>, pub selected_route_id: Option<String>, pub deployment_route_id: Option<String>, pub deployment_choices: serde_json::Value, pub map_output_title: String, pub map_output_zoom: f64, pub map_output_bearing: f64 }
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveWorkspaceStateInput { pub plan_id: String, pub active_nav: String, pub selected_point_id: Option<String>, pub selected_route_id: Option<String>, pub deployment_route_id: Option<String>, pub deployment_choices: serde_json::Value, pub map_output_title: String, pub map_output_zoom: f64, pub map_output_bearing: f64 }

pub fn initialize_state(app_data_dir: PathBuf) -> Result<AppState, String> {
    fs::create_dir_all(&app_data_dir).map_err(|error| format!("無法建立應用程式資料目錄：{error}"))?;
    let database_path = app_data_dir.join("dutygrid.db");
    migrate(&database_path)?;
    Ok(AppState { database_path })
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
    let personnel_assignment_migration_done: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 8)", [], |row| row.get(0)).map_err(|e| format!("無法檢查人力配置 migration：{e}"))?;
    if !personnel_assignment_migration_done {
        connection.execute_batch(include_str!("../migrations/0008_personnel_assignments.sql")).map_err(|e| format!("無法套用人力配置 migration：{e}"))?;
        connection.execute("INSERT INTO schema_migrations(version) VALUES (?1)", [8]).map_err(|e| format!("無法記錄人力配置 migration：{e}"))?;
    }
    let personnel_import_error_migration_done: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 9)", [], |row| row.get(0)).map_err(|e| format!("無法檢查人力匯入錯誤 migration：{e}"))?;
    if !personnel_import_error_migration_done {
        connection.execute_batch(include_str!("../migrations/0009_personnel_import_errors.sql")).map_err(|e| format!("無法套用人力匯入錯誤 migration：{e}"))?;
        connection.execute("INSERT INTO schema_migrations(version) VALUES (?1)", [9]).map_err(|e| format!("無法記錄人力匯入錯誤 migration：{e}"))?;
    }
    let cross_route_assignment_migration_done: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 10)", [], |row| row.get(0)).map_err(|e| format!("無法檢查跨路線人力配置 migration：{e}"))?;
    if !cross_route_assignment_migration_done {
        connection.execute_batch(include_str!("../migrations/0010_allow_cross_route_assignments.sql")).map_err(|e| format!("無法套用跨路線人力配置 migration：{e}"))?;
        connection.execute("INSERT INTO schema_migrations(version) VALUES (?1)", [10]).map_err(|e| format!("無法記錄跨路線人力配置 migration：{e}"))?;
    }
    let personnel_phone_migration_done: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 11)", [], |row| row.get(0)).map_err(|e| format!("無法檢查人員電話 migration：{e}"))?;
    if !personnel_phone_migration_done {
        connection.execute_batch(include_str!("../migrations/0011_personnel_phone.sql")).map_err(|e| format!("無法套用人員電話 migration：{e}"))?;
        connection.execute("INSERT INTO schema_migrations(version) VALUES (?1)", [11]).map_err(|e| format!("無法記錄人員電話 migration：{e}"))?;
    }
    let deployment_equipment_migration_done: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 12)", [], |row| row.get(0)).map_err(|e| format!("無法檢查裝備配置 migration：{e}"))?;
    if !deployment_equipment_migration_done {
        connection.execute_batch(include_str!("../migrations/0012_deployment_equipment.sql")).map_err(|e| format!("無法套用裝備配置 migration：{e}"))?;
        connection.execute("INSERT INTO schema_migrations(version) VALUES (?1)", [12]).map_err(|e| format!("無法記錄裝備配置 migration：{e}"))?;
    }
    let workspace_state_migration_done: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 13)", [], |row| row.get(0)).map_err(|e| format!("無法檢查工作區狀態 migration：{e}"))?;
    if !workspace_state_migration_done {
        connection.execute_batch(include_str!("../migrations/0013_workspace_state.sql")).map_err(|e| format!("無法套用工作區狀態 migration：{e}"))?;
        connection.execute("INSERT INTO schema_migrations(version) VALUES (?1)", [13]).map_err(|e| format!("無法記錄工作區狀態 migration：{e}"))?;
    }
    let route_line_style_migration_done: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 14)", [], |row| row.get(0)).map_err(|e| format!("無法檢查路線樣式 migration：{e}"))?;
    if !route_line_style_migration_done {
        connection.execute_batch(include_str!("../migrations/0014_route_line_style.sql")).map_err(|e| format!("無法套用路線樣式 migration：{e}"))?;
        connection.execute("INSERT INTO schema_migrations(version) VALUES (?1)", [14]).map_err(|e| format!("無法記錄路線樣式 migration：{e}"))?;
    }
    let workspace_deployment_route_migration_done: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 15)", [], |row| row.get(0)).map_err(|e| format!("無法檢查部署表工作區 migration：{e}"))?;
    if !workspace_deployment_route_migration_done {
        connection.execute_batch(include_str!("../migrations/0015_workspace_deployment_route.sql")).map_err(|e| format!("無法套用部署表工作區 migration：{e}"))?;
        connection.execute("INSERT INTO schema_migrations(version) VALUES (?1)", [15]).map_err(|e| format!("無法記錄部署表工作區 migration：{e}"))?;
    }
    let duty_point_type_migration_done: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 16)", [], |row| row.get(0)).map_err(|e| format!("無法檢查號誌點位 migration：{e}"))?;
    if !duty_point_type_migration_done {
        connection.execute_batch(include_str!("../migrations/0016_duty_point_type.sql")).map_err(|e| format!("無法套用號誌點位 migration：{e}"))?;
        connection.execute("INSERT INTO schema_migrations(version) VALUES (?1)", [16]).map_err(|e| format!("無法記錄號誌點位 migration：{e}"))?;
    }
    seed_personnel(&connection)?;
    Ok(())
}

pub fn load_workspace_state(path: &Path, plan_id: &str) -> Result<Option<WorkspaceState>, String> {
    let connection = open_database(path)?;
    connection.query_row("SELECT plan_id, active_nav, selected_point_id, selected_route_id, deployment_route_id, deployment_choices_json, map_output_title, map_output_zoom, map_output_bearing FROM workspace_states WHERE plan_id = ?1", [plan_id], |row| {
        let choices_json: String = row.get(5)?;
        Ok(WorkspaceState { plan_id: row.get(0)?, active_nav: row.get(1)?, selected_point_id: row.get(2)?, selected_route_id: row.get(3)?, deployment_route_id: row.get(4)?, deployment_choices: serde_json::from_str(&choices_json).unwrap_or_else(|_| serde_json::json!({})), map_output_title: row.get(6)?, map_output_zoom: row.get(7)?, map_output_bearing: row.get(8)? })
    }).map(Some).or_else(|error| match error { rusqlite::Error::QueryReturnedNoRows => Ok(None), error => Err(format!("無法讀取勤務工作區狀態：{error}")) })
}

pub fn save_workspace_state(path: &Path, input: SaveWorkspaceStateInput) -> Result<(), String> {
    let connection = open_database(path)?;
    let choices = serde_json::to_string(&input.deployment_choices).map_err(|error| format!("無法保存部署表選項：{error}"))?;
    connection.execute("INSERT INTO workspace_states(plan_id, active_nav, selected_point_id, selected_route_id, deployment_route_id, deployment_choices_json, map_output_title, map_output_zoom, map_output_bearing, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, CURRENT_TIMESTAMP) ON CONFLICT(plan_id) DO UPDATE SET active_nav = excluded.active_nav, selected_point_id = excluded.selected_point_id, selected_route_id = excluded.selected_route_id, deployment_route_id = excluded.deployment_route_id, deployment_choices_json = excluded.deployment_choices_json, map_output_title = excluded.map_output_title, map_output_zoom = excluded.map_output_zoom, map_output_bearing = excluded.map_output_bearing, updated_at = CURRENT_TIMESTAMP", params![input.plan_id, input.active_nav, input.selected_point_id, input.selected_route_id, input.deployment_route_id, choices, input.map_output_title, input.map_output_zoom, input.map_output_bearing]).map_err(|error| format!("無法保存勤務工作區狀態：{error}"))?;
    Ok(())
}

fn seed_personnel(connection: &Connection) -> Result<(), String> {
    let count: i64 = connection.query_row("SELECT COUNT(*) FROM personnel", [], |row| row.get(0)).map_err(|error| format!("無法檢查人力種子資料：{error}"))?;
    if count > 0 { return sync_seed_personnel_phones(connection); }
    let batch_id: String = connection.query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0)).map_err(|error| format!("無法建立人力匯入批次：{error}"))?;
    connection.execute("INSERT INTO import_batches(id, source_file_name, total_rows, accepted_rows, rejected_rows) VALUES (?1, 'personnel-sample.csv', 56, 56, 0)", [batch_id.as_str()]).map_err(|error| format!("無法建立人力匯入批次：{error}"))?;
    let transaction = connection.unchecked_transaction().map_err(|error| format!("無法建立人力匯入交易：{error}"))?;
    for row in include_str!("../../data/seeds/personnel-sample.csv").lines().skip(1) {
        let columns = row.split(',').collect::<Vec<_>>();
        if columns.len() != 6 { return Err("人力種子資料欄位不完整。".to_owned()); }
        let id: String = transaction.query_row("SELECT lower(hex(randomblob(16)))", [], |result| result.get(0)).map_err(|error| format!("無法建立人員識別碼：{error}"))?;
        let raw_row_json = serde_json::json!({ "personnel_code": columns[0], "radio_code": columns[1], "name": columns[2], "title": columns[3], "unit": columns[4], "phone": columns[5] }).to_string();
        transaction.execute("INSERT INTO personnel(id, personnel_code, radio_code, name, title, unit, phone, import_batch_id, raw_row_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)", params![id, columns[0], columns[1], columns[2], columns[3], columns[4], columns[5], batch_id, raw_row_json]).map_err(|error| format!("無法匯入人力種子資料：{error}"))?;
    }
    transaction.commit().map_err(|error| format!("無法完成種子人力匯入：{error}"))
}

fn sync_seed_personnel_phones(connection: &Connection) -> Result<(), String> {
    for row in include_str!("../../data/seeds/personnel-sample.csv").lines().skip(1) {
        let columns = row.split(',').collect::<Vec<_>>();
        if columns.len() == 6 { connection.execute("UPDATE personnel SET phone = ?2 WHERE personnel_code = ?1 AND phone = ''", params![columns[0], columns[5]]).map_err(|error| format!("無法更新種子人員電話：{error}"))?; }
    }
    Ok(())
}

pub fn list_duty_routes(path: &Path, plan_id: &str) -> Result<Vec<DutyRoute>, String> {
    let connection = open_database(path)?;
    let mut statement = connection.prepare("SELECT id, plan_id, route_name, color, route_type, geometry_json, line_style FROM duty_routes WHERE plan_id = ?1 ORDER BY created_at").map_err(|e| format!("無法讀取勤務路線：{e}"))?;
    let rows = statement.query_map([plan_id], |row| Ok((row.get::<_, String>(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get::<_, Option<String>>(5)?, row.get(6)?))).map_err(|e| format!("無法查詢勤務路線：{e}"))?;
    rows.map(|row| { let (id, plan_id, route_name, color, route_type, geometry_json, line_style) = row.map_err(|e| format!("無法讀取勤務路線：{e}"))?; let mut stops = connection.prepare("SELECT point_id FROM duty_route_stops WHERE route_id = ?1 ORDER BY stop_order").map_err(|e| format!("無法讀取路線點位：{e}"))?; let point_ids = stops.query_map([&id], |r| r.get(0)).map_err(|e| format!("無法讀取路線點位：{e}"))?.collect::<Result<Vec<String>, _>>().map_err(|e| format!("無法讀取路線點位：{e}"))?; let geometry = geometry_json.map(|json| serde_json::from_str(&json).map_err(|e| format!("無法讀取手繪路線：{e}"))).transpose()?; Ok(DutyRoute { id, plan_id, route_name, color, point_ids, route_type, geometry, line_style }) }).collect()
}

pub fn create_duty_route(path: &Path, input: CreateDutyRouteInput) -> Result<DutyRoute, String> {
    if input.route_name.trim().is_empty() || input.point_ids.len() < 2 { return Err("請輸入路線名稱，並至少選擇兩個勤務點位。".to_owned()); }
    if !["solid", "dashed", "arrow", "dashed_arrow"].contains(&input.line_style.as_str()) { return Err("僅支援實線、虛線、實箭頭線或虛箭頭線。".to_owned()); }
    let mut connection = open_database(path)?; let tx = connection.transaction().map_err(|e| format!("無法建立路線交易：{e}"))?;
    let id: String = tx.query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0)).map_err(|e| format!("無法建立路線識別碼：{e}"))?;
    tx.execute("INSERT INTO duty_routes(id, plan_id, route_name, color, line_style) VALUES (?1, ?2, ?3, ?4, ?5)", params![id, input.plan_id, input.route_name.trim(), input.color, input.line_style]).map_err(|e| format!("無法保存勤務路線：{e}"))?;
    for (index, point_id) in input.point_ids.iter().enumerate() { tx.execute("INSERT INTO duty_route_stops(route_id, point_id, stop_order) VALUES (?1, ?2, ?3)", params![id, point_id, index as i64]).map_err(|e| format!("無法保存路線點位：{e}"))?; }
    tx.commit().map_err(|e| format!("無法完成勤務路線保存：{e}"))?; Ok(DutyRoute { id, plan_id: input.plan_id, route_name: input.route_name.trim().to_owned(), color: input.color, point_ids: input.point_ids, route_type: "point_sequence".to_owned(), geometry: None, line_style: input.line_style })
}

pub fn create_manual_route(path: &Path, input: CreateManualRouteInput) -> Result<DutyRoute, String> { if input.route_name.trim().is_empty() || input.geometry.len() < 2 { return Err("請輸入路線名稱，並繪製至少兩個節點。".to_owned()); } let connection = open_database(path)?; let id: String = connection.query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0)).map_err(|e| format!("無法建立路線識別碼：{e}"))?; let geometry_json = serde_json::to_string(&input.geometry).map_err(|e| format!("無法保存手繪路線：{e}"))?; connection.execute("INSERT INTO duty_routes(id, plan_id, route_name, color, route_type, geometry_json) VALUES (?1,?2,?3,?4,'manual',?5)", params![id,input.plan_id,input.route_name.trim(),input.color,geometry_json]).map_err(|e| format!("無法保存手繪路線：{e}"))?; Ok(DutyRoute { id, plan_id: input.plan_id, route_name: input.route_name.trim().to_owned(), color: input.color, point_ids: vec![], route_type: "manual".to_owned(), geometry: Some(input.geometry), line_style: "solid".to_owned() }) }

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

pub fn update_duty_route_line_style(path: &Path, route_id: &str, line_style: &str) -> Result<(), String> {
    if !["solid", "dashed", "arrow", "dashed_arrow"].contains(&line_style) { return Err("僅支援實線、虛線、實箭頭線或虛箭頭線。".to_owned()); }
    let connection = open_database(path)?;
    let updated = connection.execute("UPDATE duty_routes SET line_style = ?2 WHERE id = ?1", params![route_id, line_style]).map_err(|error| format!("無法更新路線樣式：{error}"))?;
    if updated == 0 { return Err("找不到要更新的路線。".to_owned()); }
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

pub fn list_personnel(path: &Path) -> Result<Vec<Personnel>, String> {
    let connection = open_database(path)?;
    let mut statement = connection.prepare("SELECT personnel.id, personnel.personnel_code, personnel.radio_code, personnel.name, personnel.title, personnel.unit, personnel.phone, COALESCE(import_batches.source_file_name = 'personnel-sample.csv', 0) FROM personnel LEFT JOIN import_batches ON import_batches.id = personnel.import_batch_id ORDER BY personnel.unit, personnel.title, personnel.personnel_code").map_err(|error| format!("無法讀取人員資料：{error}"))?;
    let rows = statement.query_map([], |row| Ok(Personnel { id: row.get(0)?, personnel_code: row.get(1)?, radio_code: row.get(2)?, name: row.get(3)?, title: row.get(4)?, unit: row.get(5)?, phone: row.get(6)?, is_sample: row.get(7)? })).map_err(|error| format!("無法查詢人員資料：{error}"))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|error| format!("無法讀取人員資料：{error}"))
}

pub fn list_personnel_assignments(path: &Path, plan_id: &str) -> Result<Vec<PersonnelAssignment>, String> {
    let connection = open_database(path)?;
    let mut statement = connection.prepare("SELECT id, plan_id, personnel_id, duty_point_id, assigned_unit, assigned_title FROM personnel_assignments WHERE plan_id = ?1 ORDER BY created_at").map_err(|error| format!("無法讀取人力配置：{error}"))?;
    let rows = statement.query_map([plan_id], |row| Ok(PersonnelAssignment { id: row.get(0)?, plan_id: row.get(1)?, personnel_id: row.get(2)?, duty_point_id: row.get(3)?, assigned_unit: row.get(4)?, assigned_title: row.get(5)? })).map_err(|error| format!("無法查詢人力配置：{error}"))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|error| format!("無法讀取人力配置：{error}"))
}

pub fn create_personnel_assignment(path: &Path, input: CreatePersonnelAssignmentInput) -> Result<PersonnelAssignment, String> {
    let connection = open_database(path)?;
    let id: String = connection.query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0)).map_err(|error| format!("無法建立人力配置識別碼：{error}"))?;
    connection.execute("INSERT INTO personnel_assignments(id, plan_id, personnel_id, duty_point_id, assigned_unit, assigned_title) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![id, input.plan_id, input.personnel_id, input.duty_point_id, input.assigned_unit, input.assigned_title]).map_err(|error| format!("無法配置人員：{error}"))?;
    Ok(PersonnelAssignment { id, plan_id: input.plan_id, personnel_id: input.personnel_id, duty_point_id: input.duty_point_id, assigned_unit: input.assigned_unit, assigned_title: input.assigned_title })
}

pub fn delete_personnel_assignment(path: &Path, assignment_id: &str) -> Result<(), String> {
    let connection = open_database(path)?;
    if connection.execute("DELETE FROM personnel_assignments WHERE id = ?1", [assignment_id]).map_err(|error| format!("無法移除人力配置：{error}"))? == 0 { return Err("找不到要移除的人力配置。".to_owned()); }
    Ok(())
}

pub fn move_personnel_assignment(path: &Path, assignment_id: &str, duty_point_id: String) -> Result<(), String> {
    let connection = open_database(path)?;
    if connection.execute("UPDATE personnel_assignments SET duty_point_id = ?2 WHERE id = ?1", params![assignment_id, duty_point_id]).map_err(|error| format!("無法移動人力配置：{error}"))? == 0 { return Err("找不到要移動的人力配置。".to_owned()); }
    Ok(())
}

pub fn list_deployment_equipment(path: &Path, plan_id: &str) -> Result<Vec<DeploymentEquipment>, String> {
    let connection = open_database(path)?;
    let mut statement = connection.prepare("SELECT plan_id, duty_point_id, selected_items_json FROM deployment_equipment WHERE plan_id = ?1 ORDER BY duty_point_id").map_err(|error| format!("無法讀取部署裝備：{error}"))?;
    let rows = statement.query_map([plan_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))).map_err(|error| format!("無法查詢部署裝備：{error}"))?;
    rows.map(|row| {
        let (plan_id, duty_point_id, selected_items_json) = row.map_err(|error| format!("無法讀取部署裝備：{error}"))?;
        let selected_items = serde_json::from_str(&selected_items_json).map_err(|error| format!("部署裝備資料格式錯誤：{error}"))?;
        Ok(DeploymentEquipment { plan_id, duty_point_id, selected_items })
    }).collect()
}

pub fn save_deployment_equipment(path: &Path, input: SaveDeploymentEquipmentInput) -> Result<DeploymentEquipment, String> {
    let selected_items = input.selected_items.into_iter().map(|item| item.trim().to_owned()).filter(|item| !item.is_empty()).collect::<Vec<_>>();
    let selected_items_json = serde_json::to_string(&selected_items).map_err(|error| format!("無法保存部署裝備：{error}"))?;
    let connection = open_database(path)?;
    connection.execute(
        "INSERT INTO deployment_equipment(plan_id, duty_point_id, selected_items_json) VALUES (?1, ?2, ?3) ON CONFLICT(plan_id, duty_point_id) DO UPDATE SET selected_items_json = excluded.selected_items_json, updated_at = CURRENT_TIMESTAMP",
        params![input.plan_id, input.duty_point_id, selected_items_json],
    ).map_err(|error| format!("無法保存部署裝備：{error}"))?;
    Ok(DeploymentEquipment { plan_id: input.plan_id, duty_point_id: input.duty_point_id, selected_items })
}

pub fn import_personnel_xlsx(path: &Path, input: ImportPersonnelInput) -> Result<ImportPersonnelResult, String> {
    if !input.file_name.to_lowercase().ends_with(".xlsx") { return Err("僅接受 .xlsx 人力資料檔。".to_owned()); }
    let mut workbook = Xlsx::new(Cursor::new(input.file_data)).map_err(|error| format!("無法讀取 Excel：{error}"))?;
    let range = workbook.worksheet_range_at(0).ok_or_else(|| "Excel 沒有工作表。".to_owned())?.map_err(|error| format!("無法讀取工作表：{error}"))?;
    let required = ["personnel_code", "radio_code", "name", "title", "unit", "phone"];
    let aliases = |field: &str| match field {
        "personnel_code" => &["personnel_code", "personnel-number", "員編"][..],
        "radio_code" => &["radio_code", "radio", "無線電代號"][..],
        "name" => &["name", "姓名"][..],
        "title" => &["title", "職稱"][..],
        "unit" => &["unit", "所屬單位"][..],
        "phone" => &["phone", "聯絡電話"][..],
        _ => &[][..],
    };
    let (header_row_index, headers) = range.rows().take(10).enumerate().find_map(|(index, row)| {
        let headers = row.iter().map(|cell| cell.to_string().trim().trim_start_matches('\u{feff}').to_owned()).collect::<Vec<_>>();
        let has_header = |field: &str| headers.iter().any(|header| aliases(field).contains(&header.as_str()));
        required.iter().all(|field| has_header(field)).then_some((index, headers))
    }).ok_or_else(|| "Excel 前 10 列必須包含：員編、無線電代號、姓名、職稱、所屬單位、聯絡電話（亦支援英文欄名）。".to_owned())?;
    let index_of = |field: &str| headers.iter().position(|header| aliases(field).contains(&header.as_str())).expect("validated required header");
    let connection = open_database(path)?;
    let batch_id: String = connection.query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0)).map_err(|error| format!("無法建立匯入批次：{error}"))?;
    connection.execute("INSERT INTO import_batches(id, source_file_name) VALUES (?1, ?2)", params![batch_id, input.file_name]).map_err(|error| format!("無法建立匯入批次：{error}"))?;
    let mut total_rows = 0usize; let mut accepted_rows = 0usize; let mut rejected_rows = 0usize;
    for (offset, row) in range.rows().enumerate().skip(header_row_index + 1) {
        if row.iter().all(|cell| cell.to_string().trim().is_empty()) { continue; }
        total_rows += 1;
        let value = |field: &str| row.get(index_of(field)).map(|cell| cell.to_string().trim().to_owned()).unwrap_or_default();
        let personnel_code = value("personnel_code"); let radio_code = value("radio_code"); let name = value("name"); let title = value("title"); let unit = value("unit"); let phone = value("phone");
        let raw_row_json = serde_json::json!({ "personnel_code": personnel_code, "radio_code": radio_code, "name": name, "title": title, "unit": unit, "phone": phone }).to_string();
        let error_reason = if personnel_code.is_empty() || radio_code.is_empty() || name.is_empty() || title.is_empty() || unit.is_empty() || phone.is_empty() { Some("必填欄位不可空白。".to_owned()) } else { None };
        if let Some(reason) = error_reason { rejected_rows += 1; connection.execute("INSERT INTO personnel_import_errors(id, import_batch_id, row_number, raw_row_json, error_reason) VALUES (lower(hex(randomblob(16))), ?1, ?2, ?3, ?4)", params![batch_id, (offset + 1) as i64, raw_row_json, reason]).map_err(|error| format!("無法記錄匯入錯誤：{error}"))?; continue; }
        let id: String = connection.query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0)).map_err(|error| format!("無法建立人員識別碼：{error}"))?;
        match connection.execute("INSERT INTO personnel(id, personnel_code, radio_code, name, title, unit, phone, import_batch_id, raw_row_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)", params![id, personnel_code, radio_code, name, title, unit, phone, batch_id, raw_row_json]) { Ok(_) => accepted_rows += 1, Err(error) => { rejected_rows += 1; connection.execute("INSERT INTO personnel_import_errors(id, import_batch_id, row_number, raw_row_json, error_reason) VALUES (lower(hex(randomblob(16))), ?1, ?2, ?3, ?4)", params![batch_id, (offset + 1) as i64, raw_row_json, error.to_string()]).map_err(|record_error| format!("無法記錄匯入錯誤：{record_error}"))?; } }
    }
    connection.execute("UPDATE import_batches SET total_rows = ?2, accepted_rows = ?3, rejected_rows = ?4 WHERE id = ?1", params![batch_id, total_rows as i64, accepted_rows as i64, rejected_rows as i64]).map_err(|error| format!("無法完成匯入批次：{error}"))?;
    Ok(ImportPersonnelResult { total_rows, accepted_rows, rejected_rows })
}

pub fn list_duty_points(path: &Path, plan_id: &str) -> Result<Vec<DutyPoint>, String> {
    let connection = open_database(path)?;
    let mut statement = connection.prepare("SELECT id, plan_id, point_code, point_name, note, color, point_type, latitude, longitude, visible FROM duty_points WHERE plan_id = ?1 ORDER BY point_code").map_err(|e| format!("無法讀取勤務點位：{e}"))?;
    let points = statement.query_map([plan_id], |r| Ok(DutyPoint { id:r.get(0)?, plan_id:r.get(1)?, point_code:r.get(2)?, point_name:r.get(3)?, note:r.get(4)?, color:r.get(5)?, point_type:r.get(6)?, latitude:r.get(7)?, longitude:r.get(8)?, visible:r.get::<_, i64>(9)? != 0 })).map_err(|e| format!("無法查詢勤務點位：{e}"))?.collect::<Result<Vec<_>,_>>().map_err(|e| format!("無法讀取勤務點位資料：{e}"))?;
    Ok(points)
}

pub fn create_duty_point(path: &Path, input: CreateDutyPointInput) -> Result<DutyPoint, String> {
    if input.point_code.trim().is_empty() || input.point_name.trim().is_empty() { return Err("點位編號與名稱不可空白。".to_owned()); }
    let connection = open_database(path)?;
    let id: String = connection.query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0)).map_err(|e| format!("無法建立點位識別碼：{e}"))?;
    if !["duty", "hollow", "signal"].contains(&input.point_type.as_str()) { return Err("不支援的點位類型。".to_owned()); }
    connection.execute("INSERT INTO duty_points(id, plan_id, point_code, point_name, note, color, point_type, latitude, longitude) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)", params![id,input.plan_id,input.point_code.trim(),input.point_name.trim(),input.note,input.color,input.point_type,input.latitude,input.longitude]).map_err(|e| format!("無法保存勤務點位：{e}"))?;
    connection.query_row("SELECT id, plan_id, point_code, point_name, note, color, point_type, latitude, longitude, visible FROM duty_points WHERE id=?1", [id], |r| Ok(DutyPoint { id:r.get(0)?, plan_id:r.get(1)?, point_code:r.get(2)?, point_name:r.get(3)?, note:r.get(4)?, color:r.get(5)?, point_type:r.get(6)?, latitude:r.get(7)?, longitude:r.get(8)?, visible:r.get::<_, i64>(9)? != 0 })).map_err(|e| format!("勤務點位已保存，但無法讀回資料：{e}"))
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
    #[test]
    fn deployment_equipment_persists_per_point() {
        let path = std::env::temp_dir().join(format!("dutygrid-equipment-test-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        migrate(&path).expect("migration should succeed");
        let plan = create_duty_plan(&path, CreateDutyPlanInput { plan_name: "裝備測試".to_owned(), duty_date: None, start_time: None, end_time: None, description: None }).expect("plan should be saved");
        let point = create_duty_point(&path, CreateDutyPointInput { plan_id: plan.id.clone(), point_code: "901".to_owned(), point_name: "測試崗哨".to_owned(), note: None, color: "red".to_owned(), point_type: "signal".to_owned(), latitude: 25.0, longitude: 121.0 }).expect("point should be saved");
        assert_eq!(point.point_type, "signal");
        save_deployment_equipment(&path, SaveDeploymentEquipmentInput { plan_id: plan.id.clone(), duty_point_id: point.id, selected_items: vec!["制服".to_owned(), "無線電(空氣導管耳機)".to_owned()] }).expect("equipment should be saved");
        let saved = list_deployment_equipment(&path, &plan.id).expect("equipment should load");
        assert_eq!(saved[0].selected_items, ["制服", "無線電(空氣導管耳機)"]);
        let _ = std::fs::remove_file(path);
    }
}
