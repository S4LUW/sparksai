import { useEffect, useState } from "react";
import { api, audioSrc, errMsg, type Molde, type Outlier, type Roteiro, type RoteiroRow } from "../api";

interface Props {
  ideiaInicial: Outlier | null;
  onIdeiaUsada: () => void;
}

export default function Roteiros({ ideiaInicial, onIdeiaUsada }: Props) {
  const [moldes, setMoldes] = useState<Molde[]>([]);
  const [ideias, setIdeias] = useState<Outlier[]>([]);
  const [roteiros, setRoteiros] = useState<RoteiroRow[]>([]);
  const [moldeId, setMoldeId] = useState("");
  const [ideia, setIdeia] = useState<Outlier | null>(ideiaInicial);
  const [tema, setTema] = useState("");
  const [gerando, setGerando] = useState(false);
  const [erro, setErro] = useState("");
  const [aberto, setAberto] = useState<RoteiroRow | null>(null);

  const carregar = () => api.listRoteiros().then(setRoteiros).catch((e) => setErro(errMsg(e)));

  useEffect(() => {
    api.listMoldes().then((ms) => {
      setMoldes(ms);
      if (ms[0]) setMoldeId((id) => id || ms[0].id);
    });
    api.listIdeias().then(setIdeias);
    carregar();
  }, []);

  useEffect(() => {
    if (ideiaInicial) {
      setIdeia(ideiaInicial);
      onIdeiaUsada();
    }
  }, [ideiaInicial, onIdeiaUsada]);

  // A ideia vinda da busca pode não estar salva; ela entra na lista do seletor mesmo assim.
  const opcoes = ideia && !ideias.some((i) => i.video_id === ideia.video_id) ? [ideia, ...ideias] : ideias;

  const gerar = async () => {
    setErro("");
    setGerando(true);
    try {
      const r = await api.gerarRoteiro({ molde_id: moldeId, ideia, tema });
      await carregar();
      setAberto(r);
    } catch (e) {
      setErro(errMsg(e));
    } finally {
      setGerando(false);
    }
  };

  if (aberto) {
    return (
      <Editor
        row={aberto}
        molde={moldes.find((m) => m.id === aberto.molde_id)}
        onVoltar={() => {
          setAberto(null);
          carregar();
        }}
      />
    );
  }

  return (
    <section>
      <h1>Roteiros</h1>
      <p className="muted">
        Escolha um molde e uma ideia (ou escreva um tema). O Claude escreve um roteiro original inspirado no ângulo que
        fez o vídeo funcionar, com títulos, descrição e tags.
      </p>

      {moldes.length === 0 ? (
        <p className="error">Crie um molde primeiro, na aba Moldes.</p>
      ) : (
        <div className="card form">
          <div className="cols">
            <label>
              Molde
              <select value={moldeId} onChange={(e) => setMoldeId(e.target.value)}>
                {moldes.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.nome}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Ideia de referência
              <select
                value={ideia?.video_id ?? ""}
                onChange={(e) => setIdeia(opcoes.find((i) => i.video_id === e.target.value) ?? null)}
              >
                <option value="">Nenhuma (usar só o tema)</option>
                {opcoes.map((i) => (
                  <option key={i.video_id} value={i.video_id}>
                    {i.score}x · {i.titulo}
                  </option>
                ))}
              </select>
            </label>
          </div>
          <label>
            Tema ou instruções extras
            <textarea
              rows={3}
              value={tema}
              onChange={(e) => setTema(e.target.value)}
              placeholder="Ex.: focar no caso brasileiro, citar 3 exemplos, terminar com uma pergunta para os comentários"
            />
          </label>
          <div className="row">
            <button className="primary" onClick={gerar} disabled={gerando || !moldeId || (!ideia && !tema.trim())}>
              {gerando ? "Escrevendo o roteiro..." : "Gerar roteiro"}
            </button>
            {gerando && <span className="muted">Isso costuma levar de 30 segundos a 2 minutos.</span>}
          </div>
          {erro && <p className="error">{erro}</p>}
        </div>
      )}

      <h2>Roteiros salvos</h2>
      {roteiros.length === 0 && <p className="muted">Nenhum roteiro ainda.</p>}
      <div className="list">
        {roteiros.map((r) => (
          <button key={r.id} className="card item" onClick={() => setAberto(r)}>
            <strong>{r.titulo}</strong>
            <span className="muted small">
              {moldes.find((m) => m.id === r.molde_id)?.nome ?? "molde apagado"} ·{" "}
              {new Date(r.criado_em).toLocaleString("pt-BR")} {r.audio_path ? "· 🔊 narrado" : ""}
            </span>
          </button>
        ))}
      </div>
    </section>
  );
}

function Editor({ row, molde, onVoltar }: { row: RoteiroRow; molde?: Molde; onVoltar: () => void }) {
  const [r, setR] = useState<Roteiro>(row.dados);
  const [titulo, setTitulo] = useState(row.titulo);
  const [audio, setAudio] = useState(row.audio_path);
  // Muda a cada nova narração para o player não tocar a versão antiga do cache.
  const [versao, setVersao] = useState(0);
  const [msg, setMsg] = useState("");
  const [erro, setErro] = useState("");
  const [narrando, setNarrando] = useState(false);

  const salvar = async () => {
    setErro("");
    try {
      await api.saveRoteiro(row.id, titulo, r);
      setMsg("Roteiro salvo.");
      return true;
    } catch (e) {
      setErro(errMsg(e));
      return false;
    }
  };

  const narrar = async () => {
    if (!(await salvar())) return;
    setNarrando(true);
    setMsg("");
    try {
      const atualizado = await api.narrarRoteiro(row.id);
      setAudio(atualizado.audio_path);
      setVersao(Date.now());
      setMsg("Narração pronta.");
    } catch (e) {
      setErro(errMsg(e));
    } finally {
      setNarrando(false);
    }
  };

  const setSecao = (i: number, k: "titulo" | "narracao" | "visual", v: string) =>
    setR({ ...r, secoes: r.secoes.map((s, j) => (j === i ? { ...s, [k]: v } : s)) });

  const copiar = (t: string) => navigator.clipboard.writeText(t).then(() => setMsg("Copiado."));
  const palavras = [r.gancho, ...r.secoes.map((s) => s.narracao), r.encerramento].join(" ").split(/\s+/).filter(Boolean).length;

  return (
    <section>
      <div className="row between">
        <button onClick={onVoltar}>← Voltar</button>
        <div className="row">
          <button onClick={salvar}>Salvar</button>
          <button className="primary" onClick={narrar} disabled={narrando || !molde?.voz_profile_id}>
            {narrando ? "Narrando no Voicebox..." : audio ? "Narrar de novo" : "Narrar com Voicebox"}
          </button>
        </div>
      </div>
      {!molde?.voz_profile_id && <p className="error">Escolha uma voz no molde para poder narrar.</p>}
      {msg && <p className="ok">{msg}</p>}
      {erro && <p className="error">{erro}</p>}
      {audio && (
        <div className="card">
          <audio key={versao} controls src={`${audioSrc(audio)}?v=${versao}`} />
          <p className="muted small">Arquivo: {audio}</p>
        </div>
      )}

      <p className="muted small">
        {molde?.nome ?? "Molde apagado"} · {palavras} palavras · cerca de {Math.max(1, Math.round(palavras / 150))} min de
        narração
      </p>

      <div className="card form">
        <label>
          Título escolhido
          <input value={titulo} onChange={(e) => setTitulo(e.target.value)} />
        </label>
        <div className="chips">
          {r.titulos.map((t) => (
            <button key={t} className={t === titulo ? "chip active" : "chip"} onClick={() => setTitulo(t)}>
              {t}
            </button>
          ))}
        </div>
      </div>

      <div className="card form">
        <label>
          Gancho
          <textarea rows={3} value={r.gancho} onChange={(e) => setR({ ...r, gancho: e.target.value })} />
        </label>
        {r.secoes.map((s, i) => (
          <div key={i} className="secao">
            <input className="secao-titulo" value={s.titulo} onChange={(e) => setSecao(i, "titulo", e.target.value)} />
            <textarea rows={5} value={s.narracao} onChange={(e) => setSecao(i, "narracao", e.target.value)} />
            <label className="small">
              Visual
              <textarea rows={2} value={s.visual} onChange={(e) => setSecao(i, "visual", e.target.value)} />
            </label>
          </div>
        ))}
        <label>
          Encerramento
          <textarea rows={3} value={r.encerramento} onChange={(e) => setR({ ...r, encerramento: e.target.value })} />
        </label>
      </div>

      <div className="card form">
        <label>
          <span className="row between">
            Descrição <button onClick={() => copiar(r.descricao)}>Copiar</button>
          </span>
          <textarea rows={5} value={r.descricao} onChange={(e) => setR({ ...r, descricao: e.target.value })} />
        </label>
        <label>
          <span className="row between">
            Tags (separadas por vírgula) <button onClick={() => copiar(r.tags.join(", "))}>Copiar</button>
          </span>
          <input
            value={r.tags.join(", ")}
            onChange={(e) => setR({ ...r, tags: e.target.value.split(",").map((t) => t.trim()).filter(Boolean) })}
          />
        </label>
        <label>
          Ideia de thumbnail
          <textarea rows={2} value={r.ideia_thumbnail} onChange={(e) => setR({ ...r, ideia_thumbnail: e.target.value })} />
        </label>
      </div>

      <div className="row">
        <button
          className="danger"
          onClick={async () => {
            if (confirm("Apagar este roteiro e a narração?")) {
              await api.deleteRoteiro(row.id);
              onVoltar();
            }
          }}
        >
          Apagar roteiro
        </button>
      </div>
    </section>
  );
}
