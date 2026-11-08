//! Caçada de outliers usando a YouTube Data API v3.
//!
//! Fluxo e custo de cota (padrão: 10.000 unidades/dia):
//! 1. `search.list` com o nicho (100 unidades).
//! 2. `videos.list` com as estatísticas dos resultados (1 unidade).
//! 3. `channels.list` para achar a playlist de uploads de cada canal (1 unidade por lote de 50).
//! 4. `playlistItems.list` + `videos.list` por canal para medir a mediana de views (2 unidades por canal).
//!
//! Uma busca de 25 vídeos custa em torno de 150 unidades.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

const API: &str = "https://www.googleapis.com/youtube/v3";

#[derive(Debug, Clone, Deserialize)]
pub struct SearchParams {
    pub nicho: String,
    /// Busca só vídeos publicados nos últimos N dias (0 = sem limite).
    #[serde(default)]
    pub dias: u32,
    /// "todos", "longos" ou "shorts".
    #[serde(default)]
    pub formato: String,
    /// Código ISO 639-1, ex.: "pt".
    #[serde(default)]
    pub idioma: String,
    /// Código ISO 3166-1, ex.: "BR".
    #[serde(default)]
    pub regiao: String,
    #[serde(default = "default_max")]
    pub max_resultados: u32,
}

fn default_max() -> u32 {
    25
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Outlier {
    pub video_id: String,
    pub titulo: String,
    pub descricao: String,
    pub canal_id: String,
    pub canal_nome: String,
    pub thumbnail: String,
    pub publicado_em: String,
    pub views: u64,
    pub likes: u64,
    pub comentarios: u64,
    pub duracao_segundos: u64,
    pub inscritos_canal: u64,
    /// Mediana de views dos uploads recentes do canal (sem o próprio vídeo).
    pub mediana_canal: u64,
    /// views / mediana_canal. Acima de 3 já é um outlier interessante.
    pub score: f64,
}

pub fn median(values: &mut [u64]) -> u64 {
    if values.is_empty() {
        return 0;
    }
    values.sort_unstable();
    let n = values.len();
    if n % 2 == 1 {
        values[n / 2]
    } else {
        (values[n / 2 - 1] + values[n / 2]) / 2
    }
}

/// Score de outlier: quantas vezes o vídeo superou a mediana do canal.
pub fn outlier_score(views: u64, mediana: u64) -> f64 {
    if mediana == 0 {
        return 0.0;
    }
    let s = views as f64 / mediana as f64;
    (s * 10.0).round() / 10.0
}

/// Converte durações ISO 8601 do YouTube (ex.: "PT1H2M3S", "P1DT2H") em segundos.
pub fn parse_duration(iso: &str) -> u64 {
    let mut total = 0u64;
    let mut num = 0u64;
    let mut in_time = false;
    for c in iso.chars() {
        match c {
            'P' => {}
            'T' => in_time = true,
            '0'..='9' => num = num * 10 + c.to_digit(10).unwrap() as u64,
            'D' => {
                total += num * 86_400;
                num = 0;
            }
            'H' if in_time => {
                total += num * 3600;
                num = 0;
            }
            'M' if in_time => {
                total += num * 60;
                num = 0;
            }
            'S' if in_time => {
                total += num;
                num = 0;
            }
            _ => num = 0,
        }
    }
    total
}

/// Shorts podem ter até 3 minutos.
pub fn passa_formato(formato: &str, duracao: u64) -> bool {
    match formato {
        "shorts" => duracao > 0 && duracao <= 180,
        "longos" => duracao > 180,
        _ => true,
    }
}

fn num(v: &Value) -> u64 {
    match v {
        Value::String(s) => s.parse().unwrap_or(0),
        Value::Number(n) => n.as_u64().unwrap_or(0),
        _ => 0,
    }
}

fn s(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}

pub struct YouTube {
    http: reqwest::Client,
    key: String,
}

impl YouTube {
    pub fn new(http: reqwest::Client, key: String) -> Self {
        Self { http, key }
    }

    async fn get(&self, path: &str, query: &[(&str, String)]) -> Result<Value, String> {
        let resp = self
            .http
            .get(format!("{API}/{path}"))
            .query(query)
            .query(&[("key", &self.key)])
            .send()
            .await
            .map_err(|e| format!("Falha de rede ao chamar o YouTube: {e}"))?;
        let status = resp.status();
        let body: Value = resp
            .json()
            .await
            .map_err(|e| format!("Resposta inválida do YouTube: {e}"))?;
        if !status.is_success() {
            let msg = body["error"]["message"].as_str().unwrap_or("erro desconhecido");
            return Err(format!("YouTube API ({status}): {msg}"));
        }
        Ok(body)
    }

    async fn videos(&self, ids: &[String]) -> Result<Vec<Value>, String> {
        let mut out = Vec::new();
        for chunk in ids.chunks(50) {
            let body = self
                .get(
                    "videos",
                    &[
                        ("part", "snippet,statistics,contentDetails".into()),
                        ("id", chunk.join(",")),
                        ("maxResults", "50".into()),
                    ],
                )
                .await?;
            if let Some(items) = body["items"].as_array() {
                out.extend(items.iter().cloned());
            }
        }
        Ok(out)
    }

    /// Retorna (playlist de uploads, inscritos) por canal.
    async fn channels(&self, ids: &[String]) -> Result<HashMap<String, (String, u64)>, String> {
        let mut out = HashMap::new();
        for chunk in ids.chunks(50) {
            let body = self
                .get(
                    "channels",
                    &[
                        ("part", "contentDetails,statistics".into()),
                        ("id", chunk.join(",")),
                        ("maxResults", "50".into()),
                    ],
                )
                .await?;
            for item in body["items"].as_array().cloned().unwrap_or_default() {
                let uploads = s(&item["contentDetails"]["relatedPlaylists"]["uploads"]);
                let subs = num(&item["statistics"]["subscriberCount"]);
                out.insert(s(&item["id"]), (uploads, subs));
            }
        }
        Ok(out)
    }

    /// Views dos últimos 30 uploads do canal.
    async fn recent_views(&self, uploads_playlist: &str) -> Result<Vec<(String, u64)>, String> {
        if uploads_playlist.is_empty() {
            return Ok(vec![]);
        }
        let body = self
            .get(
                "playlistItems",
                &[
                    ("part", "contentDetails".into()),
                    ("playlistId", uploads_playlist.into()),
                    ("maxResults", "30".into()),
                ],
            )
            .await;
        // Canais com playlist privada ou removida não devem derrubar a busca toda.
        let Ok(body) = body else { return Ok(vec![]) };
        let ids: Vec<String> = body["items"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|i| s(&i["contentDetails"]["videoId"]))
            .filter(|id| !id.is_empty())
            .collect();
        let vids = self.videos(&ids).await?;
        Ok(vids
            .iter()
            .map(|v| (s(&v["id"]), num(&v["statistics"]["viewCount"])))
            .collect())
    }

    pub async fn hunt(&self, p: &SearchParams) -> Result<Vec<Outlier>, String> {
        let nicho = p.nicho.trim();
        if nicho.is_empty() {
            return Err("Digite um nicho para buscar.".into());
        }
        let mut q: Vec<(&str, String)> = vec![
            ("part", "snippet".into()),
            ("type", "video".into()),
            ("q", nicho.into()),
            ("order", "viewCount".into()),
            ("maxResults", p.max_resultados.clamp(5, 50).to_string()),
        ];
        if p.dias > 0 {
            let after = chrono::Utc::now() - chrono::Duration::days(p.dias as i64);
            q.push(("publishedAfter", after.format("%Y-%m-%dT%H:%M:%SZ").to_string()));
        }
        if !p.idioma.is_empty() {
            q.push(("relevanceLanguage", p.idioma.clone()));
        }
        if !p.regiao.is_empty() {
            q.push(("regionCode", p.regiao.to_uppercase()));
        }
        // "short" na API = menos de 4 min. Longos são filtrados depois, pela duração real.
        if p.formato == "shorts" {
            q.push(("videoDuration", "short".into()));
        }

        let search = self.get("search", &q).await?;
        let ids: Vec<String> = search["items"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|i| s(&i["id"]["videoId"]))
            .filter(|id| !id.is_empty())
            .collect();
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let videos = self.videos(&ids).await?;
        let channel_ids: Vec<String> = videos
            .iter()
            .map(|v| s(&v["snippet"]["channelId"]))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let channels = self.channels(&channel_ids).await?;

        let mut recent: HashMap<String, Vec<(String, u64)>> = HashMap::new();
        for (cid, (uploads, _)) in &channels {
            recent.insert(cid.clone(), self.recent_views(uploads).await?);
        }

        let mut out = Vec::new();
        for v in &videos {
            let video_id = s(&v["id"]);
            let canal_id = s(&v["snippet"]["channelId"]);
            let duracao = parse_duration(v["contentDetails"]["duration"].as_str().unwrap_or(""));
            if !passa_formato(&p.formato, duracao) {
                continue;
            }
            let views = num(&v["statistics"]["viewCount"]);
            let mut outros: Vec<u64> = recent
                .get(&canal_id)
                .map(|r| r.iter().filter(|(id, _)| *id != video_id).map(|(_, v)| *v).collect())
                .unwrap_or_default();
            let mediana = median(&mut outros);
            let thumbs = &v["snippet"]["thumbnails"];
            let thumbnail = ["maxres", "high", "medium", "default"]
                .iter()
                .map(|k| s(&thumbs[*k]["url"]))
                .find(|u| !u.is_empty())
                .unwrap_or_default();
            out.push(Outlier {
                video_id,
                titulo: s(&v["snippet"]["title"]),
                descricao: s(&v["snippet"]["description"]),
                canal_nome: s(&v["snippet"]["channelTitle"]),
                thumbnail,
                publicado_em: s(&v["snippet"]["publishedAt"]),
                views,
                likes: num(&v["statistics"]["likeCount"]),
                comentarios: num(&v["statistics"]["commentCount"]),
                duracao_segundos: duracao,
                inscritos_canal: channels.get(&canal_id).map(|c| c.1).unwrap_or(0),
                mediana_canal: mediana,
                score: outlier_score(views, mediana),
                canal_id,
            });
        }
        out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_odd_even_empty() {
        assert_eq!(median(&mut []), 0);
        assert_eq!(median(&mut [5, 1, 3]), 3);
        assert_eq!(median(&mut [4, 1, 3, 2]), 2);
    }

    #[test]
    fn score() {
        assert_eq!(outlier_score(1_000_000, 100_000), 10.0);
        assert_eq!(outlier_score(150, 100), 1.5);
        assert_eq!(outlier_score(10, 0), 0.0);
    }

    #[test]
    fn durations() {
        assert_eq!(parse_duration("PT45S"), 45);
        assert_eq!(parse_duration("PT12M3S"), 723);
        assert_eq!(parse_duration("PT1H2M3S"), 3723);
        assert_eq!(parse_duration("P1DT1H"), 90_000);
        assert_eq!(parse_duration("P0D"), 0);
    }

    #[test]
    fn formatos() {
        assert!(passa_formato("shorts", 59));
        assert!(passa_formato("shorts", 180));
        assert!(!passa_formato("shorts", 181));
        assert!(passa_formato("longos", 600));
        assert!(!passa_formato("longos", 60));
        assert!(passa_formato("todos", 0));
    }
}
