mod database;

use std::{fs::{self, File}, io::{Cursor, Read, Write}, path::PathBuf};

use database::{AppState, CommonRoute, CreateCommonRouteInput, CreateDutyPlanInput, CreateDutyPointInput, CreateDutyRouteInput, CreateManualRouteInput, CreatePersonnelAssignmentInput, DeploymentEquipment, DutyPlan, DutyPoint, DutyRoute, ImportPersonnelInput, ImportPersonnelResult, Personnel, PersonnelAssignment, RoadReference, SaveDeploymentEquipmentInput, SaveWorkspaceStateInput, WorkspaceState};
use serde::Deserialize;
use tauri::{Manager, State};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeploymentExportRow {
    sequence: u32,
    post_type: String,
    point_name: String,
    unit: String,
    police_count: usize,
    personnel_text: String,
    radio_text: String,
    equipment_text: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeploymentExportInput {
    #[allow(dead_code)]
    plan_name: String,
    rows: Vec<DeploymentExportRow>,
}

#[tauri::command]
fn app_health() -> &'static str {
    "ok"
}

#[tauri::command]
fn list_duty_plans(state: State<'_, AppState>) -> Result<Vec<DutyPlan>, String> {
    database::list_duty_plans(&state.database_path)
}

#[tauri::command]
fn create_duty_plan(state: State<'_, AppState>, input: CreateDutyPlanInput) -> Result<DutyPlan, String> {
    database::create_duty_plan(&state.database_path, input)
}

#[tauri::command]
fn lookup_banqiao_intersection(state: State<'_, AppState>, road_name: String, cross_road_name: String) -> Result<Vec<RoadReference>, String> {
    database::lookup_intersection(&state.road_reference_path, &road_name, &cross_road_name)
}
#[tauri::command]
fn list_duty_points(state: State<'_, AppState>, plan_id: String) -> Result<Vec<DutyPoint>, String> { database::list_duty_points(&state.database_path, &plan_id) }
#[tauri::command]
fn create_duty_point(state: State<'_, AppState>, input: CreateDutyPointInput) -> Result<DutyPoint, String> { database::create_duty_point(&state.database_path, input) }
#[tauri::command]
fn delete_duty_point(state: State<'_, AppState>, point_id: String) -> Result<(), String> { database::delete_duty_point(&state.database_path, &point_id) }
#[tauri::command]
fn move_duty_point(state: State<'_, AppState>, point_id: String, latitude: f64, longitude: f64) -> Result<(), String> { database::move_duty_point(&state.database_path, &point_id, latitude, longitude) }
#[tauri::command]
fn list_duty_routes(state: State<'_, AppState>, plan_id: String) -> Result<Vec<DutyRoute>, String> { database::list_duty_routes(&state.database_path, &plan_id) }
#[tauri::command]
fn create_duty_route(state: State<'_, AppState>, input: CreateDutyRouteInput) -> Result<DutyRoute, String> { database::create_duty_route(&state.database_path, input) }
#[tauri::command]
fn create_manual_route(state: State<'_, AppState>, input: CreateManualRouteInput) -> Result<DutyRoute, String> { database::create_manual_route(&state.database_path, input) }
#[tauri::command]
fn delete_duty_route(state: State<'_, AppState>, route_id: String) -> Result<(), String> { database::delete_duty_route(&state.database_path, &route_id) }
#[tauri::command]
fn update_duty_route_color(state: State<'_, AppState>, route_id: String, color: String) -> Result<(), String> { database::update_duty_route_color(&state.database_path, &route_id, &color) }
#[tauri::command]
fn update_duty_route_line_style(state: State<'_, AppState>, route_id: String, line_style: String) -> Result<(), String> { database::update_duty_route_line_style(&state.database_path, &route_id, &line_style) }
#[tauri::command]
fn update_duty_route_name(state: State<'_, AppState>, route_id: String, route_name: String) -> Result<(), String> { database::update_duty_route_name(&state.database_path, &route_id, &route_name) }
#[tauri::command]
fn list_common_routes(state: State<'_, AppState>) -> Result<Vec<CommonRoute>, String> { database::list_common_routes(&state.database_path) }
#[tauri::command]
fn create_common_route(state: State<'_, AppState>, input: CreateCommonRouteInput) -> Result<CommonRoute, String> { database::create_common_route(&state.database_path, input) }
#[tauri::command]
fn delete_common_route(state: State<'_, AppState>, route_id: String) -> Result<(), String> { database::delete_common_route(&state.database_path, &route_id) }
#[tauri::command]
fn list_personnel(state: State<'_, AppState>) -> Result<Vec<Personnel>, String> { database::list_personnel(&state.database_path) }
#[tauri::command]
fn list_personnel_assignments(state: State<'_, AppState>, plan_id: String) -> Result<Vec<PersonnelAssignment>, String> { database::list_personnel_assignments(&state.database_path, &plan_id) }
#[tauri::command]
fn create_personnel_assignment(state: State<'_, AppState>, input: CreatePersonnelAssignmentInput) -> Result<PersonnelAssignment, String> { database::create_personnel_assignment(&state.database_path, input) }
#[tauri::command]
fn delete_personnel_assignment(state: State<'_, AppState>, assignment_id: String) -> Result<(), String> { database::delete_personnel_assignment(&state.database_path, &assignment_id) }
#[tauri::command]
fn move_personnel_assignment(state: State<'_, AppState>, assignment_id: String, duty_point_id: String) -> Result<(), String> { database::move_personnel_assignment(&state.database_path, &assignment_id, duty_point_id) }
#[tauri::command]
fn import_personnel_xlsx(state: State<'_, AppState>, input: ImportPersonnelInput) -> Result<ImportPersonnelResult, String> { database::import_personnel_xlsx(&state.database_path, input) }
#[tauri::command]
fn list_deployment_equipment(state: State<'_, AppState>, plan_id: String) -> Result<Vec<DeploymentEquipment>, String> { database::list_deployment_equipment(&state.database_path, &plan_id) }
#[tauri::command]
fn save_deployment_equipment(state: State<'_, AppState>, input: SaveDeploymentEquipmentInput) -> Result<DeploymentEquipment, String> { database::save_deployment_equipment(&state.database_path, input) }
#[tauri::command]
fn load_workspace_state(state: State<'_, AppState>, plan_id: String) -> Result<Option<WorkspaceState>, String> { database::load_workspace_state(&state.database_path, &plan_id) }
#[tauri::command]
fn save_workspace_state(state: State<'_, AppState>, input: SaveWorkspaceStateInput) -> Result<(), String> { database::save_workspace_state(&state.database_path, input) }

fn xml_escape(value: &str) -> String { value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;") }
fn replace_cell_value(xml: &mut String, cell: &str, style: u8, value: &str) -> Result<(), String> {
    let needle = format!("<c r=\"{cell}\"");
    let start = xml.find(&needle).ok_or_else(|| format!("範本缺少儲存格 {cell}"))?;
    let tag_end = xml[start..].find('>').ok_or_else(|| format!("範本儲存格格式錯誤：{cell}"))? + start;
    let end = if xml[start..=tag_end].ends_with("/>") { tag_end + 1 } else { xml[tag_end..].find("</c>").ok_or_else(|| format!("範本儲存格格式錯誤：{cell}"))? + tag_end + 4 };
    let replacement = if value.is_empty() { format!("<c r=\"{cell}\" s=\"{style}\"/>") } else { format!("<c r=\"{cell}\" s=\"{style}\" t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>", xml_escape(value)) };
    xml.replace_range(start..end, &replacement);
    Ok(())
}
fn deployment_template_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let bundled = app.path().resolve("resources/standard_deployment_template.xlsx", tauri::path::BaseDirectory::Resource).map_err(|error| format!("無法尋找 Excel 範本：{error}"))?;
    Ok(if bundled.is_file() { bundled } else { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../標準化部署表.xlsx") })
}
#[tauri::command]
fn export_deployment_xlsx(app: tauri::AppHandle, input: DeploymentExportInput) -> Result<Vec<u8>, String> {
    if input.rows.len() > 33 { return Err("勤務點位超過範本可填入的 33 列。".into()); }
    let file = File::open(deployment_template_path(&app)?).map_err(|error| format!("無法讀取 Excel 範本：{error}"))?;
    let mut source = ZipArchive::new(file).map_err(|error| format!("無法開啟 Excel 範本：{error}"))?;
    let mut worksheet = String::new();
    source.by_name("xl/worksheets/sheet1.xml").map_err(|error| format!("無法讀取部署表工作表：{error}"))?.read_to_string(&mut worksheet).map_err(|error| format!("無法讀取部署表資料：{error}"))?;
    for row_number in 7..=39 { for (column, style) in [("A", 7), ("B", 8), ("C", 8), ("D", 8), ("E", 9), ("F", 8), ("G", 8), ("H", 8)] { replace_cell_value(&mut worksheet, &format!("{column}{row_number}"), style, "")?; } }
    for (offset, row) in input.rows.iter().enumerate() { let number = offset + 7; for (column, style, value) in [("A", 7, row.sequence.to_string()), ("B", 8, row.post_type.clone()), ("C", 8, row.point_name.clone()), ("D", 8, row.unit.clone()), ("E", 9, row.police_count.to_string()), ("F", 8, row.personnel_text.clone()), ("G", 8, row.radio_text.clone()), ("H", 8, row.equipment_text.clone())] { replace_cell_value(&mut worksheet, &format!("{column}{number}"), style, &value)?; } }
    let mut output = ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..source.len() { let mut entry = source.by_index(index).map_err(|error| format!("無法讀取範本內容：{error}"))?; let name = entry.name().to_string(); output.start_file(&name, SimpleFileOptions::default().compression_method(CompressionMethod::Deflated)).map_err(|error| format!("無法建立匯出內容：{error}"))?; if name == "xl/worksheets/sheet1.xml" { output.write_all(worksheet.as_bytes()).map_err(|error| format!("無法寫入部署表資料：{error}"))?; } else { let mut bytes = Vec::new(); entry.read_to_end(&mut bytes).map_err(|error| format!("無法讀取範本內容：{error}"))?; output.write_all(&bytes).map_err(|error| format!("無法寫入範本內容：{error}"))?; } }
    Ok(output.finish().map_err(|error| format!("無法完成 Excel 匯出：{error}"))?.into_inner())
}

#[tauri::command]
fn save_exported_file(path: String, bytes: Vec<u8>) -> Result<(), String> {
    fs::write(path, bytes).map_err(|error| format!("無法儲存匯出檔案：{error}"))
}

fn development_reference_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/reference/banqiao_roads.db")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app.path().app_local_data_dir()?;
            let bundled_path = app.path().resolve("resources/banqiao_roads.db", tauri::path::BaseDirectory::Resource)?;
            let road_reference_path = if bundled_path.is_file() { bundled_path } else { development_reference_path() };
            app.manage(database::initialize_state(app_data_dir, road_reference_path).map_err(std::io::Error::other)?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![app_health, list_duty_plans, create_duty_plan, lookup_banqiao_intersection, list_duty_points, create_duty_point, delete_duty_point, move_duty_point, list_duty_routes, create_duty_route, create_manual_route, delete_duty_route, update_duty_route_color, update_duty_route_line_style, update_duty_route_name, list_common_routes, create_common_route, delete_common_route, list_personnel, list_personnel_assignments, create_personnel_assignment, delete_personnel_assignment, move_personnel_assignment, import_personnel_xlsx, list_deployment_equipment, save_deployment_equipment, load_workspace_state, save_workspace_state, export_deployment_xlsx, save_exported_file])
        .run(tauri::generate_context!())
        .expect("error while running DutyGrid");
}
