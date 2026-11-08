//! Geração de roteiro com a API do Claude (Messages API, HTTP direto).
//!
//! A saída é forçada para JSON com `output_config.format` (structured outputs),
//! então o app sempre recebe título, seções, descrição e tags num formato previsível.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::db::Molde;
use crate::youtube::Outlier;

const URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2026-10-21";
/// Se um classificador de segurança recusar, a API refaz o pedido em outro modelo.
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

#[derive(Debug, Clone, Deserialize)]
pub struct RoteiroInput {
    pub molde_id: String,
    /// Vídeo outlier usado como inspiração (opcional).
    pub ideia: Option<Outlier>,
    /// Tema livre quando não há outlier, ou instruções extras.
    #[serde(default)]
    pub tema: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Secao {
    pub titulo: String,
    pub narracao: String,
    pub visual: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Roteiro {
    pub titulos: Vec<String>,
    pub gancho: String,
    pub secoes: Vec<Secao>,
    pub encerramento: String,
    pub descricao: String,
    pub tags: Vec<String>,
    pub ideia_thumbnail: String,
}

impl Roteiro {
    /// Texto corrido para a narração (gancho + seções + encerramento).
    pub fn texto_narracao(&self) -> String {
        let mut partes = vec![self.gancho.trim().to_string()];
        partes.extend(self.secoes.iter().map(|s| s.narracao.trim().to_string()));
        partes.push(self.encerramento.trim().to_string());
        partes.retain(|p| !p.is_empty());
        partes.join("\n\n")
    }
}

fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "titulos": {"type": "array", "items": {"type": "string"}},
            "gancho": {"type": "string"},
            "secoes": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "titulo": {"type": "string"},
                        "narracao": {"type": "string"},
                        "visual": {"type": "string"}
                    },
                    "required": ["titulo", "narracao", "visual"],
                    "additionalProperties": false
                }
            },
            "encerramento": {"type": "string"},
            "descricao": {"type": "string"},
            "tags": {"type": "array", "items": {"type": "string"}},
            "ideia_thumbnail": {"type": "string"}
        },
        "required": ["titulos", "gancho", "secoes", "encerramento", "descricao", "tags", "ideia_thumbnail"],
        "additionalProperties": false
    })
}

const SYSTEM: &str = "Você é roteirista de um canal de YouTube. Escreve roteiros originais, \
com fatos corretos, narração natural para ser lida em voz alta e ritmo que segura a atenção. \
Quando receber um vídeo de referência, use apenas o tema e o ângulo que fizeram ele funcionar: \
nunca copie o texto, a estrutura exata nem o título dele. Não invente dados, números ou citações; \
se não tiver certeza de algo, escreva de forma que não afirme o que você não sabe. \
Escreva tudo no idioma pedido.";

pub fn build_prompt(molde: &Molde, input: &RoteiroInput) -> String {
    let palavras = (molde.duracao_minutos.max(1) * 150).to_string();
    let mut p = format!(
        "Escreva o roteiro de um vídeo para este canal.\n\n\
         <molde>\n\
         Nome do canal: {}\nNicho: {}\nIdioma do vídeo: {}\nTom: {}\nPúblico: {}\n\
         Estrutura preferida: {}\nChamada para ação: {}\n\
         Duração alvo: {} minutos (cerca de {} palavras de narração no total)\n\
         </molde>\n",
        molde.nome,
        molde.nicho,
        molde.idioma,
        or_dash(&molde.tom),
        or_dash(&molde.publico),
        or_dash(&molde.estrutura),
        or_dash(&molde.cta),
        molde.duracao_minutos,
        palavras,
    );
    if let Some(i) = &input.ideia {
        let desc: String = i.descricao.chars().take(1500).collect();
        p.push_str(&format!(
            "\n<video_referencia>\n\
             Título: {}\nCanal: {}\nViews: {} ({}x a mediana do canal)\nDescrição: {}\n\
             </video_referencia>\n",
            i.titulo, i.canal_nome, i.views, i.score, desc
        ));
    }
    if !input.tema.trim().is_empty() {
        p.push_str(&format!("\n<instrucoes_extras>\n{}\n</instrucoes_extras>\n", input.tema.trim()));
    }
    p.push_str(
        "\nRegras do formato:\n\
         - titulos: 5 opções de título com até 70 caracteres.\n\
         - gancho: os primeiros 15 a 30 segundos, que fazem a pessoa continuar assistindo.\n\
         - secoes: de 4 a 8 seções. Em cada uma, `narracao` é o texto exato a ser lido, \
         e `visual` descreve as imagens ou cenas que acompanham a fala.\n\
         - encerramento: fechamento com a chamada para ação do molde.\n\
         - descricao: descrição do YouTube com 2 ou 3 parágrafos curtos.\n\
         - tags: de 10 a 15 tags.\n\
         - ideia_thumbnail: descrição da thumbnail, com o texto curto que vai nela.\n",
    );
    p
}

fn or_dash(s: &str) -> &str {
    if s.trim().is_empty() {
        "(livre)"
    } else {
        s
    }
}

/// Extrai o roteiro da resposta da Messages API.
pub fn parse_response(body: &Value) -> Result<Roteiro, String> {
    match body["stop_reason"].as_str() {
        Some("refusal") => {
            return Err("O Claude recusou este pedido. Tente reformular o tema.".into());
        }
        Some("max_tokens") => {
            return Err("O roteiro ficou longo demais e foi cortado. Diminua a duração do molde.".into());
        }
        _ => {}
    }
    let text = body["content"]
        .as_array()
        .and_then(|blocks| blocks.iter().find(|b| b["type"] == "text"))
        .and_then(|b| b["text"].as_str())
        .ok_or("A resposta do Claude veio sem texto.")?;
    serde_json::from_str(text).map_err(|e| format!("O Claude devolveu um JSON inválido: {e}"))
}

pub async fn gerar_roteiro(
    http: &reqwest::Client,
    api_key: &str,
    model: &str,
    molde: &Molde,
    input: &RoteiroInput,
) -> Result<Roteiro, String> {
    if api_key.is_empty() {
        return Err("Configure a chave da API do Claude em Configurações.".into());
    }
    let body = json!({
        "model": model,
        "max_tokens": 16000,
        "fallbacks": "default",
        "system": SYSTEM,
        "output_config": {
            "effort": "medium",
            "format": {"type": "json_schema", "schema": schema()}
        },
        "messages": [{"role": "user", "content": build_prompt(molde, input)}]
    });
    let resp = http
        .post(URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", API_VERSION)
        .header("anthropic-beta", FALLBACK_BETA)
        .json(&body)
        .timeout(std::time::Duration::from_secs(600))
        .send()
        .await
        .map_err(|e| format!("Falha de rede ao chamar o Claude: {e}"))?;
    let status = resp.status();
    let value: Value = resp
        .json()
        .await
        .map_err(|e| format!("Resposta inválida do Claude: {e}"))?;
    if !status.is_success() {
        let msg = value["error"]["message"].as_str().unwrap_or("erro desconhecido");
        return Err(format!("Claude API ({status}): {msg}"));
    }
    parse_response(&value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn molde() -> Molde {
        Molde {
            id: "m".into(),
            nome: "Mistérios BR".into(),
            nicho: "mistérios".into(),
            idioma: "pt".into(),
            tom: "".into(),
            publico: "".into(),
            estrutura: "".into(),
            duracao_minutos: 8,
            cta: "".into(),
            voz_profile_id: "".into(),
            criado_em: "".into(),
        }
    }

    #[test]
    fn prompt_includes_molde_and_reference() {
        let input = RoteiroInput {
            molde_id: "m".into(),
            ideia: Some(Outlier {
                video_id: "v".into(),
                titulo: "O segredo da pirâmide".into(),
                descricao: "".into(),
                canal_id: "c".into(),
                canal_nome: "Canal X".into(),
                thumbnail: "".into(),
                publicado_em: "".into(),
                views: 1000,
                likes: 0,
                comentarios: 0,
                duracao_segundos: 600,
                inscritos_canal: 0,
                mediana_canal: 100,
                score: 10.0,
            }),
            tema: "".into(),
        };
        let p = build_prompt(&molde(), &input);
        assert!(p.contains("Mistérios BR"));
        assert!(p.contains("1200 palavras"));
        assert!(p.contains("O segredo da pirâmide"));
        assert!(p.contains("(livre)"));
        assert!(!p.contains("instrucoes_extras"));
    }

    #[test]
    fn parses_text_block_and_refusal() {
        let roteiro = json!({
            "titulos": ["a"], "gancho": "g",
            "secoes": [{"titulo": "s", "narracao": "n", "visual": "v"}],
            "encerramento": "e", "descricao": "d", "tags": ["t"], "ideia_thumbnail": "i"
        });
        let body = json!({
            "stop_reason": "end_turn",
            "content": [
                {"type": "thinking", "thinking": ""},
                {"type": "text", "text": roteiro.to_string()}
            ]
        });
        let r = parse_response(&body).unwrap();
        assert_eq!(r.texto_narracao(), "g\n\nn\n\ne");

        let refused = json!({"stop_reason": "refusal", "content": []});
        assert!(parse_response(&refused).is_err());
    }
}
