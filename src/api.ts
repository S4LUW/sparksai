// Ponte tipada entre o React e os comandos Rust (src-tauri/src/lib.rs).
import { invoke, convertFileSrc } from "@tauri-apps/api/core";

export interface Settings {
  youtube_api_key: string;
  anthropic_api_key: string;
  claude_model: string;
  voicebox_url: string;
}

export interface SearchParams {
  nicho: string;
  dias: number;
  formato: "todos" | "longos" | "shorts";
  idioma: string;
  regiao: string;
  max_resultados: number;
}

export interface Outlier {
  video_id: string;
  titulo: string;
  descricao: string;
  canal_id: string;
  canal_nome: string;
  thumbnail: string;
  publicado_em: string;
  views: number;
  likes: number;
  comentarios: number;
  duracao_segundos: number;
  inscritos_canal: number;
  mediana_canal: number;
  score: number;
}

export interface Molde {
  id: string;
  nome: string;
  nicho: string;
  idioma: string;
  tom: string;
  publico: string;
  estrutura: string;
  duracao_minutos: number;
  cta: string;
  voz_profile_id: string;
  criado_em: string;
}

export interface Secao {
  titulo: string;
  narracao: string;
  visual: string;
}

export interface Roteiro {
  titulos: string[];
  gancho: string;
  secoes: Secao[];
  encerramento: string;
  descricao: string;
  tags: string[];
  ideia_thumbnail: string;
}

export interface RoteiroRow {
  id: string;
  molde_id: string;
  video_id: string | null;
  titulo: string;
  dados: Roteiro;
  audio_path: string | null;
  criado_em: string;
}

export interface PerfilVoz {
  id: string;
  name: string;
  language: string;
  description?: string | null;
}

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),

  huntOutliers: (params: SearchParams) => invoke<Outlier[]>("hunt_outliers", { params }),
  saveIdeia: (ideia: Outlier, nicho: string) => invoke<void>("save_ideia", { ideia, nicho }),
  listIdeias: () => invoke<Outlier[]>("list_ideias"),
  deleteIdeia: (videoId: string) => invoke<void>("delete_ideia", { videoId }),

  listMoldes: () => invoke<Molde[]>("list_moldes"),
  saveMolde: (molde: Molde) => invoke<Molde>("save_molde", { molde }),
  deleteMolde: (id: string) => invoke<void>("delete_molde", { id }),

  gerarRoteiro: (input: { molde_id: string; ideia: Outlier | null; tema: string }) =>
    invoke<RoteiroRow>("gerar_roteiro", { input }),
  listRoteiros: () => invoke<RoteiroRow[]>("list_roteiros"),
  saveRoteiro: (id: string, titulo: string, dados: Roteiro) =>
    invoke<void>("save_roteiro", { id, titulo, dados }),
  deleteRoteiro: (id: string) => invoke<void>("delete_roteiro", { id }),

  voiceboxPerfis: () => invoke<PerfilVoz[]>("voicebox_perfis"),
  narrarRoteiro: (id: string) => invoke<RoteiroRow>("narrar_roteiro", { id }),
};

export const audioSrc = (path: string) => convertFileSrc(path);

export const fmtNum = (n: number) =>
  new Intl.NumberFormat("pt-BR", { notation: "compact", maximumFractionDigits: 1 }).format(n);

export const fmtDur = (s: number) => {
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  const mm = h ? String(m).padStart(2, "0") : String(m);
  return `${h ? h + ":" : ""}${mm}:${String(sec).padStart(2, "0")}`;
};

export const errMsg = (e: unknown) => (typeof e === "string" ? e : e instanceof Error ? e.message : String(e));
