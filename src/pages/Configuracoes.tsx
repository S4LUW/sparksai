import { useEffect, useState } from "react";
import { api, errMsg, type PerfilVoz, type Settings } from "../api";

export default function Configuracoes() {
  const [s, setS] = useState<Settings | null>(null);
  const [msg, setMsg] = useState("");
  const [vozes, setVozes] = useState<PerfilVoz[] | null>(null);
  const [vbErro, setVbErro] = useState("");

  useEffect(() => {
    api.getSettings().then(setS).catch((e) => setMsg(errMsg(e)));
  }, []);

  if (!s) return <p className="muted">{msg || "Carregando..."}</p>;

  const set = (k: keyof Settings) => (e: React.ChangeEvent<HTMLInputElement>) => setS({ ...s, [k]: e.target.value });

  const salvar = async () => {
    try {
      setS(await api.saveSettings(s));
      setMsg("Configurações salvas.");
    } catch (e) {
      setMsg(errMsg(e));
    }
  };

  const testarVoicebox = async () => {
    setVbErro("");
    setVozes(null);
    try {
      await api.saveSettings(s);
      setVozes(await api.voiceboxPerfis());
    } catch (e) {
      setVbErro(errMsg(e));
    }
  };

  return (
    <section>
      <h1>Configurações</h1>
      <p className="muted">
        Tudo fica salvo só neste computador, no banco local do app. As chaves são suas e os custos das APIs vão direto
        para a sua conta.
      </p>

      <div className="card form">
        <label>
          Chave da YouTube Data API v3
          <input type="password" value={s.youtube_api_key} onChange={set("youtube_api_key")} placeholder="AIza..." />
          <small>
            Crie no Google Cloud Console, em APIs e serviços, ativando a "YouTube Data API v3". A cota grátis é de 10
            mil unidades por dia, e cada busca usa cerca de 150.
          </small>
        </label>

        <label>
          Chave da API do Claude (Anthropic)
          <input type="password" value={s.anthropic_api_key} onChange={set("anthropic_api_key")} placeholder="sk-ant-..." />
          <small>Crie em console.anthropic.com. É usada para gerar roteiros, títulos, descrição e tags.</small>
        </label>

        <label>
          Modelo do Claude
          <input value={s.claude_model} onChange={set("claude_model")} />
          <small>Padrão: claude-opus-5-5. Para gastar menos por roteiro, use claude-sonnet-5-5.</small>
        </label>

        <label>
          Endereço do Voicebox
          <input value={s.voicebox_url} onChange={set("voicebox_url")} />
          <small>O Voicebox precisa estar aberto neste computador. O endereço padrão é http://127.0.0.1:17493.</small>
        </label>

        <div className="row">
          <button className="primary" onClick={salvar}>
            Salvar
          </button>
          <button onClick={testarVoicebox}>Testar Voicebox</button>
          {msg && <span className="muted">{msg}</span>}
        </div>

        {vbErro && <p className="error">{vbErro}</p>}
        {vozes && (
          <p className="ok">
            Voicebox conectado. {vozes.length} voz(es) encontrada(s)
            {vozes.length > 0 && `: ${vozes.map((v) => v.name).join(", ")}`}.
          </p>
        )}
      </div>
    </section>
  );
}
