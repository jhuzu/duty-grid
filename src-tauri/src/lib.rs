mod database;

use std::path::PathBuf;

use database::{AppState, CreateDutyPlanInput, DutyPlan, RoadReference};
use tauri::{Manager, State};

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
        .invoke_handler(tauri::generate_handler![app_health, list_duty_plans, create_duty_plan, lookup_banqiao_intersection])
        .run(tauri::generate_context!())
        .expect("error while running DutyGrid");
}
