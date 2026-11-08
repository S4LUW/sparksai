# Sparks AI

Aplicativo desktop (Tauri) para criar vídeos de YouTube com IA, rodando no seu computador:

1. **Ideias:** busca vídeos "outliers" de um nicho, ou seja, vídeos com muito mais views que a mediana do próprio canal.
2. **Moldes:** guarda o perfil de cada canal (nicho, idioma, tom, estrutura, duração e voz).
3. **Roteiros:** o Claude escreve um roteiro original a partir da ideia e do molde, com 5 títulos, gancho, seções, encerramento, descrição, tags e ideia de thumbnail. Tudo é editável.
4. **Narração:** o [Voicebox](https://github.com/jamiepine/voicebox) gera a voz localmente, sem custo por vídeo.

Edição de vídeo, thumbnail e upload para o YouTube ficam para as próximas etapas (veja o roadmap abaixo).

## O que você precisa

- [Node.js](https://nodejs.org/) 20 ou mais novo
- [Rust](https://rustup.rs/) estável
- Pré-requisitos do Tauri para o seu sistema: https://v2.tauri.app/start/prerequisites/
- [Voicebox](https://github.com/jamiepine/voicebox) instalado e aberto, com pelo menos um perfil de voz criado
- Uma chave da **YouTube Data API v3** (Google Cloud Console → APIs e serviços → ativar "YouTube Data API v3" → Credenciais → Chave de API)
- Uma chave da **API do Claude** (https://console.anthropic.com)

## Rodando

```bash
npm install
npm run tauri dev
```

Na primeira vez, o Rust compila todas as dependências e isso leva alguns minutos.

Depois abra **Configurações**, cole as duas chaves, clique em **Testar Voicebox** e salve.

Para gerar o instalador (`.msi`/`.exe` no Windows, `.dmg` no macOS, `.deb`/`.AppImage` no Linux):

```bash
npm run tauri build
```

## Como funciona por dentro

| Parte | Onde fica |
|---|---|
| Telas (React + TypeScript) | `src/` |
| Comandos chamados pelas telas | `src-tauri/src/lib.rs` |
| Banco local (SQLite) | `src-tauri/src/db.rs` |
| Busca de outliers (YouTube Data API) | `src-tauri/src/youtube.rs` |
| Roteiro (Claude, saída em JSON garantida) | `src-tauri/src/claude.rs` |
| Narração (Voicebox) | `src-tauri/src/voicebox.rs` |

As chamadas de rede ficam no Rust, então as chaves nunca passam pelo navegador interno. O banco `sparksai.db` e as narrações ficam na pasta de dados do app (no Windows, `%APPDATA%\com.sparksai.app`). As chaves ficam salvas nesse banco, sem criptografia, apenas neste computador.

### Score de outlier

Para cada vídeo encontrado, o app pega os 30 uploads mais recentes do canal e calcula a mediana de views, sem contar o próprio vídeo. O score é `views do vídeo ÷ mediana`. Um score de 10x quer dizer que o vídeo teve 10 vezes mais views que um vídeo típico do canal.

### Custos

- **YouTube Data API:** grátis, com cota de 10 mil unidades por dia. Uma busca de 25 vídeos usa cerca de 150 unidades, o que dá umas 60 buscas por dia.
- **Claude:** pago por uso. O modelo padrão é o `claude-opus-5-5`; em Configurações dá para trocar por `claude-sonnet-5-5`, que custa menos.
- **Voicebox:** grátis, roda na sua máquina. Sem GPU a narração fica bem mais lenta.

## Testes

```bash
cd src-tauri
cargo test
```

Para testar a narração com o Voicebox aberto:

```bash
VOICEBOX_TEST_URL=http://127.0.0.1:17493 cargo test -- --ignored voicebox_real
```

## Roadmap

- [x] Busca de outliers por nicho
- [x] Moldes de canal
- [x] Roteiro com Claude
- [x] Narração local com Voicebox
- [ ] Legendas sincronizadas (Whisper)
- [ ] Imagens por seção (IA ou banco de imagens)
- [ ] Edição automática com estilos (Remotion + ffmpeg)
- [ ] Thumbnail
- [ ] Upload e agendamento no YouTube (OAuth + `videos.insert`)
- [ ] Painel com views, inscritos e receita (YouTube Analytics API)

> Atenção: o YouTube corta a monetização de conteúdo "inautêntico ou produzido em massa". Use o app para acelerar a produção, mas revise e personalize cada vídeo.
