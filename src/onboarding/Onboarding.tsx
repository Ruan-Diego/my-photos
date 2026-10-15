import { motion } from "motion/react";
import { Logo } from "../shell/Sidebar";
import { folderDetail, type LibraryFolder } from "./flow";

const PROMISES = [
  {
    icon: "M12 3l8 3v6c0 4.5-3.4 8-8 9-4.6-1-8-4.5-8-9V6zM9 12l2 2 4-4",
    title: "Ler e indexar nunca altera seus arquivos.",
    body: " Varredura, metadados e miniaturas só leem o disco.",
  },
  {
    icon: "M4 7h16M6 7l1 13h10l1-13M9 7V4h6v3",
    title: "Mover, renomear e excluir só quando você pedir.",
    body: " Excluir vai para a lixeira do app e fica lá por 30 dias.",
  },
];

const DB_ICON =
  "M5 6a7 3 0 1 0 14 0a7 3 0 1 0 -14 0M5 6v12c0 1.7 3.1 3 7 3s7-1.3 7-3V6M5 12c0 1.7 3.1 3 7 3s7-1.3 7-3";

function PromiseIcon({ d }: { d: string }) {
  return (
    <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" className="welcome-promise-icon">
      <path d={d} />
    </svg>
  );
}

// Quatro "fotos" inclinadas no painel escuro (artboard `Main`): entram uma a uma, sem loop.
const STACK = [
  { className: "stack-1", rotate: -11 },
  { className: "stack-2", rotate: 7 },
  { className: "stack-3", rotate: -2 },
  { className: "stack-4", rotate: 12 },
];

export function Onboarding({
  folder,
  starting,
  failure,
  onStart,
}: {
  folder: LibraryFolder | null;
  starting: boolean;
  failure: string | null;
  onStart: () => void;
}) {
  const root = folder?.rootPath ?? "E:\\";
  return (
    <main className="welcome">
      <section className="welcome-text">
        <div className="welcome-brand">
          <Logo size={28} />
          My Photos
        </div>

        <div className="welcome-lead">
          <h1>Seu acervo do Google Fotos, de volta no seu disco.</h1>
          <p>
            Aponte a pasta do Takeout. O My Photos lê datas, locais e pessoas dos JSONs e monta a sua linha do
            tempo, tudo offline: nada sai deste computador.
          </p>
        </div>

        <div className="welcome-folder">
          <span className="welcome-label">Pasta da biblioteca</span>
          <div className="welcome-folder-card">
            <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
              <rect x="3" y="6" width="18" height="12" rx="2" />
              <path d="M7 14h.01M11 14h6" />
            </svg>
            <div className="welcome-folder-text">
              <strong>{root}</strong>
              <span>{folder === null ? "procurando o disco…" : folderDetail(folder)}</span>
            </div>
            {/* Trocar a pasta muda a regra do F11 (AD-007): fica para uma feature própria. */}
            <button type="button" className="welcome-change" disabled title="Em breve">
              Trocar pasta
            </button>
          </div>
        </div>

        <ul className="welcome-promises">
          {PROMISES.map((promise) => (
            <li key={promise.title}>
              <PromiseIcon d={promise.icon} />
              <span>
                <strong>{promise.title}</strong>
                {promise.body}
              </span>
            </li>
          ))}
          <li>
            <PromiseIcon d={DB_ICON} />
            <span>
              <strong>O índice viaja com o HD.</strong> Fica em <code>{root}.myphotos\library.db</code>, numa pasta
              oculta.
            </span>
          </li>
        </ul>

        <div className="welcome-actions">
          <button type="button" className="welcome-start" onClick={onStart} disabled={starting || folder === null}>
            {starting ? "Começando…" : `Indexar ${root}`}
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
              <path d="M5 12h14M13 6l6 6-6 6" />
            </svg>
          </button>
          <span className="welcome-hint">Leva alguns minutos. Dá para navegar enquanto indexa.</span>
        </div>
        {failure !== null ? (
          <p className="welcome-failure" role="alert">
            {failure}
          </p>
        ) : null}
      </section>

      <section className="welcome-art" aria-hidden="true">
        <div className="welcome-stack">
          {STACK.map((card, index) => (
            <motion.div
              key={card.className}
              className={`welcome-photo ${card.className}`}
              initial={{ opacity: 0, y: 24, rotate: card.rotate - 4 }}
              animate={{ opacity: 1, y: 0, rotate: card.rotate }}
              transition={{ delay: 0.12 * index, type: "spring", stiffness: 140, damping: 18 }}
            />
          ))}
        </div>
      </section>
    </main>
  );
}
