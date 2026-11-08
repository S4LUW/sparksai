import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, errMsg, fmtDur, fmtNum, type Outlier, type SearchParams } from "../api";

interface Props {
  onCriarRoteiro: (ideia: Outlier) => void;
}

export default function Ideias({ onCriarRoteiro }: Props) {
  const [p, setP] = useState<SearchParams>({
    nicho: "",
    dias: 90,
    formato: "longos",
    idioma: "pt",
    regiao: "BR",
    max_resultados: 25,
  });
  const [res, setRes] = useState<Outlier[] | null>(null);
  const [salvas, setSalvas] = useState<Outlier[]>([]);
  const [carregando, setCarregando] = useState(false);
  const [erro, setErro] = useState("");
  const [ver, setVer] = useState<"busca" | "salvas">("busca");
  const [minScore, setMinScore] = useState(2);

  const carregarSalvas = () => api.listIdeias().then(setSalvas).catch(() => {});
  useEffect(() => {
    carregarSalvas();
  }, []);

  const buscar = async (e: React.FormEvent) => {
    e.preventDefault();
    setErro("");
    setCarregando(true);
    try {
      setRes(await api.huntOutliers(p));
      setVer("busca");
    } catch (e) {
      setErro(errMsg(e));
    } finally {
      setCarregando(false);
    }
  };

  const idsSalvos = new Set(salvas.map((s) => s.video_id));
  const lista = ver === "busca" ? (res ?? []).filter((o) => o.score >= minScore) : salvas;

  return (
    <section>
      <h1>Caçada de ideias</h1>
      <p className="muted">
        Digite um nicho e veja os vídeos que tiveram muito mais views que a média do próprio canal. O número em destaque
        é quantas vezes o vídeo superou a mediana dos uploads recentes do canal.
      </p>

      <form className="card search" onSubmit={buscar}>
        <input
          className="grow"
          placeholder="Ex.: mistérios da história, finanças pessoais, curiosidades do espaço"
          value={p.nicho}
          onChange={(e) => setP({ ...p, nicho: e.target.value })}
        />
        <select value={p.dias} onChange={(e) => setP({ ...p, dias: Number(e.target.value) })}>
          <option value={7}>Últimos 7 dias</option>
          <option value={30}>Últimos 30 dias</option>
          <option value={90}>Últimos 90 dias</option>
          <option value={365}>Último ano</option>
          <option value={0}>Qualquer data</option>
        </select>
        <select value={p.formato} onChange={(e) => setP({ ...p, formato: e.target.value as SearchParams["formato"] })}>
          <option value="longos">Vídeos longos</option>
          <option value="shorts">Shorts</option>
          <option value="todos">Todos</option>
        </select>
        <input className="tiny" title="Idioma (ex.: pt, en, es)" value={p.idioma} onChange={(e) => setP({ ...p, idioma: e.target.value })} />
        <input className="tiny" title="Região (ex.: BR, US)" value={p.regiao} onChange={(e) => setP({ ...p, regiao: e.target.value })} />
        <button className="primary" disabled={carregando || !p.nicho.trim()}>
          {carregando ? "Buscando..." : "Buscar"}
        </button>
      </form>

      {erro && <p className="error">{erro}</p>}

      <div className="row tabs">
        <button className={ver === "busca" ? "tab active" : "tab"} onClick={() => setVer("busca")}>
          Resultados {res ? `(${res.filter((o) => o.score >= minScore).length})` : ""}
        </button>
        <button className={ver === "salvas" ? "tab active" : "tab"} onClick={() => setVer("salvas")}>
          Salvas ({salvas.length})
        </button>
        {ver === "busca" && (
          <label className="inline">
            Score mínimo
            <select value={minScore} onChange={(e) => setMinScore(Number(e.target.value))}>
              {[0, 1, 2, 3, 5, 10].map((n) => (
                <option key={n} value={n}>
                  {n}x
                </option>
              ))}
            </select>
          </label>
        )}
      </div>

      {ver === "busca" && res && lista.length === 0 && (
        <p className="muted">Nenhum vídeo passou do score mínimo. Diminua o filtro ou tente outro nicho.</p>
      )}

      <div className="grid">
        {lista.map((o) => (
          <article key={o.video_id} className="card video">
            <a
              href={`https://www.youtube.com/watch?v=${o.video_id}`}
              onClick={(e) => {
                e.preventDefault();
                openUrl(`https://www.youtube.com/watch?v=${o.video_id}`);
              }}
            >
              <img src={o.thumbnail} alt="" />
              <span className={o.score >= 5 ? "score hot" : "score"}>{o.score ? `${o.score}x` : "?"}</span>
              <span className="dur">{fmtDur(o.duracao_segundos)}</span>
            </a>
            <h3 title={o.titulo}>{o.titulo}</h3>
            <p className="muted small">
              {o.canal_nome} · {fmtNum(o.inscritos_canal)} inscritos
            </p>
            <p className="small">
              <strong>{fmtNum(o.views)}</strong> views · mediana do canal {fmtNum(o.mediana_canal)} ·{" "}
              {new Date(o.publicado_em).toLocaleDateString("pt-BR")}
            </p>
            <div className="row">
              {idsSalvos.has(o.video_id) ? (
                <button
                  onClick={async () => {
                    await api.deleteIdeia(o.video_id);
                    carregarSalvas();
                  }}
                >
                  Remover
                </button>
              ) : (
                <button
                  onClick={async () => {
                    await api.saveIdeia(o, p.nicho);
                    carregarSalvas();
                  }}
                >
                  Salvar
                </button>
              )}
              <button className="primary" onClick={() => onCriarRoteiro(o)}>
                Criar roteiro
              </button>
            </div>
          </article>
        ))}
      </div>
    </section>
  );
}
