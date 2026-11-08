import { useState } from "react";
import Ideias from "./pages/Ideias";
import Moldes from "./pages/Moldes";
import Roteiros from "./pages/Roteiros";
import Configuracoes from "./pages/Configuracoes";
import type { Outlier } from "./api";

type Aba = "ideias" | "moldes" | "roteiros" | "config";

const ABAS: { id: Aba; nome: string; desc: string }[] = [
  { id: "ideias", nome: "Ideias", desc: "Vídeos que bombaram no nicho" },
  { id: "moldes", nome: "Moldes", desc: "Perfil de cada canal" },
  { id: "roteiros", nome: "Roteiros", desc: "Roteiro e narração" },
  { id: "config", nome: "Configurações", desc: "Chaves e conexões" },
];

export default function App() {
  const [aba, setAba] = useState<Aba>("ideias");
  // Ideia escolhida em "Ideias" que abre direto em "Roteiros".
  const [ideiaParaRoteiro, setIdeiaParaRoteiro] = useState<Outlier | null>(null);

  return (
    <div className="layout">
      <aside className="sidebar">
        <div className="brand">
          <span className="spark">✦</span> Sparks AI
        </div>
        <nav>
          {ABAS.map((a) => (
            <button key={a.id} className={aba === a.id ? "nav active" : "nav"} onClick={() => setAba(a.id)}>
              <strong>{a.nome}</strong>
              <small>{a.desc}</small>
            </button>
          ))}
        </nav>
      </aside>
      <main className="content">
        {aba === "ideias" && (
          <Ideias
            onCriarRoteiro={(i) => {
              setIdeiaParaRoteiro(i);
              setAba("roteiros");
            }}
          />
        )}
        {aba === "moldes" && <Moldes />}
        {aba === "roteiros" && (
          <Roteiros ideiaInicial={ideiaParaRoteiro} onIdeiaUsada={() => setIdeiaParaRoteiro(null)} />
        )}
        {aba === "config" && <Configuracoes />}
      </main>
    </div>
  );
}
