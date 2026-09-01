mod database;

use std::{
    fs::{self, File},
    io::{Cursor, Read, Write},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use database::{
    AppState, CommonRoute, CreateCommonRouteInput, CreateDutyPlanInput, CreateDutyPointInput,
    CreateDutyRouteInput, CreateManualRouteInput, CreatePersonnelAssignmentInput,
    DeploymentEquipment, DutyPlan, DutyPoint, DutyRoute, ImportPersonnelInput,
    ImportPersonnelResult, Personnel, PersonnelAssignment, PersonnelImportLog,
    SaveDeploymentEquipmentInput, SaveWorkspaceStateInput, UpdateDutyPointInput, WorkspaceState,
};
use serde::Deserialize;
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

const MAX_BASEMAP_BYTES: u64 = 20 * 1024 * 1024;
const MAX_PERSONNEL_BYTES: u64 = 10 * 1024 * 1024;
const MAX_WORKSPACE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_EXPORT_BYTES: usize = 50 * 1024 * 1024;

fn checked_regular_file(
    path: &PathBuf,
    allowed_extensions: &[&str],
    max_bytes: u64,
    label: &str,
) -> Result<(), String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| format!("{label}缺少副檔名。"))?;
    if !allowed_extensions.contains(&extension.as_str()) {
        return Err(format!("{label}格式不支援。"));
    }
    let metadata = fs::metadata(path).map_err(|error| format!("無法讀取{label}資訊：{error}"))?;
    if !metadata.is_file() {
        return Err(format!("{label}必須是一般檔案。"));
    }
    if metadata.len() > max_bytes {
        return Err(format!("{label}不可超過 {} MB。", max_bytes / 1024 / 1024));
    }
    Ok(())
}

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
    coordinator_phone: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeploymentExportInput {
    #[allow(dead_code)]
    plan_name: String,
    title: String,
    rows: Vec<DeploymentExportRow>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveGeneratedFileInput {
    suggested_name: String,
    extension: String,
    bytes: Vec<u8>,
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
fn create_duty_plan(
    state: State<'_, AppState>,
    input: CreateDutyPlanInput,
) -> Result<DutyPlan, String> {
    database::create_duty_plan(&state.database_path, input)
}

#[tauri::command]
fn delete_duty_plan(state: State<'_, AppState>, plan_id: String) -> Result<(), String> {
    database::delete_duty_plan(&state.database_path, &plan_id)
}

fn copy_custom_basemap(state: &AppState, source: PathBuf) -> Result<String, String> {
    checked_regular_file(
        &source,
        &["png", "jpg", "jpeg", "webp", "svg"],
        MAX_BASEMAP_BYTES,
        "底圖檔案",
    )?;
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .expect("checked extension");
    let directory = state.app_data_dir.join("custom-basemaps");
    fs::create_dir_all(&directory).map_err(|error| format!("無法建立底圖資料夾：{error}"))?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis();
    let destination = directory.join(format!("{stamp}.{extension}"));
    fs::copy(&source, &destination).map_err(|error| format!("無法複製底圖檔案：{error}"))?;
    Ok(destination.to_string_lossy().to_string())
}

#[tauri::command]
async fn select_custom_basemap(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Option<String>, String> {
    let selected = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("底圖圖片", &["png", "jpg", "jpeg", "webp", "svg"])
            .blocking_pick_file()
    })
    .await
    .map_err(|error| format!("無法開啟底圖選擇器：{error}"))?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected
        .into_path()
        .map_err(|error| format!("無法讀取底圖路徑：{error}"))?;
    copy_custom_basemap(&state, path).map(Some)
}

#[tauri::command]
fn import_guide_example_basemap(state: State<'_, AppState>) -> Result<String, String> {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/guide/example.jpg");
    copy_custom_basemap(&state, source)
}

#[tauri::command]
fn list_duty_points(state: State<'_, AppState>, plan_id: String) -> Result<Vec<DutyPoint>, String> {
    database::list_duty_points(&state.database_path, &plan_id)
}
#[tauri::command]
fn create_duty_point(
    state: State<'_, AppState>,
    input: CreateDutyPointInput,
) -> Result<DutyPoint, String> {
    database::create_duty_point(&state.database_path, input)
}
#[tauri::command]
fn delete_duty_point(state: State<'_, AppState>, point_id: String) -> Result<(), String> {
    database::delete_duty_point(&state.database_path, &point_id)
}
#[tauri::command]
fn move_duty_point(
    state: State<'_, AppState>,
    point_id: String,
    latitude: f64,
    longitude: f64,
    coordinate_x: Option<f64>,
    coordinate_y: Option<f64>,
) -> Result<(), String> {
    database::move_duty_point(
        &state.database_path,
        &point_id,
        latitude,
        longitude,
        coordinate_x,
        coordinate_y,
    )
}
#[tauri::command]
fn update_duty_point_name(
    state: State<'_, AppState>,
    point_id: String,
    point_name: String,
) -> Result<(), String> {
    database::update_duty_point_name(&state.database_path, &point_id, &point_name)
}
#[tauri::command]
fn update_duty_point(
    state: State<'_, AppState>,
    point_id: String,
    input: UpdateDutyPointInput,
) -> Result<DutyPoint, String> {
    database::update_duty_point(&state.database_path, &point_id, input)
}
#[tauri::command]
fn list_duty_routes(state: State<'_, AppState>, plan_id: String) -> Result<Vec<DutyRoute>, String> {
    database::list_duty_routes(&state.database_path, &plan_id)
}
#[tauri::command]
fn create_duty_route(
    state: State<'_, AppState>,
    input: CreateDutyRouteInput,
) -> Result<DutyRoute, String> {
    database::create_duty_route(&state.database_path, input)
}
#[tauri::command]
fn create_manual_route(
    state: State<'_, AppState>,
    input: CreateManualRouteInput,
) -> Result<DutyRoute, String> {
    database::create_manual_route(&state.database_path, input)
}
#[tauri::command]
fn delete_duty_route(state: State<'_, AppState>, route_id: String) -> Result<(), String> {
    database::delete_duty_route(&state.database_path, &route_id)
}
#[tauri::command]
fn update_duty_route_color(
    state: State<'_, AppState>,
    route_id: String,
    color: String,
) -> Result<(), String> {
    database::update_duty_route_color(&state.database_path, &route_id, &color)
}
#[tauri::command]
fn update_duty_route_line_style(
    state: State<'_, AppState>,
    route_id: String,
    line_style: String,
) -> Result<(), String> {
    database::update_duty_route_line_style(&state.database_path, &route_id, &line_style)
}
#[tauri::command]
fn update_duty_route_name(
    state: State<'_, AppState>,
    route_id: String,
    route_name: String,
) -> Result<(), String> {
    database::update_duty_route_name(&state.database_path, &route_id, &route_name)
}
#[tauri::command]
fn list_common_routes(state: State<'_, AppState>) -> Result<Vec<CommonRoute>, String> {
    database::list_common_routes(&state.database_path)
}
#[tauri::command]
fn create_common_route(
    state: State<'_, AppState>,
    input: CreateCommonRouteInput,
) -> Result<CommonRoute, String> {
    database::create_common_route(&state.database_path, input)
}
#[tauri::command]
fn delete_common_route(state: State<'_, AppState>, route_id: String) -> Result<(), String> {
    database::delete_common_route(&state.database_path, &route_id)
}
#[tauri::command]
fn list_personnel(state: State<'_, AppState>) -> Result<Vec<Personnel>, String> {
    database::list_personnel(&state.database_path)
}
#[tauri::command]
fn clear_personnel(state: State<'_, AppState>) -> Result<(), String> {
    database::clear_personnel(&state.database_path)
}
#[tauri::command]
fn list_personnel_assignments(
    state: State<'_, AppState>,
    plan_id: String,
) -> Result<Vec<PersonnelAssignment>, String> {
    database::list_personnel_assignments(&state.database_path, &plan_id)
}
#[tauri::command]
fn create_personnel_assignment(
    state: State<'_, AppState>,
    input: CreatePersonnelAssignmentInput,
) -> Result<PersonnelAssignment, String> {
    database::create_personnel_assignment(&state.database_path, input)
}
#[tauri::command]
fn delete_personnel_assignment(
    state: State<'_, AppState>,
    assignment_id: String,
) -> Result<(), String> {
    database::delete_personnel_assignment(&state.database_path, &assignment_id)
}
#[tauri::command]
fn move_personnel_assignment(
    state: State<'_, AppState>,
    assignment_id: String,
    duty_point_id: String,
) -> Result<(), String> {
    database::move_personnel_assignment(&state.database_path, &assignment_id, duty_point_id)
}
#[tauri::command]
fn import_personnel_xlsx(
    state: State<'_, AppState>,
    input: ImportPersonnelInput,
) -> Result<ImportPersonnelResult, String> {
    database::import_personnel_xlsx(&state.database_path, input)
}
fn import_personnel_from_path(
    database_path: &std::path::Path,
    file_path: PathBuf,
) -> Result<ImportPersonnelResult, String> {
    checked_regular_file(
        &file_path,
        &["xlsx", "csv"],
        MAX_PERSONNEL_BYTES,
        "人力資料檔",
    )?;
    let file_name = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "無法取得人力資料檔名。".to_owned())?
        .to_owned();
    let file_data = fs::read(&file_path).map_err(|error| format!("無法讀取人力資料檔：{error}"))?;
    database::import_personnel_xlsx(
        database_path,
        ImportPersonnelInput {
            file_name,
            file_data,
        },
    )
}
#[tauri::command]
async fn select_personnel_file(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Option<ImportPersonnelResult>, String> {
    let selected = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("人力資料", &["xlsx", "csv"])
            .blocking_pick_file()
    })
    .await
    .map_err(|error| format!("無法開啟人力資料選擇器：{error}"))?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected
        .into_path()
        .map_err(|error| format!("無法讀取人力資料路徑：{error}"))?;
    import_personnel_from_path(&state.database_path, path).map(Some)
}
#[tauri::command]
fn import_default_personnel_file(
    state: State<'_, AppState>,
) -> Result<ImportPersonnelResult, String> {
    let data_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data/seeds");
    let mut files = fs::read_dir(&data_directory)
        .map_err(|error| format!("無法讀取範例資料目錄 {}：{error}", data_directory.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && matches!(
                    path.extension()
                        .and_then(|extension| extension.to_str())
                        .map(str::to_ascii_lowercase)
                        .as_deref(),
                    Some("xlsx") | Some("csv")
                )
        })
        .collect::<Vec<_>>();
    files.sort_by_key(|path| {
        (
            if path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("xlsx"))
            {
                0
            } else {
                1
            },
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default(),
        )
    });
    let path = files.into_iter().next().ok_or_else(|| {
        format!(
            "找不到範例人力資料。請將 .xlsx 或 .csv 放到 {}；會優先讀取 .xlsx。",
            data_directory.display()
        )
    })?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "無法取得測試資料檔名。".to_owned())?
        .to_owned();
    let file_data = fs::read(&path).map_err(|error| format!("無法讀取範例人力資料檔：{error}"))?;
    database::import_personnel_xlsx(
        &state.database_path,
        ImportPersonnelInput {
            file_name,
            file_data,
        },
    )
}
#[tauri::command]
fn latest_personnel_import_log(
    state: State<'_, AppState>,
) -> Result<Option<PersonnelImportLog>, String> {
    database::latest_personnel_import_log(&state.database_path)
}
#[tauri::command]
fn list_deployment_equipment(
    state: State<'_, AppState>,
    plan_id: String,
) -> Result<Vec<DeploymentEquipment>, String> {
    database::list_deployment_equipment(&state.database_path, &plan_id)
}
#[tauri::command]
fn save_deployment_equipment(
    state: State<'_, AppState>,
    input: SaveDeploymentEquipmentInput,
) -> Result<DeploymentEquipment, String> {
    database::save_deployment_equipment(&state.database_path, input)
}
#[tauri::command]
fn load_workspace_state(
    state: State<'_, AppState>,
    plan_id: String,
) -> Result<Option<WorkspaceState>, String> {
    database::load_workspace_state(&state.database_path, &plan_id)
}
#[tauri::command]
fn save_workspace_state(
    state: State<'_, AppState>,
    input: SaveWorkspaceStateInput,
) -> Result<(), String> {
    database::save_workspace_state(&state.database_path, input)
}
#[tauri::command]
fn delete_workspace_state(state: State<'_, AppState>, plan_id: String) -> Result<(), String> {
    database::delete_workspace_state(&state.database_path, &plan_id)
}
#[tauri::command]
fn clear_workspace_states(state: State<'_, AppState>) -> Result<(), String> {
    database::clear_workspace_states(&state.database_path)
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
fn replace_cell_value(xml: &mut String, cell: &str, style: u8, value: &str) -> Result<(), String> {
    let needle = format!("<c r=\"{cell}\"");
    let start = xml
        .find(&needle)
        .ok_or_else(|| format!("範本缺少儲存格 {cell}"))?;
    let tag_end = xml[start..]
        .find('>')
        .ok_or_else(|| format!("範本儲存格格式錯誤：{cell}"))?
        + start;
    let end = if xml[start..=tag_end].ends_with("/>") {
        tag_end + 1
    } else {
        xml[tag_end..]
            .find("</c>")
            .ok_or_else(|| format!("範本儲存格格式錯誤：{cell}"))?
            + tag_end
            + 4
    };
    let replacement = if value.is_empty() {
        format!("<c r=\"{cell}\" s=\"{style}\"/>")
    } else {
        format!("<c r=\"{cell}\" s=\"{style}\" t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>", xml_escape(value))
    };
    xml.replace_range(start..end, &replacement);
    Ok(())
}
fn deployment_template_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let bundled = app
        .path()
        .resolve(
            "resources/standard_deployment_template.xlsx",
            tauri::path::BaseDirectory::Resource,
        )
        .map_err(|error| format!("無法尋找 Excel 範本：{error}"))?;
    Ok(if bundled.is_file() {
        bundled
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../標準化部署表.xlsx")
    })
}
#[tauri::command]
fn export_deployment_xlsx(
    app: tauri::AppHandle,
    input: DeploymentExportInput,
) -> Result<Vec<u8>, String> {
    if input.rows.len() > 33 {
        return Err("勤務點位超過範本可填入的 33 列。".into());
    }
    let file = File::open(deployment_template_path(&app)?)
        .map_err(|error| format!("無法讀取 Excel 範本：{error}"))?;
    let mut source =
        ZipArchive::new(file).map_err(|error| format!("無法開啟 Excel 範本：{error}"))?;
    let mut worksheet = String::new();
    source
        .by_name("xl/worksheets/sheet1.xml")
        .map_err(|error| format!("無法讀取部署表工作表：{error}"))?
        .read_to_string(&mut worksheet)
        .map_err(|error| format!("無法讀取部署表資料：{error}"))?;
    if !input.title.trim().is_empty() {
        replace_cell_value(&mut worksheet, "A1", 14, input.title.trim())?;
        replace_cell_value(&mut worksheet, "A2", 11, "")?;
    }
    for row_number in 7..=39 {
        for (column, style) in [
            ("A", 7),
            ("B", 8),
            ("C", 8),
            ("D", 8),
            ("E", 9),
            ("F", 8),
            ("G", 8),
            ("H", 8),
            ("I", 10),
        ] {
            replace_cell_value(&mut worksheet, &format!("{column}{row_number}"), style, "")?;
        }
    }
    for (offset, row) in input.rows.iter().enumerate() {
        let number = offset + 7;
        for (column, style, value) in [
            ("A", 7, row.sequence.to_string()),
            ("B", 8, row.post_type.clone()),
            ("C", 8, row.point_name.clone()),
            ("D", 8, row.unit.clone()),
            ("E", 9, row.police_count.to_string()),
            ("F", 8, row.personnel_text.clone()),
            ("G", 8, row.radio_text.clone()),
            ("H", 8, row.equipment_text.clone()),
            ("I", 10, row.coordinator_phone.clone()),
        ] {
            replace_cell_value(&mut worksheet, &format!("{column}{number}"), style, &value)?;
        }
    }
    let mut output = ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..source.len() {
        let mut entry = source
            .by_index(index)
            .map_err(|error| format!("無法讀取範本內容：{error}"))?;
        let name = entry.name().to_string();
        output
            .start_file(
                &name,
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .map_err(|error| format!("無法建立匯出內容：{error}"))?;
        if name == "xl/worksheets/sheet1.xml" {
            output
                .write_all(worksheet.as_bytes())
                .map_err(|error| format!("無法寫入部署表資料：{error}"))?;
        } else {
            let mut bytes = Vec::new();
            entry
                .read_to_end(&mut bytes)
                .map_err(|error| format!("無法讀取範本內容：{error}"))?;
            output
                .write_all(&bytes)
                .map_err(|error| format!("無法寫入範本內容：{error}"))?;
        }
    }
    Ok(output
        .finish()
        .map_err(|error| format!("無法完成 Excel 匯出：{error}"))?
        .into_inner())
}

fn validated_export_name(input: &SaveGeneratedFileInput) -> Result<(String, String), String> {
    if input.bytes.len() > MAX_EXPORT_BYTES {
        return Err("匯出檔案不可超過 50 MB。".to_owned());
    }
    let extension = input.extension.trim().to_ascii_lowercase();
    if !["xlsx", "png", "pdf", "json"].contains(&extension.as_str()) {
        return Err("不支援的匯出檔案格式。".to_owned());
    }
    let name = PathBuf::from(input.suggested_name.trim())
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "匯出檔名無效。".to_owned())?
        .to_owned();
    let name_extension = PathBuf::from(&name)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    if name_extension.as_deref() != Some(extension.as_str()) {
        return Err("匯出檔名與格式不一致。".to_owned());
    }
    Ok((name, extension))
}

#[tauri::command]
async fn save_generated_file(
    app: tauri::AppHandle,
    input: SaveGeneratedFileInput,
) -> Result<bool, String> {
    let (name, extension) = validated_export_name(&input)?;
    let dialog_extension = extension.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_file_name(&name)
            .add_filter("DutyGrid 匯出檔", &[dialog_extension.as_str()])
            .blocking_save_file()
    })
    .await
    .map_err(|error| format!("無法開啟儲存對話框：{error}"))?;
    let Some(selected) = selected else {
        return Ok(false);
    };
    let path = selected
        .into_path()
        .map_err(|error| format!("無法讀取儲存位置：{error}"))?;
    if path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
        != Some(extension.as_str())
    {
        return Err("選擇的檔案副檔名與匯出格式不一致。".to_owned());
    }
    fs::write(path, input.bytes).map_err(|error| format!("無法儲存匯出檔案：{error}"))?;
    Ok(true)
}
#[tauri::command]
async fn select_workspace_file(app: tauri::AppHandle) -> Result<Option<Vec<u8>>, String> {
    let selected = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("DutyGrid 工作區", &["json"])
            .blocking_pick_file()
    })
    .await
    .map_err(|error| format!("無法開啟工作區選擇器：{error}"))?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected
        .into_path()
        .map_err(|error| format!("無法讀取工作區路徑：{error}"))?;
    checked_regular_file(&path, &["json"], MAX_WORKSPACE_BYTES, "工作區檔案")?;
    fs::read(path)
        .map(Some)
        .map_err(|error| format!("無法讀取工作區檔案：{error}"))
}
#[tauri::command]
fn read_managed_basemap(state: State<'_, AppState>, path: String) -> Result<Vec<u8>, String> {
    let path = PathBuf::from(path)
        .canonicalize()
        .map_err(|error| format!("無法讀取底圖檔案：{error}"))?;
    let directory = state
        .app_data_dir
        .join("custom-basemaps")
        .canonicalize()
        .map_err(|error| format!("無法讀取底圖資料夾：{error}"))?;
    if !path.starts_with(&directory) {
        return Err("僅能讀取本程式管理的底圖檔案。".to_owned());
    }
    checked_regular_file(
        &path,
        &["png", "jpg", "jpeg", "webp", "svg"],
        MAX_BASEMAP_BYTES,
        "底圖檔案",
    )?;
    fs::read(path).map_err(|error| format!("無法讀取底圖檔案：{error}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app.path().app_local_data_dir()?;
            app.manage(database::initialize_state(app_data_dir).map_err(std::io::Error::other)?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_health,
            list_duty_plans,
            create_duty_plan,
            delete_duty_plan,
            select_custom_basemap,
            import_guide_example_basemap,
            list_duty_points,
            create_duty_point,
            delete_duty_point,
            move_duty_point,
            update_duty_point_name,
            update_duty_point,
            list_duty_routes,
            create_duty_route,
            create_manual_route,
            delete_duty_route,
            update_duty_route_color,
            update_duty_route_line_style,
            update_duty_route_name,
            list_common_routes,
            create_common_route,
            delete_common_route,
            list_personnel,
            clear_personnel,
            list_personnel_assignments,
            create_personnel_assignment,
            delete_personnel_assignment,
            move_personnel_assignment,
            import_personnel_xlsx,
            select_personnel_file,
            import_default_personnel_file,
            latest_personnel_import_log,
            list_deployment_equipment,
            save_deployment_equipment,
            load_workspace_state,
            save_workspace_state,
            delete_workspace_state,
            clear_workspace_states,
            export_deployment_xlsx,
            save_generated_file,
            select_workspace_file,
            read_managed_basemap
        ])
        .run(tauri::generate_context!())
        .expect("error while running DutyGrid");
}
