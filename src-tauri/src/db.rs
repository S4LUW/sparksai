//! Persistência local em SQLite: configurações, moldes, ideias salvas, roteiros e narrações.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

pub struct Db(pub Mutex<Connection>);

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS moldes (
    id               TEXT PRIMARY KEY,
    nome             TEXT NOT NULL,
    nicho            TEXT NOT NULL DEFAULT '',
    idioma           TEXT NOT NULL DEFAULT 'pt',
    tom              TEXT NOT NULL DEFAULT '',
    publico          TEXT NOT NULL DEFAULT '',
    estrutura        TEXT NOT NULL DEFAULT '',
    duracao_minutos  INTEGER NOT NULL DEFAULT 8,
    cta              TEXT NOT NULL DEFAULT '',
    voz_profile_id   TEXT NOT NULL DEFAULT '',
    criado_em        TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS ideias (
    video_id   TEXT PRIMARY KEY,
    nicho      TEXT NOT NULL,
    dados      TEXT NOT NULL,
    salvo_em   TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS roteiros (
    id         TEXT PRIMARY KEY,
    molde_id   TEXT NOT NULL,
    video_id   TEXT,
    titulo     TEXT NOT NULL,
    dados      TEXT NOT NULL,
    audio_path TEXT,
    criado_em  TEXT NOT NULL
);
"#;

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(SCHEMA)?;
    Ok(conn)
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ---------- Configurações ----------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    pub youtube_api_key: String,
    pub anthropic_api_key: String,
    pub claude_model: String,
    pub voicebox_url: String,
}

pub const DEFAULT_CLAUDE_MODEL: &str = "claude-opus-5-5";
pub const DEFAULT_VOICEBOX_URL: &str = "http://127.0.0.1:17493";

pub fn get_settings(conn: &Connection) -> rusqlite::Result<Settings> {
    let get = |k: &str| -> rusqlite::Result<Option<String>> {
        conn.query_row("SELECT value FROM settings WHERE key = ?1", [k], |r| r.get(0))
            .optional()
    };
    Ok(Settings {
        youtube_api_key: get("youtube_api_key")?.unwrap_or_default(),
        anthropic_api_key: get("anthropic_api_key")?.unwrap_or_default(),
        claude_model: get("claude_model")?
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_CLAUDE_MODEL.into()),
        voicebox_url: get("voicebox_url")?
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_VOICEBOX_URL.into()),
    })
}

pub fn save_settings(conn: &Connection, s: &Settings) -> rusqlite::Result<()> {
    let pairs = [
        ("youtube_api_key", s.youtube_api_key.trim()),
        ("anthropic_api_key", s.anthropic_api_key.trim()),
        ("claude_model", s.claude_model.trim()),
        ("voicebox_url", s.voicebox_url.trim().trim_end_matches('/')),
    ];
    for (k, v) in pairs {
        conn.execute(
            "INSERT INTO settings(key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![k, v],
        )?;
    }
    Ok(())
}

// ---------- Moldes ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Molde {
    #[serde(default)]
    pub id: String,
    pub nome: String,
    pub nicho: String,
    pub idioma: String,
    pub tom: String,
    pub publico: String,
    pub estrutura: String,
    pub duracao_minutos: i64,
    pub cta: String,
    pub voz_profile_id: String,
    #[serde(default)]
    pub criado_em: String,
}

fn row_to_molde(r: &rusqlite::Row) -> rusqlite::Result<Molde> {
    Ok(Molde {
        id: r.get(0)?,
        nome: r.get(1)?,
        nicho: r.get(2)?,
        idioma: r.get(3)?,
        tom: r.get(4)?,
        publico: r.get(5)?,
        estrutura: r.get(6)?,
        duracao_minutos: r.get(7)?,
        cta: r.get(8)?,
        voz_profile_id: r.get(9)?,
        criado_em: r.get(10)?,
    })
}

const MOLDE_COLS: &str =
    "id, nome, nicho, idioma, tom, publico, estrutura, duracao_minutos, cta, voz_profile_id, criado_em";

pub fn list_moldes(conn: &Connection) -> rusqlite::Result<Vec<Molde>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {MOLDE_COLS} FROM moldes ORDER BY criado_em DESC"
    ))?;
    let rows = stmt.query_map([], row_to_molde)?;
    rows.collect()
}

pub fn get_molde(conn: &Connection, id: &str) -> rusqlite::Result<Option<Molde>> {
    conn.query_row(
        &format!("SELECT {MOLDE_COLS} FROM moldes WHERE id = ?1"),
        [id],
        row_to_molde,
    )
    .optional()
}

pub fn upsert_molde(conn: &Connection, mut m: Molde) -> rusqlite::Result<Molde> {
    if m.id.is_empty() {
        m.id = uuid::Uuid::new_v4().to_string();
    }
    if m.criado_em.is_empty() {
        m.criado_em = now();
    }
    conn.execute(
        "INSERT INTO moldes (id, nome, nicho, idioma, tom, publico, estrutura, duracao_minutos, cta, voz_profile_id, criado_em)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(id) DO UPDATE SET
            nome = excluded.nome, nicho = excluded.nicho, idioma = excluded.idioma,
            tom = excluded.tom, publico = excluded.publico, estrutura = excluded.estrutura,
            duracao_minutos = excluded.duracao_minutos, cta = excluded.cta,
            voz_profile_id = excluded.voz_profile_id",
        params![
            m.id, m.nome, m.nicho, m.idioma, m.tom, m.publico, m.estrutura,
            m.duracao_minutos, m.cta, m.voz_profile_id, m.criado_em
        ],
    )?;
    Ok(m)
}

pub fn delete_molde(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM moldes WHERE id = ?1", [id])?;
    Ok(())
}

// ---------- Ideias salvas ----------

pub fn save_ideia(conn: &Connection, video_id: &str, nicho: &str, dados: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO ideias (video_id, nicho, dados, salvo_em) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(video_id) DO UPDATE SET dados = excluded.dados, nicho = excluded.nicho",
        params![video_id, nicho, dados, now()],
    )?;
    Ok(())
}

pub fn list_ideias(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT dados FROM ideias ORDER BY salvo_em DESC")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    rows.collect()
}

pub fn delete_ideia(conn: &Connection, video_id: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM ideias WHERE video_id = ?1", [video_id])?;
    Ok(())
}

// ---------- Roteiros ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoteiroRow {
    pub id: String,
    pub molde_id: String,
    pub video_id: Option<String>,
    pub titulo: String,
    pub dados: serde_json::Value,
    pub audio_path: Option<String>,
    pub criado_em: String,
}

fn row_to_roteiro(r: &rusqlite::Row) -> rusqlite::Result<RoteiroRow> {
    let dados: String = r.get(4)?;
    Ok(RoteiroRow {
        id: r.get(0)?,
        molde_id: r.get(1)?,
        video_id: r.get(2)?,
        titulo: r.get(3)?,
        dados: serde_json::from_str(&dados).unwrap_or(serde_json::Value::Null),
        audio_path: r.get(5)?,
        criado_em: r.get(6)?,
    })
}

const ROTEIRO_COLS: &str = "id, molde_id, video_id, titulo, dados, audio_path, criado_em";

pub fn insert_roteiro(conn: &Connection, r: &RoteiroRow) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO roteiros (id, molde_id, video_id, titulo, dados, audio_path, criado_em)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            r.id, r.molde_id, r.video_id, r.titulo,
            r.dados.to_string(), r.audio_path, r.criado_em
        ],
    )?;
    Ok(())
}

pub fn update_roteiro_dados(conn: &Connection, id: &str, titulo: &str, dados: &serde_json::Value) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE roteiros SET titulo = ?2, dados = ?3 WHERE id = ?1",
        params![id, titulo, dados.to_string()],
    )?;
    Ok(())
}

pub fn set_roteiro_audio(conn: &Connection, id: &str, path: &str) -> rusqlite::Result<()> {
    conn.execute("UPDATE roteiros SET audio_path = ?2 WHERE id = ?1", params![id, path])?;
    Ok(())
}

pub fn list_roteiros(conn: &Connection) -> rusqlite::Result<Vec<RoteiroRow>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {ROTEIRO_COLS} FROM roteiros ORDER BY criado_em DESC"
    ))?;
    let rows = stmt.query_map([], row_to_roteiro)?;
    rows.collect()
}

pub fn get_roteiro(conn: &Connection, id: &str) -> rusqlite::Result<Option<RoteiroRow>> {
    conn.query_row(
        &format!("SELECT {ROTEIRO_COLS} FROM roteiros WHERE id = ?1"),
        [id],
        row_to_roteiro,
    )
    .optional()
}

pub fn delete_roteiro(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM roteiros WHERE id = ?1", [id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(SCHEMA).unwrap();
        c
    }

    #[test]
    fn settings_defaults_and_roundtrip() {
        let c = mem();
        let s = get_settings(&c).unwrap();
        assert_eq!(s.claude_model, DEFAULT_CLAUDE_MODEL);
        assert_eq!(s.voicebox_url, DEFAULT_VOICEBOX_URL);

        save_settings(
            &c,
            &Settings {
                youtube_api_key: " yt ".into(),
                anthropic_api_key: "sk".into(),
                claude_model: "".into(),
                voicebox_url: "http://localhost:9999/".into(),
            },
        )
        .unwrap();
        let s = get_settings(&c).unwrap();
        assert_eq!(s.youtube_api_key, "yt");
        assert_eq!(s.claude_model, DEFAULT_CLAUDE_MODEL);
        assert_eq!(s.voicebox_url, "http://localhost:9999");
    }

    #[test]
    fn molde_crud() {
        let c = mem();
        let m = upsert_molde(
            &c,
            Molde {
                id: String::new(),
                nome: "Curiosidades".into(),
                nicho: "história".into(),
                idioma: "pt".into(),
                tom: "misterioso".into(),
                publico: "".into(),
                estrutura: "".into(),
                duracao_minutos: 10,
                cta: "".into(),
                voz_profile_id: "".into(),
                criado_em: String::new(),
            },
        )
        .unwrap();
        assert!(!m.id.is_empty());
        let mut m2 = m.clone();
        m2.nome = "Curiosidades 2".into();
        upsert_molde(&c, m2).unwrap();
        let all = list_moldes(&c).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].nome, "Curiosidades 2");
        delete_molde(&c, &m.id).unwrap();
        assert!(list_moldes(&c).unwrap().is_empty());
    }
}
