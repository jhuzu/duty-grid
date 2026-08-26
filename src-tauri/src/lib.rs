mod database;

use std::path::PathBuf;

use database::{AppState, CommonRoute, CreateCommonRouteInput, CreateDutyPlanInput, CreateDutyPointInput, CreateDutyRouteInput, CreateManualRouteInput, CreatePersonnelAssignmentInput, DutyPlan, DutyPoint, DutyRoute, ImportPersonnelInput, ImportPersonnelResult, Personnel, PersonnelAssignment, RoadReference};
use rust_xlsxwriter::{Format, FormatAlign, FormatBorder, Workbook};
use serde::Deserialize;
use tauri::{Manager, State};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeploymentExportRow {
    sequence: u32,
    point_name: String,
    unit: String,
    police_count: usize,
    personnel_text: String,
    radio_text: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeploymentExportInput {
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
fn export_deployment_xlsx(input: DeploymentExportInput) -> Result<Vec<u8>, String> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("安全維護部署表").map_err(|error| format!("無法設定工作表名稱：{error}"))?;
    let title = Format::new().set_bold().set_font_size(18.0).set_align(FormatAlign::Center).set_align(FormatAlign::VerticalCenter);
    let centered = Format::new().set_align(FormatAlign::Center).set_align(FormatAlign::VerticalCenter).set_text_wrap().set_border(FormatBorder::Thin);
    let header = Format::new().set_bold().set_align(FormatAlign::Center).set_align(FormatAlign::VerticalCenter).set_text_wrap().set_border(FormatBorder::Thin);
    worksheet.merge_range(0, 0, 0, 8, "安全維護部署表", &title).map_err(|error| format!("無法建立表頭：{error}"))?;
    let plan_label = format!("勤務計畫：{}", input.plan_name);
    worksheet.merge_range(1, 0, 1, 8, &plan_label, &centered).map_err(|error| format!("無法寫入勤務計畫：{error}"))?;
    worksheet.merge_range(2, 0, 2, 8, "勤務日期：________________　勤務時間：________________　承辦單位：________________", &centered).map_err(|error| format!("無法建立基本資料列：{error}"))?;
    worksheet.merge_range(3, 0, 3, 8, "本表第 7 列起依目前勤務點位與人力配置自動填入；服裝及協調欄位可於 Excel 中續填。", &centered).map_err(|error| format!("無法建立說明列：{error}"))?;
    let headers = ["編號", "崗哨別", "崗哨位置", "派遣單位", "警力", "職稱姓名", "無線電代號", "服裝及應勤裝備", "分（協調）區協調員電話"];
    for (column, label) in headers.iter().enumerate() { worksheet.write_string_with_format(5, column as u16, *label, &header).map_err(|error| format!("無法建立欄位：{error}"))?; }
    let widths = [7.0, 16.0, 24.0, 16.0, 8.0, 24.0, 16.0, 22.0, 22.0];
    for (column, width) in widths.iter().enumerate() { worksheet.set_column_width(column as u16, *width).map_err(|error| format!("無法設定欄寬：{error}"))?; }
    worksheet.set_row_height(0, 30.0).map_err(|error| format!("無法設定標題列：{error}"))?;
    worksheet.set_row_height(5, 72.0).map_err(|error| format!("無法設定欄位列：{error}"))?;
    for (offset, row) in input.rows.iter().enumerate() {
        let target = (6 + offset) as u32;
        let values = [row.sequence.to_string(), "".to_string(), row.point_name.clone(), row.unit.clone(), row.police_count.to_string(), row.personnel_text.clone(), row.radio_text.clone(), "制服、無線電（空氣導管耳機）、服務證".to_string(), "".to_string()];
        for (column, value) in values.iter().enumerate() { worksheet.write_string_with_format(target, column as u16, value, &centered).map_err(|error| format!("無法寫入部署資料：{error}"))?; }
        worksheet.set_row_height(target, 72.0).map_err(|error| format!("無法設定部署資料列：{error}"))?;
    }
    workbook.save_to_buffer().map_err(|error| format!("無法建立 Excel：{error}"))
}

fn development_reference_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/reference/banqiao_roads.db")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app.path().app_local_data_dir()?;
            let bundled_path = app.path().resolve("resources/banqiao_roads.db", tauri::path::BaseDirectory::Resource)?;
            let road_reference_path = if bundled_path.is_file() { bundled_path } else { development_reference_path() };
            app.manage(database::initialize_state(app_data_dir, road_reference_path).map_err(std::io::Error::other)?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![app_health, list_duty_plans, create_duty_plan, lookup_banqiao_intersection, list_duty_points, create_duty_point, delete_duty_point, move_duty_point, list_duty_routes, create_duty_route, create_manual_route, delete_duty_route, update_duty_route_color, update_duty_route_name, list_common_routes, create_common_route, delete_common_route, list_personnel, list_personnel_assignments, create_personnel_assignment, delete_personnel_assignment, move_personnel_assignment, import_personnel_xlsx, export_deployment_xlsx])
        .run(tauri::generate_context!())
        .expect("error while running DutyGrid");
}
