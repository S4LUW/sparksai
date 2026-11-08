import { useEffect, useState } from "react";
import { api, errMsg, type Molde, type PerfilVoz } from "../api";

const VAZIO: Molde = {
  id: "",
  nome: "",
  nicho: "",
  idioma: "pt",
  tom: "",
  publico: "",
  estrutura: "",
  duracao_minutos: 8,
  cta: "",
  voz_profile_id: "",
  criado_em: "",
};

export default function Moldes() {
  const [moldes, setMoldes] = useState<Molde[]>([]);
  const [edit, setEdit] = useState<Molde | null>(null);
  const [vozes, setVozes] = useState<PerfilVoz[]>([]);
  const [vozErro, setVozErro] = useState("");
  const [erro, setErro] = useState("");

  const carregar = () => api.listMoldes().then(setMoldes).catch((e) => setErro(errMsg(e)));

  useEffect(() => {
    carregar();
    api
      .voiceboxPerfis()
      .then(setVozes)
      .catch((e) => setVozErro(errMsg(e)));
  }, []);

  const salvar = async () => {
    if (!edit) return;
    setErro("");
    try {
      await api.saveMolde(edit);
      setEdit(null);
      carregar();
    } catch (e) {
      setErro(errMsg(e));
    }
  };

  const set =
    (k: keyof Molde) => (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>) =>
      edit && setEdit({ ...edit, [k]: k === "duracao_minutos" ? Number(e.target.value) : e.target.value });

  const nomeVoz = (id: string) => vozes.find((v) => v.id === id)?.name;

  return (
    <section>
      <div className="row between">
        <h1>Moldes</h1>
        {!edit && (
          <button className="primary" onClick={() => setEdit({ ...VAZIO })}>
            Novo molde
          </button>
        )}
      </div>
      <p className="muted">
        Um molde guarda o jeito de cada canal: nicho, idioma, tom, estrutura e a voz da narração. Todo roteiro segue um
        molde.
      </p>
      {erro && <p className="error">{erro}</p>}

      {edit && (
        <div className="card form">
          <div className="cols">
            <label>
              Nome do canal
              <input value={edit.nome} onChange={set("nome")} placeholder="Ex.: Arquivo Secreto" />
            </label>
            <label>
              Nicho
              <input value={edit.nicho} onChange={set("nicho")} placeholder="Ex.: mistérios históricos" />
            </label>
            <label>
              Idioma do vídeo
              <input value={edit.idioma} onChange={set("idioma")} placeholder="pt, en, es..." />
            </label>
            <label>
              Duração alvo (minutos)
              <input type="number" min={1} max={60} value={edit.duracao_minutos} onChange={set("duracao_minutos")} />
            </label>
          </div>
          <label>
            Tom de voz
            <input value={edit.tom} onChange={set("tom")} placeholder="Ex.: misterioso, calmo, com pausas dramáticas" />
          </label>
          <label>
            Público
            <input value={edit.publico} onChange={set("publico")} placeholder="Ex.: adultos de 25 a 45 anos curiosos por história" />
          </label>
          <label>
            Estrutura preferida
            <textarea
              rows={3}
              value={edit.estrutura}
              onChange={set("estrutura")}
              placeholder="Ex.: gancho com pergunta, contexto, 3 reviravoltas, revelação final"
            />
          </label>
          <label>
            Chamada para ação
            <input value={edit.cta} onChange={set("cta")} placeholder="Ex.: convidar para o próximo vídeo da série" />
          </label>
          <label>
            Voz (Voicebox)
            {vozes.length > 0 ? (
              <select value={edit.voz_profile_id} onChange={set("voz_profile_id")}>
                <option value="">Escolha uma voz</option>
                {vozes.map((v) => (
                  <option key={v.id} value={v.id}>
                    {v.name} {v.language ? `(${v.language})` : ""}
                  </option>
                ))}
              </select>
            ) : (
              <small className="error">
                {vozErro || "Nenhuma voz encontrada. Crie um perfil de voz no Voicebox e volte aqui."}
              </small>
            )}
          </label>
          <div className="row">
            <button className="primary" onClick={salvar}>
              Salvar molde
            </button>
            <button onClick={() => setEdit(null)}>Cancelar</button>
          </div>
        </div>
      )}

      {!edit && moldes.length === 0 && <p className="muted">Nenhum molde ainda. Crie o primeiro para gerar roteiros.</p>}

      <div className="grid">
        {!edit &&
          moldes.map((m) => (
            <article key={m.id} className="card">
              <h3>{m.nome}</h3>
              <p className="muted small">
                {m.nicho || "sem nicho"} · {m.idioma} · {m.duracao_minutos} min
              </p>
              {m.tom && <p className="small">Tom: {m.tom}</p>}
              <p className="small">Voz: {nomeVoz(m.voz_profile_id) ?? (m.voz_profile_id ? m.voz_profile_id : "nenhuma")}</p>
              <div className="row">
                <button onClick={() => setEdit(m)}>Editar</button>
                <button
                  className="danger"
                  onClick={async () => {
                    if (confirm(`Apagar o molde "${m.nome}"?`)) {
                      await api.deleteMolde(m.id);
                      carregar();
                    }
                  }}
                >
                  Apagar
                </button>
              </div>
            </article>
          ))}
      </div>
    </section>
  );
}
