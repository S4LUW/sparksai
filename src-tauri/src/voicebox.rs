//! Narração local com o Voicebox (https://github.com/jamiepine/voicebox).
//!
//! O Voicebox roda na máquina do usuário e expõe uma API REST (padrão: http://127.0.0.1:17493):
//! - `GET  /profiles`              lista os perfis de voz
//! - `POST /generate`              enfileira uma geração e devolve o `id`
//! - `GET  /history/{id}`          mostra o status ("generating", "completed", "failed")
//! - `GET  /audio/{id}`            baixa o áudio pronto

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Idiomas aceitos pelo `POST /generate` do Voicebox.
pub const IDIOMAS: &[&str] = &[
    "zh", "en", "ja", "ko", "de", "fr", "ru", "pt", "es", "it", "he", "ar", "da", "el", "fi", "hi",
    "ms", "nl", "no", "pl", "sv", "sw", "tr",
];

/// O Voicebox aceita até 50 mil caracteres por pedido e divide o texto sozinho.
pub const MAX_CHARS: usize = 50_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Perfil {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub description: Option<String>,
}

pub fn normaliza_idioma(idioma: &str) -> String {
    let code = idioma.trim().to_lowercase();
    let code = code.split(['-', '_']).next().unwrap_or("").to_string();
    if IDIOMAS.contains(&code.as_str()) {
        code
    } else {
        "pt".into()
    }
}

fn err_msg(body: &Value) -> String {
    match &body["detail"] {
        Value::String(s) => s.clone(),
        Value::Null => "erro desconhecido".into(),
        other => other.to_string(),
    }
}

pub struct Voicebox {
    http: reqwest::Client,
    base: String,
}

impl Voicebox {
    pub fn new(http: reqwest::Client, base: &str) -> Self {
        Self { http, base: base.trim_end_matches('/').to_string() }
    }

    fn offline(&self, e: reqwest::Error) -> String {
        format!(
            "Não consegui falar com o Voicebox em {}. Ele está aberto? ({e})",
            self.base
        )
    }

    pub async fn perfis(&self) -> Result<Vec<Perfil>, String> {
        let resp = self
            .http
            .get(format!("{}/profiles", self.base))
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| self.offline(e))?;
        if !resp.status().is_success() {
            return Err(format!("Voicebox respondeu {}", resp.status()));
        }
        resp.json().await.map_err(|e| format!("Resposta inválida do Voicebox: {e}"))
    }

    /// Gera a narração e salva em `dir/{nome}.{ext}`. Devolve o caminho do arquivo.
    pub async fn narrar(
        &self,
        texto: &str,
        profile_id: &str,
        idioma: &str,
        dir: &Path,
        nome: &str,
    ) -> Result<PathBuf, String> {
        if profile_id.is_empty() {
            return Err("Escolha uma voz do Voicebox no molde antes de narrar.".into());
        }
        if texto.trim().is_empty() {
            return Err("O roteiro não tem texto para narrar.".into());
        }
        if texto.chars().count() > MAX_CHARS {
            return Err(format!("O texto passa de {MAX_CHARS} caracteres, o limite do Voicebox."));
        }

        let resp = self
            .http
            .post(format!("{}/generate", self.base))
            .json(&json!({
                "profile_id": profile_id,
                "text": texto,
                "language": normaliza_idioma(idioma),
            }))
            .timeout(Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| self.offline(e))?;
        let status = resp.status();
        let body: Value = resp.json().await.map_err(|e| format!("Resposta inválida do Voicebox: {e}"))?;
        if !status.is_success() {
            return Err(format!("Voicebox ({status}): {}", err_msg(&body)));
        }
        let id = body["id"].as_str().ok_or("O Voicebox não devolveu o id da geração.")?.to_string();

        // Narrações longas podem levar vários minutos em máquinas sem GPU.
        let mut tentativas = 0u32;
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            tentativas += 1;
            let h: Value = self
                .http
                .get(format!("{}/history/{id}", self.base))
                .timeout(Duration::from_secs(15))
                .send()
                .await
                .map_err(|e| self.offline(e))?
                .json()
                .await
                .map_err(|e| format!("Resposta inválida do Voicebox: {e}"))?;
            match h["status"].as_str().unwrap_or("completed") {
                "completed" => break,
                "failed" => {
                    let e = h["error"].as_str().unwrap_or("erro desconhecido");
                    return Err(format!("O Voicebox falhou ao gerar a voz: {e}"));
                }
                _ if tentativas > 1800 => {
                    return Err("A narração passou de 1 hora e foi abandonada.".into());
                }
                _ => {}
            }
        }

        let audio = self
            .http
            .get(format!("{}/audio/{id}", self.base))
            .timeout(Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| self.offline(e))?;
        if !audio.status().is_success() {
            return Err(format!("Não consegui baixar o áudio ({})", audio.status()));
        }
        let ext = match audio
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("audio/wav")
        {
            t if t.contains("mpeg") => "mp3",
            t if t.contains("ogg") => "ogg",
            t if t.contains("mp4") || t.contains("m4a") => "m4a",
            _ => "wav",
        };
        let bytes = audio.bytes().await.map_err(|e| format!("Falha ao baixar o áudio: {e}"))?;
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let path = dir.join(format!("{nome}.{ext}"));
        std::fs::write(&path, &bytes).map_err(|e| format!("Falha ao salvar o áudio: {e}"))?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idiomas() {
        assert_eq!(normaliza_idioma("pt-BR"), "pt");
        assert_eq!(normaliza_idioma("EN"), "en");
        assert_eq!(normaliza_idioma("xx"), "pt");
    }
}

/// Teste contra um Voicebox de verdade (ou um mock). Rode com:
/// `VOICEBOX_TEST_URL=http://127.0.0.1:17493 VOICEBOX_TEST_PROFILE=<id> cargo test -- --ignored voicebox_real`
#[cfg(test)]
mod integracao {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn voicebox_real() {
        let url = std::env::var("VOICEBOX_TEST_URL").expect("defina VOICEBOX_TEST_URL");
        let vb = Voicebox::new(reqwest::Client::new(), &url);
        let perfis = vb.perfis().await.unwrap();
        let profile = std::env::var("VOICEBOX_TEST_PROFILE")
            .unwrap_or_else(|_| perfis.first().expect("nenhum perfil").id.clone());
        let dir = std::env::temp_dir().join("sparksai-teste");
        let path = vb.narrar("Olá, isto é um teste do Sparks AI.", &profile, "pt-BR", &dir, "teste").await.unwrap();
        assert!(std::fs::metadata(&path).unwrap().len() > 0);
    }
}
