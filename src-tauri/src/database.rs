use std::fs;
#[cfg(not(test))]
use std::hash::{Hash, Hasher};
use std::io::Cursor;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use calamine::{Reader, Xlsx};
use encoding_rs::BIG5;
#[cfg(not(test))]
use keyring::{Entry, Error as KeyringError};
#[cfg(not(test))]
use rand::RngCore;
use rusqlite::{params, Connection};
use serde::Serialize;

const MAX_ROUTE_VERTICES: usize = 10_000;
const MAX_EQUIPMENT_ITEMS: usize = 100;
const MAX_TEXT_LENGTH: usize = 500;
const MAX_IMPORT_BYTES: usize = 10 * 1024 * 1024;
const MAX_IMPORT_ROWS: usize = 20_000;
const LATEST_MIGRATION_VERSION: i64 = 18;
#[cfg(not(test))]
const DATABASE_KEY_ID_FILE: &str = "dutygrid.key-id";
#[cfg(not(test))]
const DATABASE_KEYRING_SERVICE: &str = "tw.gov.dutygrid.database";
#[cfg(not(test))]
const LEGACY_DATABASE_KEYRING_ACCOUNT: &str = "dutygrid.db.v1";

fn supported_color(color: &str) -> bool {
    ["red", "orange", "yellow", "green", "blue", "purple"].contains(&color)
}

fn validate_text(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{field}不可空白。"));
    }
    if value.chars().count() > MAX_TEXT_LENGTH {
        return Err(format!("{field}不可超過 {MAX_TEXT_LENGTH} 個字元。"));
    }
    Ok(())
}

fn validate_coordinates(
    latitude: f64,
    longitude: f64,
    coordinate_x: Option<f64>,
    coordinate_y: Option<f64>,
) -> Result<(), String> {
    match coordinate_x.zip(coordinate_y) {
        Some((x, y))
            if x.is_finite()
                && y.is_finite()
                && (0.0..=1000.0).contains(&x)
                && (0.0..=1000.0).contains(&y) =>
        {
            Ok(())
        }
        Some(_) => Err("XY 座標必須是介於 0 至 1000 的有限數值。".to_owned()),
        None if latitude.is_finite()
            && longitude.is_finite()
            && (-90.0..=90.0).contains(&latitude)
            && (-180.0..=180.0).contains(&longitude) =>
        {
            Ok(())
        }
        None => Err("請輸入有效的經緯度。".to_owned()),
    }
}

fn validate_geometry(geometry: &[[f64; 2]]) -> Result<(), String> {
    if geometry.len() < 2 {
        return Err("路線至少需要兩個節點。".to_owned());
    }
    if geometry.len() > MAX_ROUTE_VERTICES {
        return Err(format!("路線節點不可超過 {MAX_ROUTE_VERTICES} 個。"));
    }
    let is_geographic = geometry.iter().all(|[x, y]| {
        x.is_finite() && y.is_finite() && (-180.0..=180.0).contains(x) && (-90.0..=90.0).contains(y)
    });
    let is_custom = geometry.iter().all(|[x, y]| {
        x.is_finite() && y.is_finite() && (0.0..=1000.0).contains(x) && (0.0..=1000.0).contains(y)
    });
    if is_geographic || is_custom {
        Ok(())
    } else {
        Err("路線座標必須全部為有效經緯度或自選底圖 XY 座標。".to_owned())
    }
}

fn point_belongs_to_plan(
    connection: &Connection,
    plan_id: &str,
    point_id: &str,
) -> Result<bool, String> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM duty_points WHERE id = ?1 AND plan_id = ?2)",
            params![point_id, plan_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("無法驗證勤務點位：{error}"))
}

pub struct AppState {
    pub database_path: PathBuf,
    pub app_data_dir: PathBuf,
}

/// Records metadata only. Callers must never put names, phone numbers, SQL, or file paths in fields.
pub fn append_audit_log(
    app_data_dir: &Path,
    operation: &str,
    resource: &str,
    record_count: usize,
    success: bool,
) -> Result<(), String> {
    let audit_dir = app_data_dir.join("logs");
    fs::create_dir_all(&audit_dir).map_err(|error| format!("無法建立稽核紀錄目錄：{error}"))?;
    restrict_owner_permissions(&audit_dir, true)?;
    let day = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("無法建立稽核時間：{error}"))?
        .as_secs()
        / 86_400;
    let path = audit_dir.join(format!("audit-{day}.log"));
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("無法建立稽核時間：{error}"))?
        .as_secs();
    let line = format!(
        "{{\"timestamp\":{timestamp},\"operation\":\"{operation}\",\"resource\":\"{resource}\",\"recordCount\":{record_count},\"success\":{success}}}\n"
    );
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|error| format!("無法開啟稽核紀錄：{error}"))?;
    restrict_owner_permissions(&path, false)?;
    file.write_all(line.as_bytes())
        .map_err(|error| format!("無法寫入稽核紀錄：{error}"))
}

#[cfg(not(test))]
fn key_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn valid_database_key(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(not(test))]
fn database_key_id(path: &Path) -> Result<String, String> {
    let directory = path
        .parent()
        .ok_or_else(|| "無法判定資料庫資料目錄。".to_owned())?;
    let id_path = directory.join(DATABASE_KEY_ID_FILE);
    match fs::read_to_string(&id_path) {
        Ok(id)
            if id.trim().len() == 32 && id.trim().bytes().all(|byte| byte.is_ascii_hexdigit()) =>
        {
            Ok(id.trim().to_owned())
        }
        Ok(_) => Err("資料庫金鑰識別碼格式無效。".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut bytes = [0u8; 16];
            rand::rng().fill_bytes(&mut bytes);
            let id = key_hex(&bytes);
            fs::write(&id_path, &id)
                .map_err(|error| format!("無法建立資料庫金鑰識別碼：{error}"))?;
            restrict_owner_permissions(&id_path, false)?;
            Ok(id)
        }
        Err(error) => Err(format!("無法讀取資料庫金鑰識別碼：{error}")),
    }
}

#[cfg(test)]
fn database_key(_path: &Path) -> Result<String, String> {
    Ok("4f1d9e31b9ca2a6f0c5d81a466f9ca75f1528e9ef8d3b8c17a2e945c13bfe68d".to_owned())
}

#[cfg(test)]
fn legacy_database_key() -> Result<Option<String>, String> {
    Ok(None)
}

#[cfg(test)]
fn store_database_key(_path: &Path, _key: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(not(test))]
fn database_key(path: &Path) -> Result<String, String> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    let account = format!("dutygrid.db.v1.{:016x}", hasher.finish());
    let entry = Entry::new(DATABASE_KEYRING_SERVICE, &account)
        .map_err(|error| format!("無法使用作業系統金鑰儲存區：{error}"))?;
    match entry.get_password() {
        Ok(key) if valid_database_key(&key) => Ok(key),
        Ok(_) => Err("作業系統金鑰儲存區中的 DutyGrid 資料庫金鑰格式無效。".to_owned()),
        Err(KeyringError::NoEntry) => {
            if let Some(previous_key) = v2_database_key(path)? {
                entry
                    .set_password(&previous_key)
                    .map_err(|error| format!("無法建立相容資料庫金鑰參照：{error}"))?;
                return Ok(previous_key);
            }
            let mut bytes = [0u8; 32];
            rand::rng().fill_bytes(&mut bytes);
            let key = key_hex(&bytes);
            entry
                .set_password(&key)
                .map_err(|error| format!("無法將資料庫金鑰儲存至作業系統金鑰儲存區：{error}"))?;
            Ok(key)
        }
        Err(error) => Err(format!("無法讀取作業系統金鑰儲存區中的資料庫金鑰：{error}")),
    }
}

#[cfg(not(test))]
fn v2_database_key(path: &Path) -> Result<Option<String>, String> {
    let account = format!("dutygrid.db.v2.{}", database_key_id(path)?);
    let entry = Entry::new(DATABASE_KEYRING_SERVICE, &account)
        .map_err(|error| format!("無法使用作業系統金鑰儲存區：{error}"))?;
    match entry.get_password() {
        Ok(key) if valid_database_key(&key) => Ok(Some(key)),
        Ok(_) => Err("相容資料庫金鑰格式無效。".to_owned()),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(error) => Err(format!("無法讀取舊版路徑式資料庫金鑰：{error}")),
    }
}

#[cfg(not(test))]
fn legacy_database_key() -> Result<Option<String>, String> {
    let entry = Entry::new(DATABASE_KEYRING_SERVICE, LEGACY_DATABASE_KEYRING_ACCOUNT)
        .map_err(|error| format!("無法使用作業系統金鑰儲存區：{error}"))?;
    match entry.get_password() {
        Ok(key) if valid_database_key(&key) => Ok(Some(key)),
        Ok(_) => Err("舊版資料庫金鑰格式無效。".to_owned()),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(error) => Err(format!("無法讀取舊版資料庫金鑰：{error}")),
    }
}

#[cfg(not(test))]
fn store_database_key(path: &Path, key: &str) -> Result<(), String> {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    let account = format!("dutygrid.db.v1.{:016x}", hasher.finish());
    let entry = Entry::new(DATABASE_KEYRING_SERVICE, &account)
        .map_err(|error| format!("無法使用作業系統金鑰儲存區：{error}"))?;
    entry
        .set_password(key)
        .map_err(|error| format!("無法更新資料庫金鑰參照：{error}"))
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
    pub plan_mode: String,
    pub basemap_path: Option<String>,
    pub basemap_width: Option<i64>,
    pub basemap_height: Option<i64>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDutyPlanInput {
    pub plan_name: String,
    pub duty_date: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub description: Option<String>,
    pub plan_mode: Option<String>,
    pub basemap_path: Option<String>,
    pub basemap_width: Option<i64>,
    pub basemap_height: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DutyPoint {
    pub id: String,
    pub plan_id: String,
    pub point_code: String,
    pub point_name: String,
    pub note: Option<String>,
    pub color: String,
    pub point_type: String,
    pub latitude: f64,
    pub longitude: f64,
    pub coordinate_x: Option<f64>,
    pub coordinate_y: Option<f64>,
    pub visible: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDutyPointInput {
    pub plan_id: String,
    pub point_code: String,
    pub point_name: String,
    pub note: Option<String>,
    pub color: String,
    pub point_type: String,
    pub latitude: f64,
    pub longitude: f64,
    pub coordinate_x: Option<f64>,
    pub coordinate_y: Option<f64>,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDutyPointInput {
    pub point_code: String,
    pub point_name: String,
    pub note: Option<String>,
    pub color: String,
    pub point_type: String,
    pub latitude: f64,
    pub longitude: f64,
    pub coordinate_x: Option<f64>,
    pub coordinate_y: Option<f64>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DutyRoute {
    pub id: String,
    pub plan_id: String,
    pub route_name: String,
    pub color: String,
    pub point_ids: Vec<String>,
    pub route_type: String,
    pub geometry: Option<Vec<[f64; 2]>>,
    pub line_style: String,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDutyRouteInput {
    pub plan_id: String,
    pub route_name: String,
    pub color: String,
    pub point_ids: Vec<String>,
    pub line_style: String,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateManualRouteInput {
    pub plan_id: String,
    pub route_name: String,
    pub color: String,
    pub geometry: Vec<[f64; 2]>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonRoute {
    pub id: String,
    pub route_name: String,
    pub color: String,
    pub geometry: Vec<[f64; 2]>,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCommonRouteInput {
    pub route_name: String,
    pub color: String,
    pub geometry: Vec<[f64; 2]>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Personnel {
    pub id: String,
    pub personnel_code: String,
    pub radio_code: String,
    pub name: String,
    pub title: String,
    pub unit: String,
    pub phone: String,
    pub is_sample: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonnelAssignment {
    pub id: String,
    pub plan_id: String,
    pub personnel_id: String,
    pub duty_point_id: Option<String>,
    pub assigned_unit: String,
    pub assigned_title: String,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePersonnelAssignmentInput {
    pub plan_id: String,
    pub personnel_id: String,
    pub duty_point_id: Option<String>,
    pub assigned_unit: String,
    pub assigned_title: String,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPersonnelInput {
    pub file_name: String,
    pub file_data: Vec<u8>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPersonnelResult {
    pub total_rows: usize,
    pub accepted_rows: usize,
    pub rejected_rows: usize,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonnelImportError {
    pub row_number: i64,
    pub error_reason: String,
    pub raw_row_json: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonnelImportLog {
    pub source_file_name: String,
    pub total_rows: i64,
    pub accepted_rows: i64,
    pub rejected_rows: i64,
    pub errors: Vec<PersonnelImportError>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentEquipment {
    pub plan_id: String,
    pub duty_point_id: String,
    pub selected_items: Vec<String>,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDeploymentEquipmentInput {
    pub plan_id: String,
    pub duty_point_id: String,
    pub selected_items: Vec<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceState {
    pub plan_id: String,
    pub active_nav: String,
    pub selected_point_id: Option<String>,
    pub selected_route_id: Option<String>,
    pub deployment_route_id: Option<String>,
    pub deployment_choices: serde_json::Value,
    pub map_output_title: String,
    pub map_output_zoom: f64,
    pub map_output_bearing: f64,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveWorkspaceStateInput {
    pub plan_id: String,
    pub active_nav: String,
    pub selected_point_id: Option<String>,
    pub selected_route_id: Option<String>,
    pub deployment_route_id: Option<String>,
    pub deployment_choices: serde_json::Value,
    pub map_output_title: String,
    pub map_output_zoom: f64,
    pub map_output_bearing: f64,
}

pub fn initialize_state(app_data_dir: PathBuf) -> Result<AppState, String> {
    fs::create_dir_all(&app_data_dir)
        .map_err(|error| format!("無法建立應用程式資料目錄：{error}"))?;
    restrict_owner_permissions(&app_data_dir, true)?;
    let database_path = app_data_dir.join("dutygrid.db");
    let key = database_key(&database_path)?;
    if database_path.exists() && !is_encrypted_database(&database_path, &key) {
        if let Some(legacy_key) = legacy_database_key()? {
            if is_encrypted_database(&database_path, &legacy_key) {
                store_database_key(&database_path, &legacy_key)?;
            } else if is_plaintext_database(&database_path)? {
                migrate_plaintext_database(&database_path, &key)?;
            } else {
                return Err("無法以目前或舊版作業系統金鑰儲存區中的金鑰開啟加密資料庫。資料庫可能屬於其他 OS 帳號，或金鑰已遺失。".to_owned());
            }
        } else if is_plaintext_database(&database_path)? {
            migrate_plaintext_database(&database_path, &key)?;
        } else {
            return Err("無法以作業系統金鑰儲存區中的金鑰開啟加密資料庫。資料庫可能屬於其他 OS 帳號，或金鑰已遺失。".to_owned());
        }
    }
    if database_path.exists()
        && recorded_migration_version(&database_path) < LATEST_MIGRATION_VERSION
    {
        backup_before_migration(&database_path)?;
    }
    migrate(&database_path)?;
    restrict_owner_permissions(&database_path, false)?;
    Ok(AppState {
        database_path,
        app_data_dir,
    })
}

fn is_plaintext_database(path: &Path) -> Result<bool, String> {
    let bytes = fs::read(path).map_err(|error| format!("無法讀取既有資料庫：{error}"))?;
    Ok(bytes.starts_with(b"SQLite format 3\0"))
}

#[cfg(unix)]
fn restrict_owner_permissions(path: &Path, directory: bool) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(
        path,
        fs::Permissions::from_mode(if directory { 0o700 } else { 0o600 }),
    )
    .map_err(|error| format!("無法設定本機資料存取權限：{error}"))
}

#[cfg(not(unix))]
fn restrict_owner_permissions(_path: &Path, _directory: bool) -> Result<(), String> {
    Ok(())
}

fn recorded_migration_version(path: &Path) -> i64 {
    let Ok(connection) = open_database(path) else {
        return 0;
    };
    connection
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0)
}

fn backup_before_migration(path: &Path) -> Result<PathBuf, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("無法建立資料庫備份時間戳：{error}"))?
        .as_nanos();
    let backup_path = path.with_file_name(format!("dutygrid.pre-migration-{timestamp}.db"));
    fs::copy(path, &backup_path).map_err(|error| format!("無法備份升版前資料庫：{error}"))?;
    for suffix in ["-wal", "-shm"] {
        let source = PathBuf::from(format!("{}{}", path.display(), suffix));
        if source.exists() {
            let destination = PathBuf::from(format!("{}{}", backup_path.display(), suffix));
            fs::copy(source, destination)
                .map_err(|error| format!("無法備份升版前資料庫 journal：{error}"))?;
        }
    }
    Ok(backup_path)
}

fn open_database(path: &Path) -> Result<Connection, String> {
    let key = database_key(path)?;
    open_encrypted_database(path, &key)
}

fn open_encrypted_database(path: &Path, key: &str) -> Result<Connection, String> {
    if !valid_database_key(key) {
        return Err("資料庫金鑰格式無效。".to_owned());
    }
    let connection =
        Connection::open(path).map_err(|error| format!("無法開啟本機資料庫：{error}"))?;
    connection
        .execute_batch(&format!(
            "PRAGMA key = \"x'{key}'\"; PRAGMA cipher_memory_security = ON; PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;"
        ))
        .map_err(|error| format!("無法設定本機資料庫：{error}"))?;
    connection
        .query_row("SELECT COUNT(*) FROM sqlite_master", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|_| {
            "無法以作業系統金鑰儲存區中的金鑰開啟資料庫。資料庫可能屬於其他 OS 帳號或金鑰已遺失。"
                .to_owned()
        })?;
    Ok(connection)
}

fn is_encrypted_database(path: &Path, key: &str) -> bool {
    open_encrypted_database(path, key).is_ok()
}

fn migrate_plaintext_database(path: &Path, key: &str) -> Result<(), String> {
    let temporary = path.with_file_name("dutygrid.encrypting.db");
    if temporary.exists() {
        fs::remove_file(&temporary)
            .map_err(|error| format!("無法清除未完成的加密遷移檔案：{error}"))?;
    }
    let source =
        Connection::open(path).map_err(|error| format!("無法開啟既有明文資料庫：{error}"))?;
    source
        .execute_batch("PRAGMA journal_mode = DELETE;")
        .map_err(|error| format!("無法整理既有明文資料庫 journal：{error}"))?;
    let temporary_name = temporary.to_string_lossy().replace('\'', "''");
    source.execute_batch(&format!(
        "ATTACH DATABASE '{temporary_name}' AS encrypted KEY \"x'{key}'\"; SELECT sqlcipher_export('encrypted'); DETACH DATABASE encrypted;"
    )).map_err(|error| format!("無法建立加密資料庫副本：{error}"))?;
    drop(source);
    open_encrypted_database(&temporary, key)?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("無法以加密資料庫取代既有資料庫：{error}"))?;
    Ok(())
}

fn migration_done(connection: &Connection, version: i64) -> Result<bool, String> {
    let has_migration_table: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations')",
            [],
            |row| row.get(0),
        )
        .map_err(|error| format!("無法檢查資料庫 migration：{error}"))?;
    if !has_migration_table {
        return Ok(false);
    }
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
            [version],
            |row| row.get(0),
        )
        .map_err(|error| format!("無法檢查資料庫 migration：{error}"))
}

fn apply_transactional_migration(
    connection: &mut Connection,
    version: i64,
    sql: &str,
    label: &str,
) -> Result<(), String> {
    if migration_done(connection, version)? {
        return Ok(());
    }
    let transaction = connection
        .transaction()
        .map_err(|error| format!("無法建立{label} migration 交易：{error}"))?;
    transaction
        .execute_batch(sql)
        .map_err(|error| format!("無法套用{label} migration：{error}"))?;
    transaction
        .execute(
            "INSERT INTO schema_migrations(version) VALUES (?1)",
            [version],
        )
        .map_err(|error| format!("無法記錄{label} migration：{error}"))?;
    transaction
        .commit()
        .map_err(|error| format!("無法提交{label} migration：{error}"))
}

pub fn migrate(path: &Path) -> Result<(), String> {
    let mut connection = open_database(path)?;
    apply_transactional_migration(
        &mut connection,
        1,
        include_str!("../migrations/0001_initial.sql"),
        "資料庫",
    )?;
    apply_transactional_migration(
        &mut connection,
        2,
        include_str!("../migrations/0002_duty_points.sql"),
        "勤務點位",
    )?;
    let has_point_color = connection
        .prepare("PRAGMA table_info(duty_points)")
        .map_err(|error| format!("無法檢查勤務點位欄位：{error}"))?
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| format!("無法讀取勤務點位欄位：{error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("無法讀取勤務點位欄位：{error}"))?
        .iter()
        .any(|column| column == "color");
    if !has_point_color {
        connection
            .execute_batch(include_str!("../migrations/0003_duty_point_color.sql"))
            .map_err(|error| format!("無法套用勤務點位顏色 migration：{error}"))?;
    }
    connection
        .execute(
            "INSERT OR IGNORE INTO schema_migrations(version) VALUES (?1)",
            [3],
        )
        .map_err(|error| format!("無法記錄勤務點位顏色 migration：{error}"))?;
    connection
        .execute_batch(include_str!("../migrations/0004_duty_routes.sql"))
        .map_err(|error| format!("無法套用勤務路線 migration：{error}"))?;
    connection
        .execute(
            "INSERT OR IGNORE INTO schema_migrations(version) VALUES (?1)",
            [4],
        )
        .map_err(|error| format!("無法記錄勤務路線 migration：{error}"))?;
    let has_geometry = connection
        .prepare("PRAGMA table_info(duty_routes)")
        .map_err(|e| format!("無法檢查路線欄位：{e}"))?
        .query_map([], |r| r.get::<_, String>(1))
        .map_err(|e| format!("無法讀取路線欄位：{e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("無法讀取路線欄位：{e}"))?
        .iter()
        .any(|c| c == "geometry_json");
    if !has_geometry {
        connection
            .execute_batch(include_str!("../migrations/0005_manual_route_geometry.sql"))
            .map_err(|e| format!("無法套用手繪路線 migration：{e}"))?;
    }
    connection
        .execute(
            "INSERT OR IGNORE INTO schema_migrations(version) VALUES (?1)",
            [5],
        )
        .map_err(|e| format!("無法記錄手繪路線 migration：{e}"))?;
    let duplicate_code_migration_done: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 6)",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("無法檢查點位編號 migration：{e}"))?;
    if !duplicate_code_migration_done {
        connection
            .execute_batch(include_str!(
                "../migrations/0006_allow_duplicate_point_codes.sql"
            ))
            .map_err(|e| format!("無法允許重複點位編號：{e}"))?;
        connection
            .execute("INSERT INTO schema_migrations(version) VALUES (?1)", [6])
            .map_err(|e| format!("無法記錄點位編號 migration：{e}"))?;
    }
    apply_transactional_migration(
        &mut connection,
        7,
        include_str!("../migrations/0007_common_routes.sql"),
        "常用路線",
    )?;
    let personnel_assignment_migration_done: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 8)",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("無法檢查人力配置 migration：{e}"))?;
    if !personnel_assignment_migration_done {
        apply_transactional_migration(
            &mut connection,
            8,
            include_str!("../migrations/0008_personnel_assignments.sql"),
            "人力配置",
        )?;
    }
    let personnel_import_error_migration_done: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 9)",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("無法檢查人力匯入錯誤 migration：{e}"))?;
    if !personnel_import_error_migration_done {
        apply_transactional_migration(
            &mut connection,
            9,
            include_str!("../migrations/0009_personnel_import_errors.sql"),
            "人力匯入錯誤",
        )?;
    }
    let cross_route_assignment_migration_done: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 10)",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("無法檢查跨路線人力配置 migration：{e}"))?;
    if !cross_route_assignment_migration_done {
        connection
            .execute_batch(include_str!(
                "../migrations/0010_allow_cross_route_assignments.sql"
            ))
            .map_err(|e| format!("無法套用跨路線人力配置 migration：{e}"))?;
        connection
            .execute("INSERT INTO schema_migrations(version) VALUES (?1)", [10])
            .map_err(|e| format!("無法記錄跨路線人力配置 migration：{e}"))?;
    }
    let personnel_phone_migration_done: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 11)",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("無法檢查人員電話 migration：{e}"))?;
    if !personnel_phone_migration_done {
        connection
            .execute_batch(include_str!("../migrations/0011_personnel_phone.sql"))
            .map_err(|e| format!("無法套用人員電話 migration：{e}"))?;
        connection
            .execute("INSERT INTO schema_migrations(version) VALUES (?1)", [11])
            .map_err(|e| format!("無法記錄人員電話 migration：{e}"))?;
    }
    let deployment_equipment_migration_done: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 12)",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("無法檢查裝備配置 migration：{e}"))?;
    if !deployment_equipment_migration_done {
        connection
            .execute_batch(include_str!("../migrations/0012_deployment_equipment.sql"))
            .map_err(|e| format!("無法套用裝備配置 migration：{e}"))?;
        connection
            .execute("INSERT INTO schema_migrations(version) VALUES (?1)", [12])
            .map_err(|e| format!("無法記錄裝備配置 migration：{e}"))?;
    }
    let workspace_state_migration_done: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 13)",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("無法檢查工作區狀態 migration：{e}"))?;
    if !workspace_state_migration_done {
        connection
            .execute_batch(include_str!("../migrations/0013_workspace_state.sql"))
            .map_err(|e| format!("無法套用工作區狀態 migration：{e}"))?;
        connection
            .execute("INSERT INTO schema_migrations(version) VALUES (?1)", [13])
            .map_err(|e| format!("無法記錄工作區狀態 migration：{e}"))?;
    }
    let route_line_style_migration_done: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 14)",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("無法檢查路線樣式 migration：{e}"))?;
    if !route_line_style_migration_done {
        connection
            .execute_batch(include_str!("../migrations/0014_route_line_style.sql"))
            .map_err(|e| format!("無法套用路線樣式 migration：{e}"))?;
        connection
            .execute("INSERT INTO schema_migrations(version) VALUES (?1)", [14])
            .map_err(|e| format!("無法記錄路線樣式 migration：{e}"))?;
    }
    let workspace_deployment_route_migration_done: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 15)",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("無法檢查部署表工作區 migration：{e}"))?;
    if !workspace_deployment_route_migration_done {
        connection
            .execute_batch(include_str!(
                "../migrations/0015_workspace_deployment_route.sql"
            ))
            .map_err(|e| format!("無法套用部署表工作區 migration：{e}"))?;
        connection
            .execute("INSERT INTO schema_migrations(version) VALUES (?1)", [15])
            .map_err(|e| format!("無法記錄部署表工作區 migration：{e}"))?;
    }
    let duty_point_type_migration_done: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 16)",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("無法檢查號誌點位 migration：{e}"))?;
    if !duty_point_type_migration_done {
        connection
            .execute_batch(include_str!("../migrations/0016_duty_point_type.sql"))
            .map_err(|e| format!("無法套用號誌點位 migration：{e}"))?;
        connection
            .execute("INSERT INTO schema_migrations(version) VALUES (?1)", [16])
            .map_err(|e| format!("無法記錄號誌點位 migration：{e}"))?;
    }
    let custom_basemap_migration_done: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 17)",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("無法檢查自選底圖 migration：{e}"))?;
    if !custom_basemap_migration_done {
        connection
            .execute_batch(include_str!("../migrations/0017_custom_basemap_mode.sql"))
            .map_err(|e| format!("無法套用自選底圖 migration：{e}"))?;
        connection
            .execute("INSERT INTO schema_migrations(version) VALUES (?1)", [17])
            .map_err(|e| format!("無法記錄自選底圖 migration：{e}"))?;
    }
    apply_transactional_migration(
        &mut connection,
        18,
        include_str!("../migrations/0018_minimize_personnel_raw_data.sql"),
        "人員資料最小化",
    )?;
    seed_personnel(&connection)?;
    let mut check = connection
        .prepare("PRAGMA foreign_key_check")
        .map_err(|error| format!("無法執行資料庫完整性檢查：{error}"))?;
    let mut rows = check
        .query([])
        .map_err(|error| format!("無法讀取資料庫完整性檢查：{error}"))?;
    if rows
        .next()
        .map_err(|error| format!("無法讀取資料庫完整性檢查：{error}"))?
        .is_some()
    {
        return Err("資料庫外鍵完整性檢查失敗；已保留升版前備份。".to_owned());
    }
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
    let choices = serde_json::to_string(&input.deployment_choices)
        .map_err(|error| format!("無法保存部署表選項：{error}"))?;
    connection.execute("INSERT INTO workspace_states(plan_id, active_nav, selected_point_id, selected_route_id, deployment_route_id, deployment_choices_json, map_output_title, map_output_zoom, map_output_bearing, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, CURRENT_TIMESTAMP) ON CONFLICT(plan_id) DO UPDATE SET active_nav = excluded.active_nav, selected_point_id = excluded.selected_point_id, selected_route_id = excluded.selected_route_id, deployment_route_id = excluded.deployment_route_id, deployment_choices_json = excluded.deployment_choices_json, map_output_title = excluded.map_output_title, map_output_zoom = excluded.map_output_zoom, map_output_bearing = excluded.map_output_bearing, updated_at = CURRENT_TIMESTAMP", params![input.plan_id, input.active_nav, input.selected_point_id, input.selected_route_id, input.deployment_route_id, choices, input.map_output_title, input.map_output_zoom, input.map_output_bearing]).map_err(|error| format!("無法保存勤務工作區狀態：{error}"))?;
    Ok(())
}

pub fn delete_workspace_state(path: &Path, plan_id: &str) -> Result<(), String> {
    let connection = open_database(path)?;
    connection
        .execute("DELETE FROM workspace_states WHERE plan_id = ?1", [plan_id])
        .map_err(|error| format!("無法刪除勤務工作區快取：{error}"))?;
    Ok(())
}

pub fn clear_workspace_states(path: &Path) -> Result<(), String> {
    let connection = open_database(path)?;
    connection
        .execute("DELETE FROM workspace_states", [])
        .map_err(|error| format!("無法清除勤務工作區快取：{error}"))?;
    Ok(())
}

fn seed_personnel(connection: &Connection) -> Result<(), String> {
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM personnel", [], |row| row.get(0))
        .map_err(|error| format!("無法檢查人力種子資料：{error}"))?;
    if count > 0 {
        return sync_seed_personnel_phones(connection);
    }
    let batch_id: String = connection
        .query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
        .map_err(|error| format!("無法建立人力匯入批次：{error}"))?;
    connection.execute("INSERT INTO import_batches(id, source_file_name, total_rows, accepted_rows, rejected_rows) VALUES (?1, 'personnel-sample.csv', 56, 56, 0)", [batch_id.as_str()]).map_err(|error| format!("無法建立人力匯入批次：{error}"))?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| format!("無法建立人力匯入交易：{error}"))?;
    for row in include_str!("../../data/seeds/personnel-sample.csv")
        .lines()
        .skip(1)
    {
        let columns = row.split(',').collect::<Vec<_>>();
        if columns.len() != 6 {
            return Err("人力種子資料欄位不完整。".to_owned());
        }
        let id: String = transaction
            .query_row("SELECT lower(hex(randomblob(16)))", [], |result| {
                result.get(0)
            })
            .map_err(|error| format!("無法建立人員識別碼：{error}"))?;
        transaction.execute("INSERT INTO personnel(id, personnel_code, radio_code, name, title, unit, phone, import_batch_id, raw_row_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL)", params![id, columns[0], columns[1], columns[2], columns[3], columns[4], columns[5], batch_id]).map_err(|error| format!("無法匯入人力種子資料：{error}"))?;
    }
    transaction
        .commit()
        .map_err(|error| format!("無法完成種子人力匯入：{error}"))
}

fn sync_seed_personnel_phones(connection: &Connection) -> Result<(), String> {
    for row in include_str!("../../data/seeds/personnel-sample.csv")
        .lines()
        .skip(1)
    {
        let columns = row.split(',').collect::<Vec<_>>();
        if columns.len() == 6 {
            connection
                .execute(
                    "UPDATE personnel SET phone = ?2 WHERE personnel_code = ?1 AND phone = ''",
                    params![columns[0], columns[5]],
                )
                .map_err(|error| format!("無法更新種子人員電話：{error}"))?;
        }
    }
    Ok(())
}

pub fn list_duty_routes(path: &Path, plan_id: &str) -> Result<Vec<DutyRoute>, String> {
    let connection = open_database(path)?;
    let mut statement = connection.prepare(
        "SELECT routes.id, routes.plan_id, routes.route_name, routes.color, routes.route_type, routes.geometry_json, routes.line_style, stops.point_id
         FROM duty_routes AS routes
         LEFT JOIN duty_route_stops AS stops ON stops.route_id = routes.id
         WHERE routes.plan_id = ?1
         ORDER BY routes.created_at, stops.stop_order",
    ).map_err(|e| format!("無法讀取勤務路線：{e}"))?;
    let rows = statement
        .query_map([plan_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get(6)?,
                row.get::<_, Option<String>>(7)?,
            ))
        })
        .map_err(|e| format!("無法查詢勤務路線：{e}"))?;
    let mut routes = Vec::new();
    let mut current: Option<DutyRoute> = None;
    for row in rows {
        let (id, route_plan_id, route_name, color, route_type, geometry_json, line_style, point_id) =
            row.map_err(|e| format!("無法讀取勤務路線：{e}"))?;
        if current.as_ref().map(|route| route.id.as_str()) != Some(id.as_str()) {
            if let Some(route) = current.take() {
                routes.push(route);
            }
            let geometry = geometry_json
                .map(|json| {
                    serde_json::from_str(&json).map_err(|e| format!("無法讀取手繪路線：{e}"))
                })
                .transpose()?;
            current = Some(DutyRoute {
                id,
                plan_id: route_plan_id,
                route_name,
                color,
                point_ids: Vec::new(),
                route_type,
                geometry,
                line_style,
            });
        }
        if let Some(point_id) = point_id {
            current
                .as_mut()
                .expect("route should be initialized")
                .point_ids
                .push(point_id);
        }
    }
    if let Some(route) = current {
        routes.push(route);
    }
    Ok(routes)
}

pub fn create_duty_route(path: &Path, input: CreateDutyRouteInput) -> Result<DutyRoute, String> {
    validate_text(&input.route_name, "路線名稱")?;
    if input.point_ids.len() < 2 {
        return Err("請至少選擇兩個勤務點位。".to_owned());
    }
    if !supported_color(&input.color) {
        return Err("不支援的路線顏色。".to_owned());
    }
    if !["solid", "dashed", "arrow", "dashed_arrow"].contains(&input.line_style.as_str()) {
        return Err("僅支援實線、虛線、實箭頭線或虛箭頭線。".to_owned());
    }
    let mut connection = open_database(path)?;
    let tx = connection
        .transaction()
        .map_err(|e| format!("無法建立路線交易：{e}"))?;
    for point_id in &input.point_ids {
        let belongs: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM duty_points WHERE id = ?1 AND plan_id = ?2)",
                params![point_id, input.plan_id],
                |row| row.get(0),
            )
            .map_err(|error| format!("無法驗證勤務點位：{error}"))?;
        if !belongs {
            return Err("路線只能包含目前勤務計畫的點位。".to_owned());
        }
    }
    let id: String = tx
        .query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
        .map_err(|e| format!("無法建立路線識別碼：{e}"))?;
    tx.execute("INSERT INTO duty_routes(id, plan_id, route_name, color, line_style) VALUES (?1, ?2, ?3, ?4, ?5)", params![id, input.plan_id, input.route_name.trim(), input.color, input.line_style]).map_err(|e| format!("無法保存勤務路線：{e}"))?;
    for (index, point_id) in input.point_ids.iter().enumerate() {
        tx.execute(
            "INSERT INTO duty_route_stops(route_id, point_id, stop_order) VALUES (?1, ?2, ?3)",
            params![id, point_id, index as i64],
        )
        .map_err(|e| format!("無法保存路線點位：{e}"))?;
    }
    tx.commit()
        .map_err(|e| format!("無法完成勤務路線保存：{e}"))?;
    Ok(DutyRoute {
        id,
        plan_id: input.plan_id,
        route_name: input.route_name.trim().to_owned(),
        color: input.color,
        point_ids: input.point_ids,
        route_type: "point_sequence".to_owned(),
        geometry: None,
        line_style: input.line_style,
    })
}

pub fn create_manual_route(
    path: &Path,
    input: CreateManualRouteInput,
) -> Result<DutyRoute, String> {
    validate_text(&input.route_name, "路線名稱")?;
    validate_geometry(&input.geometry)?;
    if !supported_color(&input.color) {
        return Err("不支援的路線顏色。".to_owned());
    }
    let connection = open_database(path)?;
    let id: String = connection
        .query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))
        .map_err(|e| format!("無法建立路線識別碼：{e}"))?;
    let geometry_json =
        serde_json::to_string(&input.geometry).map_err(|e| format!("無法保存手繪路線：{e}"))?;
    connection.execute("INSERT INTO duty_routes(id, plan_id, route_name, color, route_type, geometry_json) VALUES (?1,?2,?3,?4,'manual',?5)", params![id,input.plan_id,input.route_name.trim(),input.color,geometry_json]).map_err(|e| format!("無法保存手繪路線：{e}"))?;
    Ok(DutyRoute {
        id,
        plan_id: input.plan_id,
        route_name: input.route_name.trim().to_owned(),
        color: input.color,
        point_ids: vec![],
        route_type: "manual".to_owned(),
        geometry: Some(input.geometry),
        line_style: "solid".to_owned(),
    })
}

pub fn delete_duty_route(path: &Path, route_id: &str) -> Result<(), String> {
    let connection = open_database(path)?;
    let deleted = connection
        .execute("DELETE FROM duty_routes WHERE id = ?1", [route_id])
        .map_err(|e| format!("無法刪除勤務路線：{e}"))?;
    if deleted == 0 {
        return Err("找不到要刪除的勤務路線。".to_owned());
    }
    Ok(())
}

pub fn update_duty_route_color(path: &Path, route_id: &str, color: &str) -> Result<(), String> {
    if !["red", "orange", "yellow", "green", "blue", "purple"].contains(&color) {
        return Err("不支援的路線顏色。".to_owned());
    }
    let connection = open_database(path)?;
    let updated = connection
        .execute(
            "UPDATE duty_routes SET color = ?2 WHERE id = ?1",
            params![route_id, color],
        )
        .map_err(|error| format!("無法更新路線顏色：{error}"))?;
    if updated == 0 {
        return Err("找不到要更新的勤務路線。".to_owned());
    }
    Ok(())
}

pub fn update_duty_route_line_style(
    path: &Path,
    route_id: &str,
    line_style: &str,
) -> Result<(), String> {
    if !["solid", "dashed", "arrow", "dashed_arrow"].contains(&line_style) {
        return Err("僅支援實線、虛線、實箭頭線或虛箭頭線。".to_owned());
    }
    let connection = open_database(path)?;
    let updated = connection
        .execute(
            "UPDATE duty_routes SET line_style = ?2 WHERE id = ?1",
            params![route_id, line_style],
        )
        .map_err(|error| format!("無法更新路線樣式：{error}"))?;
    if updated == 0 {
        return Err("找不到要更新的路線。".to_owned());
    }
    Ok(())
}

pub fn update_duty_route_name(path: &Path, route_id: &str, route_name: &str) -> Result<(), String> {
    let route_name = route_name.trim();
    if route_name.is_empty() {
        return Err("路線名稱不可空白。".to_owned());
    }
    let connection = open_database(path)?;
    let updated = connection
        .execute(
            "UPDATE duty_routes SET route_name = ?2 WHERE id = ?1",
            params![route_id, route_name],
        )
        .map_err(|error| format!("無法更新路線名稱：{error}"))?;
    if updated == 0 {
        return Err("找不到要更新的勤務路線。".to_owned());
    }
    Ok(())
}

pub fn list_common_routes(path: &Path) -> Result<Vec<CommonRoute>, String> {
    let connection = open_database(path)?;
    let mut statement = connection.prepare("SELECT id, route_name, color, geometry_json FROM common_routes ORDER BY created_at DESC").map_err(|error| format!("無法讀取常用路線：{error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|error| format!("無法查詢常用路線：{error}"))?;
    let common_routes = rows
        .map(|row| {
            let (id, route_name, color, geometry_json) =
                row.map_err(|error| format!("無法讀取常用路線：{error}"))?;
            let geometry = serde_json::from_str(&geometry_json)
                .map_err(|error| format!("無法讀取常用路線座標：{error}"))?;
            Ok(CommonRoute {
                id,
                route_name,
                color,
                geometry,
            })
        })
        .collect();
    common_routes
}

pub fn create_common_route(
    path: &Path,
    input: CreateCommonRouteInput,
) -> Result<CommonRoute, String> {
    validate_text(&input.route_name, "常用路線名稱")?;
    validate_geometry(&input.geometry)?;
    if !supported_color(&input.color) {
        return Err("不支援的路線顏色。".to_owned());
    }
    let connection = open_database(path)?;
    let id: String = connection
        .query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
        .map_err(|error| format!("無法建立常用路線識別碼：{error}"))?;
    let geometry_json = serde_json::to_string(&input.geometry)
        .map_err(|error| format!("無法保存常用路線：{error}"))?;
    connection.execute("INSERT INTO common_routes(id, route_name, color, geometry_json) VALUES (?1, ?2, ?3, ?4)", params![id, input.route_name.trim(), input.color, geometry_json]).map_err(|error| format!("無法保存常用路線：{error}"))?;
    Ok(CommonRoute {
        id,
        route_name: input.route_name.trim().to_owned(),
        color: input.color,
        geometry: input.geometry,
    })
}

pub fn delete_common_route(path: &Path, route_id: &str) -> Result<(), String> {
    let connection = open_database(path)?;
    let deleted = connection
        .execute("DELETE FROM common_routes WHERE id = ?1", [route_id])
        .map_err(|error| format!("無法刪除常用路線：{error}"))?;
    if deleted == 0 {
        return Err("找不到要刪除的常用路線。".to_owned());
    }
    Ok(())
}

pub fn list_personnel(path: &Path) -> Result<Vec<Personnel>, String> {
    let connection = open_database(path)?;
    let mut statement = connection.prepare("SELECT personnel.id, personnel.personnel_code, personnel.radio_code, personnel.name, personnel.title, personnel.unit, personnel.phone, COALESCE(import_batches.source_file_name = 'personnel-sample.csv', 0) FROM personnel LEFT JOIN import_batches ON import_batches.id = personnel.import_batch_id ORDER BY personnel.unit, personnel.title, personnel.personnel_code").map_err(|error| format!("無法讀取人員資料：{error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(Personnel {
                id: row.get(0)?,
                personnel_code: row.get(1)?,
                radio_code: row.get(2)?,
                name: row.get(3)?,
                title: row.get(4)?,
                unit: row.get(5)?,
                phone: row.get(6)?,
                is_sample: row.get(7)?,
            })
        })
        .map_err(|error| format!("無法查詢人員資料：{error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("無法讀取人員資料：{error}"))
}

pub fn clear_personnel(path: &Path) -> Result<(), String> {
    let mut connection = open_database(path)?;
    let transaction = connection
        .transaction()
        .map_err(|error| format!("無法建立清除人力資料交易：{error}"))?;
    transaction
        .execute("DELETE FROM personnel_assignments", [])
        .map_err(|error| format!("無法清除人力配置：{error}"))?;
    transaction
        .execute("DELETE FROM personnel", [])
        .map_err(|error| format!("無法清除人力資料：{error}"))?;
    transaction
        .execute("DELETE FROM import_batches", [])
        .map_err(|error| format!("無法清除人力匯入紀錄：{error}"))?;
    transaction
        .commit()
        .map_err(|error| format!("無法完成清除人力資料：{error}"))
}

pub fn latest_personnel_import_log(path: &Path) -> Result<Option<PersonnelImportLog>, String> {
    let connection = open_database(path)?;
    let batch = connection.query_row("SELECT id, source_file_name, total_rows, accepted_rows, rejected_rows FROM import_batches ORDER BY rowid DESC LIMIT 1", [], |row| Ok((row.get::<_, String>(0)?, PersonnelImportLog { source_file_name: row.get(1)?, total_rows: row.get(2)?, accepted_rows: row.get(3)?, rejected_rows: row.get(4)?, errors: Vec::new() }))).ok();
    let Some((batch_id, mut log)) = batch else {
        return Ok(None);
    };
    let mut statement = connection.prepare("SELECT row_number, error_reason, raw_row_json FROM personnel_import_errors WHERE import_batch_id = ?1 ORDER BY row_number LIMIT 100").map_err(|error| format!("無法讀取匯入錯誤紀錄：{error}"))?;
    log.errors = statement
        .query_map([batch_id], |row| {
            Ok(PersonnelImportError {
                row_number: row.get(0)?,
                error_reason: row.get(1)?,
                raw_row_json: row.get(2)?,
            })
        })
        .map_err(|error| format!("無法查詢匯入錯誤紀錄：{error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("無法讀取匯入錯誤紀錄：{error}"))?;
    Ok(Some(log))
}

pub fn list_personnel_assignments(
    path: &Path,
    plan_id: &str,
) -> Result<Vec<PersonnelAssignment>, String> {
    let connection = open_database(path)?;
    let mut statement = connection.prepare("SELECT id, plan_id, personnel_id, duty_point_id, assigned_unit, assigned_title FROM personnel_assignments WHERE plan_id = ?1 ORDER BY created_at").map_err(|error| format!("無法讀取人力配置：{error}"))?;
    let rows = statement
        .query_map([plan_id], |row| {
            Ok(PersonnelAssignment {
                id: row.get(0)?,
                plan_id: row.get(1)?,
                personnel_id: row.get(2)?,
                duty_point_id: row.get(3)?,
                assigned_unit: row.get(4)?,
                assigned_title: row.get(5)?,
            })
        })
        .map_err(|error| format!("無法查詢人力配置：{error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("無法讀取人力配置：{error}"))
}

pub fn create_personnel_assignment(
    path: &Path,
    input: CreatePersonnelAssignmentInput,
) -> Result<PersonnelAssignment, String> {
    let connection = open_database(path)?;
    if let Some(point_id) = &input.duty_point_id {
        if !point_belongs_to_plan(&connection, &input.plan_id, point_id)? {
            return Err("人力只能配置至同一勤務計畫的點位。".to_owned());
        }
    }
    let id: String = connection
        .query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
        .map_err(|error| format!("無法建立人力配置識別碼：{error}"))?;
    connection.execute("INSERT INTO personnel_assignments(id, plan_id, personnel_id, duty_point_id, assigned_unit, assigned_title) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![id, input.plan_id, input.personnel_id, input.duty_point_id, input.assigned_unit, input.assigned_title]).map_err(|error| format!("無法配置人員：{error}"))?;
    Ok(PersonnelAssignment {
        id,
        plan_id: input.plan_id,
        personnel_id: input.personnel_id,
        duty_point_id: input.duty_point_id,
        assigned_unit: input.assigned_unit,
        assigned_title: input.assigned_title,
    })
}

pub fn delete_personnel_assignment(path: &Path, assignment_id: &str) -> Result<(), String> {
    let connection = open_database(path)?;
    if connection
        .execute(
            "DELETE FROM personnel_assignments WHERE id = ?1",
            [assignment_id],
        )
        .map_err(|error| format!("無法移除人力配置：{error}"))?
        == 0
    {
        return Err("找不到要移除的人力配置。".to_owned());
    }
    Ok(())
}

pub fn move_personnel_assignment(
    path: &Path,
    assignment_id: &str,
    duty_point_id: String,
) -> Result<(), String> {
    let connection = open_database(path)?;
    let plan_id: String = connection
        .query_row(
            "SELECT plan_id FROM personnel_assignments WHERE id = ?1",
            [assignment_id],
            |row| row.get(0),
        )
        .map_err(|_| "找不到要移動的人力配置。".to_owned())?;
    if !point_belongs_to_plan(&connection, &plan_id, &duty_point_id)? {
        return Err("人力只能移至同一勤務計畫的點位。".to_owned());
    }
    if connection
        .execute(
            "UPDATE personnel_assignments SET duty_point_id = ?2 WHERE id = ?1",
            params![assignment_id, duty_point_id],
        )
        .map_err(|error| format!("無法移動人力配置：{error}"))?
        == 0
    {
        return Err("找不到要移動的人力配置。".to_owned());
    }
    Ok(())
}

pub fn list_deployment_equipment(
    path: &Path,
    plan_id: &str,
) -> Result<Vec<DeploymentEquipment>, String> {
    let connection = open_database(path)?;
    let mut statement = connection.prepare("SELECT plan_id, duty_point_id, selected_items_json FROM deployment_equipment WHERE plan_id = ?1 ORDER BY duty_point_id").map_err(|error| format!("無法讀取部署裝備：{error}"))?;
    let rows = statement
        .query_map([plan_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| format!("無法查詢部署裝備：{error}"))?;
    rows.map(|row| {
        let (plan_id, duty_point_id, selected_items_json) =
            row.map_err(|error| format!("無法讀取部署裝備：{error}"))?;
        let selected_items = serde_json::from_str(&selected_items_json)
            .map_err(|error| format!("部署裝備資料格式錯誤：{error}"))?;
        Ok(DeploymentEquipment {
            plan_id,
            duty_point_id,
            selected_items,
        })
    })
    .collect()
}

pub fn save_deployment_equipment(
    path: &Path,
    input: SaveDeploymentEquipmentInput,
) -> Result<DeploymentEquipment, String> {
    let selected_items = input
        .selected_items
        .into_iter()
        .map(|item| item.trim().to_owned())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    if selected_items.len() > MAX_EQUIPMENT_ITEMS
        || selected_items
            .iter()
            .any(|item| item.chars().count() > MAX_TEXT_LENGTH)
    {
        return Err("部署裝備項目過多或文字過長。".to_owned());
    }
    let selected_items_json = serde_json::to_string(&selected_items)
        .map_err(|error| format!("無法保存部署裝備：{error}"))?;
    let connection = open_database(path)?;
    if !point_belongs_to_plan(&connection, &input.plan_id, &input.duty_point_id)? {
        return Err("部署裝備只能設定於同一勤務計畫的點位。".to_owned());
    }
    connection.execute(
        "INSERT INTO deployment_equipment(plan_id, duty_point_id, selected_items_json) VALUES (?1, ?2, ?3) ON CONFLICT(plan_id, duty_point_id) DO UPDATE SET selected_items_json = excluded.selected_items_json, updated_at = CURRENT_TIMESTAMP",
        params![input.plan_id, input.duty_point_id, selected_items_json],
    ).map_err(|error| format!("無法保存部署裝備：{error}"))?;
    Ok(DeploymentEquipment {
        plan_id: input.plan_id,
        duty_point_id: input.duty_point_id,
        selected_items,
    })
}

fn parse_csv_rows(file_data: &[u8]) -> Result<Vec<Vec<String>>, String> {
    let content = match std::str::from_utf8(file_data) {
        Ok(content) => content.to_owned(),
        Err(_) => {
            let (decoded, _, had_errors) = BIG5.decode(file_data);
            if had_errors {
                return Err("CSV 編碼無法讀取；請另存為 UTF-8、Big5 或使用 .xlsx。".to_owned());
            }
            decoded.into_owned()
        }
    };
    let content = content.trim_start_matches('\u{feff}');
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = content.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '"' if quoted && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => {
                row.push(field.trim().to_owned());
                field.clear();
            }
            '\n' if !quoted => {
                row.push(field.trim().to_owned());
                field.clear();
                rows.push(row);
                row = Vec::new();
            }
            '\r' if !quoted => {}
            value => field.push(value),
        }
    }
    if quoted {
        return Err("CSV 的雙引號格式不完整。".to_owned());
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field.trim().to_owned());
        rows.push(row);
    }
    Ok(rows)
}

pub fn import_personnel_xlsx(
    path: &Path,
    input: ImportPersonnelInput,
) -> Result<ImportPersonnelResult, String> {
    if input.file_data.len() > MAX_IMPORT_BYTES {
        return Err("人力匯入檔案不可超過 10 MB。".to_owned());
    }
    let file_name = input.file_name.to_lowercase();
    let rows = if file_name.ends_with(".xlsx") {
        let mut workbook = Xlsx::new(Cursor::new(input.file_data))
            .map_err(|error| format!("無法讀取 Excel：{error}"))?;
        let range = workbook
            .worksheet_range_at(0)
            .ok_or_else(|| "Excel 沒有工作表。".to_owned())?
            .map_err(|error| format!("無法讀取工作表：{error}"))?;
        range
            .rows()
            .map(|row| row.iter().map(|cell| cell.to_string()).collect::<Vec<_>>())
            .collect::<Vec<_>>()
    } else if file_name.ends_with(".csv") {
        parse_csv_rows(&input.file_data)?
    } else {
        return Err("僅接受 .csv 或 .xlsx 人力資料檔。".to_owned());
    };
    if rows.len() > MAX_IMPORT_ROWS {
        return Err(format!("人力資料不可超過 {MAX_IMPORT_ROWS} 列。"));
    }
    if rows
        .iter()
        .flatten()
        .any(|cell| cell.chars().count() > MAX_TEXT_LENGTH)
    {
        return Err(format!("人力資料欄位不可超過 {MAX_TEXT_LENGTH} 個字元。"));
    }
    let required = [
        "personnel_code",
        "radio_code",
        "name",
        "title",
        "unit",
        "phone",
    ];
    let aliases = |field: &str| match field {
        "personnel_code" => &["personnel_code", "personnel-number", "員編"][..],
        "radio_code" => &["radio_code", "radio", "無線電代號"][..],
        "name" => &["name", "姓名"][..],
        "title" => &["title", "職稱"][..],
        "unit" => &["unit", "所屬單位"][..],
        "phone" => &["phone", "聯絡電話"][..],
        _ => &[][..],
    };
    let (header_row_index, headers) = rows.iter().take(10).enumerate().find_map(|(index, row)| {
        let headers = row.iter().map(|cell| cell.to_string().trim().trim_start_matches('\u{feff}').to_owned()).collect::<Vec<_>>();
        let has_header = |field: &str| headers.iter().any(|header| aliases(field).contains(&header.as_str()));
        required.iter().all(|field| has_header(field)).then_some((index, headers))
    }).ok_or_else(|| "Excel 前 10 列必須包含：員編、無線電代號、姓名、職稱、所屬單位、聯絡電話（亦支援英文欄名）。".to_owned())?;
    let index_of = |field: &str| {
        headers
            .iter()
            .position(|header| aliases(field).contains(&header.as_str()))
            .expect("validated required header")
    };
    let connection = open_database(path)?;
    let batch_id: String = connection
        .query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
        .map_err(|error| format!("無法建立匯入批次：{error}"))?;
    connection
        .execute(
            "INSERT INTO import_batches(id, source_file_name) VALUES (?1, ?2)",
            params![batch_id, input.file_name],
        )
        .map_err(|error| format!("無法建立匯入批次：{error}"))?;
    let mut total_rows = 0usize;
    let mut accepted_rows = 0usize;
    let mut rejected_rows = 0usize;
    for (offset, row) in rows.iter().enumerate().skip(header_row_index + 1) {
        if row.iter().all(|cell| cell.to_string().trim().is_empty()) {
            continue;
        }
        total_rows += 1;
        let value = |field: &str| {
            row.get(index_of(field))
                .map(|cell| cell.to_string().trim().to_owned())
                .unwrap_or_default()
        };
        let personnel_code = value("personnel_code");
        let radio_code = value("radio_code");
        let name = value("name");
        let title = value("title");
        let unit = value("unit");
        let phone = value("phone");
        let raw_row_json = serde_json::json!({ "personnel_code": personnel_code, "radio_code": radio_code, "name": name, "title": title, "unit": unit, "phone": phone }).to_string();
        let error_reason = if personnel_code.is_empty()
            || radio_code.is_empty()
            || name.is_empty()
            || title.is_empty()
            || unit.is_empty()
            || phone.is_empty()
        {
            Some("必填欄位不可空白。".to_owned())
        } else {
            None
        };
        if let Some(reason) = error_reason {
            rejected_rows += 1;
            connection.execute("INSERT INTO personnel_import_errors(id, import_batch_id, row_number, raw_row_json, error_reason) VALUES (lower(hex(randomblob(16))), ?1, ?2, ?3, ?4)", params![batch_id, (offset + 1) as i64, raw_row_json, reason]).map_err(|error| format!("無法記錄匯入錯誤：{error}"))?;
            continue;
        }
        let radio_match: Option<String> = connection
            .query_row(
                "SELECT id FROM personnel WHERE radio_code = ?1",
                [&radio_code],
                |row| row.get(0),
            )
            .ok();
        let personnel_match: Option<String> = connection
            .query_row(
                "SELECT id FROM personnel WHERE personnel_code = ?1",
                [&personnel_code],
                |row| row.get(0),
            )
            .ok();
        let save_result = match (radio_match, personnel_match) {
            (Some(radio_id), Some(personnel_id)) if radio_id != personnel_id => Err("員編與無線電代號分別對應不同既有人員，無法安全更新。".to_owned()),
            (Some(id), _) | (_, Some(id)) => connection.execute("UPDATE personnel SET personnel_code = ?2, radio_code = ?3, name = ?4, title = ?5, unit = ?6, phone = ?7, import_batch_id = ?8, raw_row_json = NULL WHERE id = ?1", params![id, personnel_code, radio_code, name, title, unit, phone, batch_id]).map(|_| ()).map_err(|error| error.to_string()),
            (None, None) => {
                let id: String = connection.query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0)).map_err(|error| format!("無法建立人員識別碼：{error}"))?;
                connection.execute("INSERT INTO personnel(id, personnel_code, radio_code, name, title, unit, phone, import_batch_id, raw_row_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL)", params![id, personnel_code, radio_code, name, title, unit, phone, batch_id]).map(|_| ()).map_err(|error| error.to_string())
            }
        };
        match save_result {
            Ok(()) => accepted_rows += 1,
            Err(error) => {
                rejected_rows += 1;
                connection.execute("INSERT INTO personnel_import_errors(id, import_batch_id, row_number, raw_row_json, error_reason) VALUES (lower(hex(randomblob(16))), ?1, ?2, ?3, ?4)", params![batch_id, (offset + 1) as i64, raw_row_json, error]).map_err(|record_error| format!("無法記錄匯入錯誤：{record_error}"))?;
            }
        }
    }
    connection.execute("UPDATE import_batches SET total_rows = ?2, accepted_rows = ?3, rejected_rows = ?4 WHERE id = ?1", params![batch_id, total_rows as i64, accepted_rows as i64, rejected_rows as i64]).map_err(|error| format!("無法完成匯入批次：{error}"))?;
    Ok(ImportPersonnelResult {
        total_rows,
        accepted_rows,
        rejected_rows,
    })
}

pub fn list_duty_points(path: &Path, plan_id: &str) -> Result<Vec<DutyPoint>, String> {
    let connection = open_database(path)?;
    let mut statement = connection.prepare("SELECT id, plan_id, point_code, point_name, note, color, point_type, latitude, longitude, coordinate_x, coordinate_y, visible FROM duty_points WHERE plan_id = ?1 ORDER BY point_code").map_err(|e| format!("無法讀取勤務點位：{e}"))?;
    let points = statement
        .query_map([plan_id], |r| {
            Ok(DutyPoint {
                id: r.get(0)?,
                plan_id: r.get(1)?,
                point_code: r.get(2)?,
                point_name: r.get(3)?,
                note: r.get(4)?,
                color: r.get(5)?,
                point_type: r.get(6)?,
                latitude: r.get(7)?,
                longitude: r.get(8)?,
                coordinate_x: r.get(9)?,
                coordinate_y: r.get(10)?,
                visible: r.get::<_, i64>(11)? != 0,
            })
        })
        .map_err(|e| format!("無法查詢勤務點位：{e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("無法讀取勤務點位資料：{e}"))?;
    Ok(points)
}

pub fn create_duty_point(path: &Path, input: CreateDutyPointInput) -> Result<DutyPoint, String> {
    validate_text(&input.point_code, "點位編號")?;
    validate_text(&input.point_name, "點位名稱")?;
    if !supported_color(&input.color) {
        return Err("不支援的點位顏色。".to_owned());
    }
    validate_coordinates(
        input.latitude,
        input.longitude,
        input.coordinate_x,
        input.coordinate_y,
    )?;
    let connection = open_database(path)?;
    let id: String = connection
        .query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
        .map_err(|e| format!("無法建立點位識別碼：{e}"))?;
    if !["duty", "hollow", "signal"].contains(&input.point_type.as_str()) {
        return Err("不支援的點位類型。".to_owned());
    }
    connection.execute("INSERT INTO duty_points(id, plan_id, point_code, point_name, note, color, point_type, latitude, longitude, coordinate_x, coordinate_y) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)", params![id,input.plan_id,input.point_code.trim(),input.point_name.trim(),input.note,input.color,input.point_type,input.latitude,input.longitude,input.coordinate_x,input.coordinate_y]).map_err(|e| format!("無法保存勤務點位：{e}"))?;
    connection.query_row("SELECT id, plan_id, point_code, point_name, note, color, point_type, latitude, longitude, coordinate_x, coordinate_y, visible FROM duty_points WHERE id=?1", [id], |r| Ok(DutyPoint { id:r.get(0)?, plan_id:r.get(1)?, point_code:r.get(2)?, point_name:r.get(3)?, note:r.get(4)?, color:r.get(5)?, point_type:r.get(6)?, latitude:r.get(7)?, longitude:r.get(8)?, coordinate_x:r.get(9)?, coordinate_y:r.get(10)?, visible:r.get::<_, i64>(11)? != 0 })).map_err(|e| format!("勤務點位已保存，但無法讀回資料：{e}"))
}

pub fn delete_duty_point(path: &Path, point_id: &str) -> Result<(), String> {
    let connection = open_database(path)?;
    let deleted = connection
        .execute("DELETE FROM duty_points WHERE id = ?1", [point_id])
        .map_err(|e| format!("無法刪除勤務點位：{e}"))?;
    if deleted == 0 {
        return Err("找不到要刪除的勤務點位。".to_owned());
    }
    Ok(())
}

pub fn move_duty_point(
    path: &Path,
    point_id: &str,
    latitude: f64,
    longitude: f64,
    coordinate_x: Option<f64>,
    coordinate_y: Option<f64>,
) -> Result<(), String> {
    validate_coordinates(latitude, longitude, coordinate_x, coordinate_y)?;
    let connection = open_database(path)?;
    let updated = connection.execute("UPDATE duty_points SET latitude = ?2, longitude = ?3, coordinate_x = ?4, coordinate_y = ?5, updated_at = CURRENT_TIMESTAMP WHERE id = ?1", params![point_id, latitude, longitude, coordinate_x, coordinate_y]).map_err(|e| format!("無法移動勤務點位：{e}"))?;
    if updated == 0 {
        return Err("找不到要移動的勤務點位。".to_owned());
    }
    Ok(())
}

pub fn update_duty_point_name(path: &Path, point_id: &str, point_name: &str) -> Result<(), String> {
    let point_name = point_name.trim();
    if point_name.is_empty() {
        return Err("點位名稱不可空白。".to_owned());
    }
    let connection = open_database(path)?;
    let updated = connection
        .execute(
            "UPDATE duty_points SET point_name = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
            params![point_id, point_name],
        )
        .map_err(|e| format!("無法更新勤務點位名稱：{e}"))?;
    if updated == 0 {
        return Err("找不到要改名的勤務點位。".to_owned());
    }
    Ok(())
}

pub fn update_duty_point(
    path: &Path,
    point_id: &str,
    input: UpdateDutyPointInput,
) -> Result<DutyPoint, String> {
    let point_code = input.point_code.trim();
    let point_name = input.point_name.trim();
    validate_text(point_code, "點位編號")?;
    validate_text(point_name, "點位名稱")?;
    if !supported_color(&input.color) {
        return Err("不支援的點位顏色。".to_owned());
    }
    if !["duty", "hollow", "signal"].contains(&input.point_type.as_str()) {
        return Err("不支援的點位類型。".to_owned());
    }
    let connection = open_database(path)?;
    validate_coordinates(
        input.latitude,
        input.longitude,
        input.coordinate_x,
        input.coordinate_y,
    )?;
    let updated = connection.execute("UPDATE duty_points SET point_code = ?2, point_name = ?3, note = ?4, color = ?5, point_type = ?6, latitude = ?7, longitude = ?8, coordinate_x = ?9, coordinate_y = ?10, updated_at = CURRENT_TIMESTAMP WHERE id = ?1", params![point_id, point_code, point_name, input.note.filter(|note| !note.trim().is_empty()), input.color, input.point_type, input.latitude, input.longitude, input.coordinate_x, input.coordinate_y]).map_err(|error| format!("無法更新勤務點位：{error}"))?;
    if updated == 0 {
        return Err("找不到要更新的勤務點位。".to_owned());
    }
    connection.query_row("SELECT id, plan_id, point_code, point_name, note, color, point_type, latitude, longitude, coordinate_x, coordinate_y, visible FROM duty_points WHERE id=?1", [point_id], |r| Ok(DutyPoint { id:r.get(0)?, plan_id:r.get(1)?, point_code:r.get(2)?, point_name:r.get(3)?, note:r.get(4)?, color:r.get(5)?, point_type:r.get(6)?, latitude:r.get(7)?, longitude:r.get(8)?, coordinate_x:r.get(9)?, coordinate_y:r.get(10)?, visible:r.get::<_, i64>(11)? != 0 })).map_err(|error| format!("勤務點位已更新，但無法讀回資料：{error}"))
}

pub fn list_duty_plans(path: &Path) -> Result<Vec<DutyPlan>, String> {
    let connection = open_database(path)?;
    let mut statement = connection.prepare(
        "SELECT id, plan_name, duty_date, start_time, end_time, description, status, created_at, updated_at, plan_mode, basemap_path, basemap_width, basemap_height
         FROM duty_plans ORDER BY updated_at DESC, created_at DESC",
    ).map_err(|error| format!("無法讀取勤務計畫：{error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(DutyPlan {
                id: row.get(0)?,
                plan_name: row.get(1)?,
                duty_date: row.get(2)?,
                start_time: row.get(3)?,
                end_time: row.get(4)?,
                description: row.get(5)?,
                status: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
                plan_mode: row.get(9)?,
                basemap_path: row.get(10)?,
                basemap_width: row.get(11)?,
                basemap_height: row.get(12)?,
            })
        })
        .map_err(|error| format!("無法查詢勤務計畫：{error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("無法讀取勤務計畫資料：{error}"))
}

pub fn create_duty_plan(path: &Path, input: CreateDutyPlanInput) -> Result<DutyPlan, String> {
    let plan_name = input.plan_name.trim();
    if plan_name.is_empty() {
        return Err("勤務計畫名稱不可空白。".to_owned());
    }
    let plan_mode = input.plan_mode.as_deref().unwrap_or("map");
    if !["map", "custom_basemap"].contains(&plan_mode) {
        return Err("不支援的勤務模式。".to_owned());
    }
    let connection = open_database(path)?;
    let id: String = connection
        .query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
        .map_err(|error| format!("無法建立勤務計畫識別碼：{error}"))?;
    connection.execute(
        "INSERT INTO duty_plans(id, plan_name, duty_date, start_time, end_time, description, plan_mode, basemap_path, basemap_width, basemap_height)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![id, plan_name, input.duty_date, input.start_time, input.end_time, input.description, plan_mode, input.basemap_path, input.basemap_width, input.basemap_height],
    ).map_err(|error| format!("無法保存勤務計畫：{error}"))?;
    connection.query_row(
        "SELECT id, plan_name, duty_date, start_time, end_time, description, status, created_at, updated_at, plan_mode, basemap_path, basemap_width, basemap_height
         FROM duty_plans WHERE id = ?1", [id], |row| Ok(DutyPlan {
            id: row.get(0)?, plan_name: row.get(1)?, duty_date: row.get(2)?, start_time: row.get(3)?,
            end_time: row.get(4)?, description: row.get(5)?, status: row.get(6)?, created_at: row.get(7)?, updated_at: row.get(8)?, plan_mode: row.get(9)?, basemap_path: row.get(10)?, basemap_width: row.get(11)?, basemap_height: row.get(12)?,
        }),
    ).map_err(|error| format!("勤務計畫已保存，但無法讀回資料：{error}"))
}

pub fn delete_duty_plan(path: &Path, plan_id: &str) -> Result<(), String> {
    let connection = open_database(path)?;
    let deleted = connection
        .execute("DELETE FROM duty_plans WHERE id = ?1", [plan_id])
        .map_err(|error| format!("無法刪除勤務計畫：{error}"))?;
    if deleted == 0 {
        return Err("找不到要刪除的勤務計畫。".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_plan(path: &Path, name: &str) -> DutyPlan {
        create_duty_plan(
            path,
            CreateDutyPlanInput {
                plan_name: name.to_owned(),
                duty_date: None,
                start_time: None,
                end_time: None,
                description: None,
                plan_mode: None,
                basemap_path: None,
                basemap_width: None,
                basemap_height: None,
            },
        )
        .expect("plan should be saved")
    }

    fn create_point(path: &Path, plan_id: &str, code: &str) -> DutyPoint {
        create_duty_point(
            path,
            CreateDutyPointInput {
                plan_id: plan_id.to_owned(),
                point_code: code.to_owned(),
                point_name: format!("點位 {code}"),
                note: None,
                color: "blue".to_owned(),
                point_type: "duty".to_owned(),
                latitude: 25.0,
                longitude: 121.0,
                coordinate_x: None,
                coordinate_y: None,
            },
        )
        .expect("point should be saved")
    }
    #[test]
    fn migration_and_plan_creation_persist() {
        let path = std::env::temp_dir().join(format!("dutygrid-test-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        migrate(&path).expect("migration should succeed");
        create_duty_plan(
            &path,
            CreateDutyPlanInput {
                plan_name: "板橋勤務測試".to_owned(),
                duty_date: None,
                start_time: None,
                end_time: None,
                description: None,
                plan_mode: None,
                basemap_path: None,
                basemap_width: None,
                basemap_height: None,
            },
        )
        .expect("plan should be saved");
        assert_eq!(list_duty_plans(&path).expect("plans should load").len(), 1);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn backs_up_an_outdated_database_before_migration() {
        let directory =
            std::env::temp_dir().join(format!("dutygrid-backup-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("test directory should be created");
        let database_path = directory.join("dutygrid.db");
        let connection = Connection::open(&database_path).expect("database should open");
        connection.execute_batch("CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY); INSERT INTO schema_migrations(version) VALUES (16);").expect("outdated schema marker should be created");
        drop(connection);

        initialize_state(directory.clone()).expect("migration should complete");
        let backups = std::fs::read_dir(&directory)
            .expect("backup directory should be readable")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("dutygrid.pre-migration-")
            })
            .count();
        assert_eq!(backups, 1);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn upgrades_plaintext_database_to_sqlcipher_and_writes_metadata_only_audit_log() {
        let directory =
            std::env::temp_dir().join(format!("dutygrid-encryption-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("test directory should be created");
        let database_path = directory.join("dutygrid.db");
        let plaintext = Connection::open(&database_path).expect("plaintext database should open");
        plaintext
            .execute_batch(
                "CREATE TABLE source_data(value TEXT); INSERT INTO source_data VALUES ('secret');",
            )
            .expect("plaintext data should be written");
        drop(plaintext);

        initialize_state(directory.clone()).expect("plaintext database should be encrypted");
        assert!(Connection::open(&database_path)
            .and_then(|connection| connection.query_row(
                "SELECT value FROM source_data",
                [],
                |row| row.get::<_, String>(0)
            ))
            .is_err());
        let encrypted = open_database(&database_path).expect("encrypted database should open");
        let value: String = encrypted
            .query_row("SELECT value FROM source_data", [], |row| row.get(0))
            .expect("encrypted data should remain available");
        assert_eq!(value, "secret");

        append_audit_log(&directory, "read", "personnel", 1, true)
            .expect("audit should be written");
        let audit = std::fs::read_to_string(
            directory
                .join("logs")
                .read_dir()
                .expect("audit directory should exist")
                .next()
                .expect("audit file should exist")
                .expect("audit entry should be readable")
                .path(),
        )
        .expect("audit log should be readable");
        assert!(audit.contains("\"operation\":\"read\""));
        assert!(!audit.contains("secret"));
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn loads_route_stops_in_route_order_with_one_joined_query() {
        let path = std::env::temp_dir().join(format!(
            "dutygrid-route-list-test-{}.db",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        migrate(&path).expect("migration should succeed");
        let plan = create_plan(&path, "路線測試");
        let first = create_point(&path, &plan.id, "A1");
        let second = create_point(&path, &plan.id, "A2");
        create_duty_route(
            &path,
            CreateDutyRouteInput {
                plan_id: plan.id.clone(),
                route_name: "點位路線".to_owned(),
                color: "blue".to_owned(),
                point_ids: vec![second.id.clone(), first.id.clone()],
                line_style: "solid".to_owned(),
            },
        )
        .expect("route should be saved");
        create_manual_route(
            &path,
            CreateManualRouteInput {
                plan_id: plan.id.clone(),
                route_name: "手繪路線".to_owned(),
                color: "red".to_owned(),
                geometry: vec![[121.0, 25.0], [121.1, 25.1]],
            },
        )
        .expect("manual route should be saved");
        let routes = list_duty_routes(&path, &plan.id).expect("routes should load");
        assert_eq!(routes.len(), 2);
        let point_route = routes
            .iter()
            .find(|route| route.route_name == "點位路線")
            .expect("point route should load");
        let manual_route = routes
            .iter()
            .find(|route| route.route_name == "手繪路線")
            .expect("manual route should load");
        assert_eq!(point_route.point_ids, vec![second.id, first.id]);
        assert_eq!(manual_route.point_ids, Vec::<String>::new());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rejects_cross_plan_route_and_assignment_links() {
        let path =
            std::env::temp_dir().join(format!("dutygrid-integrity-test-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        migrate(&path).expect("migration should succeed");
        let first = create_plan(&path, "計畫 A");
        let second = create_plan(&path, "計畫 B");
        let first_point = create_duty_point(
            &path,
            CreateDutyPointInput {
                plan_id: first.id.clone(),
                point_code: "A1".to_owned(),
                point_name: "點位 A".to_owned(),
                note: None,
                color: "blue".to_owned(),
                point_type: "duty".to_owned(),
                latitude: 25.0,
                longitude: 121.0,
                coordinate_x: None,
                coordinate_y: None,
            },
        )
        .expect("point should be saved");
        let second_point = create_duty_point(
            &path,
            CreateDutyPointInput {
                plan_id: second.id.clone(),
                point_code: "B1".to_owned(),
                point_name: "點位 B".to_owned(),
                note: None,
                color: "blue".to_owned(),
                point_type: "duty".to_owned(),
                latitude: 25.0,
                longitude: 121.1,
                coordinate_x: None,
                coordinate_y: None,
            },
        )
        .expect("point should be saved");
        let route = create_duty_route(
            &path,
            CreateDutyRouteInput {
                plan_id: first.id.clone(),
                route_name: "跨計畫".to_owned(),
                color: "blue".to_owned(),
                point_ids: vec![first_point.id.clone(), second_point.id.clone()],
                line_style: "solid".to_owned(),
            },
        );
        assert!(route.is_err());
        let personnel_id: String = open_database(&path)
            .expect("database should open")
            .query_row("SELECT id FROM personnel LIMIT 1", [], |row| row.get(0))
            .expect("seed personnel should exist");
        let assignment = create_personnel_assignment(
            &path,
            CreatePersonnelAssignmentInput {
                plan_id: first.id,
                personnel_id,
                duty_point_id: Some(second_point.id),
                assigned_unit: "單位".to_owned(),
                assigned_title: "職稱".to_owned(),
            },
        );
        assert!(assignment.is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rejects_invalid_point_and_route_coordinates() {
        let path = std::env::temp_dir().join(format!(
            "dutygrid-coordinate-test-{}.db",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        migrate(&path).expect("migration should succeed");
        let plan = create_plan(&path, "座標測試");
        let point = create_duty_point(
            &path,
            CreateDutyPointInput {
                plan_id: plan.id.clone(),
                point_code: "X".to_owned(),
                point_name: "錯誤座標".to_owned(),
                note: None,
                color: "blue".to_owned(),
                point_type: "duty".to_owned(),
                latitude: f64::NAN,
                longitude: 121.0,
                coordinate_x: None,
                coordinate_y: None,
            },
        );
        assert!(point.is_err());
        let route = create_manual_route(
            &path,
            CreateManualRouteInput {
                plan_id: plan.id,
                route_name: "錯誤路線".to_owned(),
                color: "blue".to_owned(),
                geometry: vec![[121.0, 25.0], [f64::INFINITY, 25.1]],
            },
        );
        assert!(route.is_err());
        let _ = std::fs::remove_file(path);
    }
    #[test]
    fn deployment_equipment_persists_per_point() {
        let path =
            std::env::temp_dir().join(format!("dutygrid-equipment-test-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        migrate(&path).expect("migration should succeed");
        let plan = create_duty_plan(
            &path,
            CreateDutyPlanInput {
                plan_name: "裝備測試".to_owned(),
                duty_date: None,
                start_time: None,
                end_time: None,
                description: None,
                plan_mode: None,
                basemap_path: None,
                basemap_width: None,
                basemap_height: None,
            },
        )
        .expect("plan should be saved");
        let point = create_duty_point(
            &path,
            CreateDutyPointInput {
                plan_id: plan.id.clone(),
                point_code: "901".to_owned(),
                point_name: "測試崗哨".to_owned(),
                note: None,
                color: "red".to_owned(),
                point_type: "signal".to_owned(),
                latitude: 25.0,
                longitude: 121.0,
                coordinate_x: None,
                coordinate_y: None,
            },
        )
        .expect("point should be saved");
        assert_eq!(point.point_type, "signal");
        save_deployment_equipment(
            &path,
            SaveDeploymentEquipmentInput {
                plan_id: plan.id.clone(),
                duty_point_id: point.id,
                selected_items: vec!["制服".to_owned(), "無線電(空氣導管耳機)".to_owned()],
            },
        )
        .expect("equipment should be saved");
        let saved = list_deployment_equipment(&path, &plan.id).expect("equipment should load");
        assert_eq!(saved[0].selected_items, ["制服", "無線電(空氣導管耳機)"]);
        let _ = std::fs::remove_file(path);
    }
    #[test]
    fn csv_personnel_import_accepts_chinese_headers_and_quoted_values() {
        let path = std::env::temp_dir().join(format!(
            "dutygrid-personnel-csv-test-{}.db",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        migrate(&path).expect("migration should succeed");
        let result = import_personnel_xlsx(&path, ImportPersonnelInput {
            file_name: "人力資料.csv".to_owned(),
            file_data: "\u{feff}員編,無線電代號,姓名,職稱,所屬單位,聯絡電話\nA001,R01,王小明,警員,第一分局,0912345678\nA002,R02,李小華,巡佐,\"第二,分局\",0987654321\n".as_bytes().to_vec(),
        }).expect("csv should import");
        assert_eq!(
            (
                result.total_rows,
                result.accepted_rows,
                result.rejected_rows
            ),
            (2, 2, 0)
        );
        let repeated = import_personnel_xlsx(&path, ImportPersonnelInput {
            file_name: "人力資料.csv".to_owned(),
            file_data: "員編,無線電代號,姓名,職稱,所屬單位,聯絡電話\nA001,R01,王小明,警員,第一分局,0912345678\nA002,R02,李小華,巡佐,第二分局,0987654321\n".as_bytes().to_vec(),
        }).expect("repeated csv should update existing personnel");
        assert_eq!(
            (
                repeated.total_rows,
                repeated.accepted_rows,
                repeated.rejected_rows
            ),
            (2, 2, 0)
        );
        let _ = std::fs::remove_file(path);
    }
}
