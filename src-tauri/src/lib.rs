mod claude;
mod db;
mod voicebox;
mod youtube;

use std::path::PathBuf;

use db::{Db, Molde, RoteiroRow, Settings};
use tauri::{Manager, State};

struct AppState {
    db: Db,
    http: reqwest::Client,
    data_dir: PathBuf,
}

type Res<T> = Result<T, String>;

fn e<E: std::fmt::Display>(err: E) -> String {
    err.to_string()
}

impl AppState {
    fn settings(&self) -> Res<Settings> {
        db::get_settings(&*self.db.0.lock().map_err(e)?).map_err(e)
    }
}

// ---------- Configurações ----------

#[tauri::command]
fn get_settings(state: State<AppState>) -> Res<Settings> {
    state.settings()
}

#[tauri::command]
fn save_settings(state: State<AppState>, settings: Settings) -> Res<Settings> {
    let conn = state.db.0.lock().map_err(e)?;
    db::save_settings(&conn, &settings).map_err(e)?;
    db::get_settings(&conn).map_err(e)
}

// ---------- Caçada de ideias ----------

#[tauri::command]
async fn hunt_outliers(state: State<'_, AppState>, params: youtube::SearchParams) -> Res<Vec<youtube::Outlier>> {
    let s = state.settings()?;
    if s.youtube_api_key.is_empty() {
        return Err("Configure a chave da YouTube Data API em Configurações.".into());
    }
    youtube::YouTube::new(state.http.clone(), s.youtube_api_key).hunt(&params).await
}

#[tauri::command]
fn save_ideia(state: State<AppState>, ideia: youtube::Outlier, nicho: String) -> Res<()> {
    let conn = state.db.0.lock().map_err(e)?;
    db::save_ideia(&conn, &ideia.video_id, &nicho, &serde_json::to_string(&ideia).map_err(e)?).map_err(e)
}

#[tauri::command]
fn list_ideias(state: State<AppState>) -> Res<Vec<youtube::Outlier>> {
    let conn = state.db.0.lock().map_err(e)?;
    Ok(db::list_ideias(&conn)
        .map_err(e)?
        .iter()
        .filter_map(|d| serde_json::from_str(d).ok())
        .collect())
}

#[tauri::command]
fn delete_ideia(state: State<AppState>, video_id: String) -> Res<()> {
    db::delete_ideia(&*state.db.0.lock().map_err(e)?, &video_id).map_err(e)
}

// ---------- Moldes ----------

#[tauri::command]
fn list_moldes(state: State<AppState>) -> Res<Vec<Molde>> {
    db::list_moldes(&*state.db.0.lock().map_err(e)?).map_err(e)
}

#[tauri::command]
fn save_molde(state: State<AppState>, molde: Molde) -> Res<Molde> {
    if molde.nome.trim().is_empty() {
        return Err("Dê um nome ao molde.".into());
    }
    db::upsert_molde(&*state.db.0.lock().map_err(e)?, molde).map_err(e)
}

#[tauri::command]
fn delete_molde(state: State<AppState>, id: String) -> Res<()> {
    db::delete_molde(&*state.db.0.lock().map_err(e)?, &id).map_err(e)
}

// ---------- Roteiros ----------

#[tauri::command]
async fn gerar_roteiro(state: State<'_, AppState>, input: claude::RoteiroInput) -> Res<RoteiroRow> {
    let s = state.settings()?;
    let molde = {
        let conn = state.db.0.lock().map_err(e)?;
        db::get_molde(&conn, &input.molde_id).map_err(e)?.ok_or("Molde não encontrado.")?
    };
    if input.ideia.is_none() && input.tema.trim().is_empty() {
        return Err("Escolha uma ideia salva ou escreva um tema.".into());
    }
    let roteiro =
        claude::gerar_roteiro(&state.http, &s.anthropic_api_key, &s.claude_model, &molde, &input).await?;
    let row = RoteiroRow {
        id: uuid::Uuid::new_v4().to_string(),
        molde_id: molde.id,
        video_id: input.ideia.map(|i| i.video_id),
        titulo: roteiro.titulos.first().cloned().unwrap_or_else(|| "Sem título".into()),
        dados: serde_json::to_value(&roteiro).map_err(e)?,
        audio_path: None,
        criado_em: db::now(),
    };
    db::insert_roteiro(&*state.db.0.lock().map_err(e)?, &row).map_err(e)?;
    Ok(row)
}

#[tauri::command]
fn list_roteiros(state: State<AppState>) -> Res<Vec<RoteiroRow>> {
    db::list_roteiros(&*state.db.0.lock().map_err(e)?).map_err(e)
}

#[tauri::command]
fn save_roteiro(state: State<AppState>, id: String, titulo: String, dados: claude::Roteiro) -> Res<()> {
    let conn = state.db.0.lock().map_err(e)?;
    db::update_roteiro_dados(&conn, &id, &titulo, &serde_json::to_value(&dados).map_err(e)?).map_err(e)
}

#[tauri::command]
fn delete_roteiro(state: State<AppState>, id: String) -> Res<()> {
    let conn = state.db.0.lock().map_err(e)?;
    if let Some(r) = db::get_roteiro(&conn, &id).map_err(e)? {
        if let Some(p) = r.audio_path {
            let _ = std::fs::remove_file(p);
        }
    }
    db::delete_roteiro(&conn, &id).map_err(e)
}

// ---------- Voz (Voicebox) ----------

#[tauri::command]
async fn voicebox_perfis(state: State<'_, AppState>) -> Res<Vec<voicebox::Perfil>> {
    let s = state.settings()?;
    voicebox::Voicebox::new(state.http.clone(), &s.voicebox_url).perfis().await
}

#[tauri::command]
async fn narrar_roteiro(state: State<'_, AppState>, id: String) -> Res<RoteiroRow> {
    let s = state.settings()?;
    let (roteiro, molde) = {
        let conn = state.db.0.lock().map_err(e)?;
        let r = db::get_roteiro(&conn, &id).map_err(e)?.ok_or("Roteiro não encontrado.")?;
        let m = db::get_molde(&conn, &r.molde_id)
            .map_err(e)?
            .ok_or("O molde deste roteiro foi apagado.")?;
        (r, m)
    };
    let dados: claude::Roteiro = serde_json::from_value(roteiro.dados.clone()).map_err(e)?;
    let path = voicebox::Voicebox::new(state.http.clone(), &s.voicebox_url)
        .narrar(
            &dados.texto_narracao(),
            &molde.voz_profile_id,
            &molde.idioma,
            &state.data_dir.join("narracoes"),
            &roteiro.id,
        )
        .await?;
    let p = path.to_string_lossy().to_string();
    let conn = state.db.0.lock().map_err(e)?;
    db::set_roteiro_audio(&conn, &id, &p).map_err(e)?;
    db::get_roteiro(&conn, &id).map_err(e)?.ok_or_else(|| "Roteiro não encontrado.".into())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let conn = db::open(&data_dir.join("sparksai.db"))?;
            app.manage(AppState {
                db: Db(std::sync::Mutex::new(conn)),
                http: reqwest::Client::builder()
                    .user_agent(concat!("SparksAI/", env!("CARGO_PKG_VERSION")))
                    .build()?,
                data_dir,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            hunt_outliers,
            save_ideia,
            list_ideias,
            delete_ideia,
            list_moldes,
            save_molde,
            delete_molde,
            gerar_roteiro,
            list_roteiros,
            save_roteiro,
            delete_roteiro,
            voicebox_perfis,
            narrar_roteiro,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Sparks AI");
}
