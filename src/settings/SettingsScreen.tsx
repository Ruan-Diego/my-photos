import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { THEME_OPTIONS, type ThemeChoice } from "../theme/theme";
import { fileSize } from "../viewer/navigation";
import {
  cacheOptions,
  cacheUsage,
  databaseLine,
  MOTION_OPTIONS,
  SCAN_ON_CONNECT,
  SECTIONS,
  VIDEO_COVERS,
  videoCoversNote,
  volumeLine,
  type MotionChoice,
  type Settings,
} from "./settings";

interface Props {
  theme: ThemeChoice;
  onTheme: (choice: ThemeChoice) => void;
  motion: MotionChoice;
  onMotion: (choice: MotionChoice) => void;
  /** "Procurar arquivos novos"; `undefined` sem disco ou durante uma indexação. */
  onScan: (() => void) | undefined;
  /** Uma indexação está rodando: as miniaturas não podem ser apagadas agora. */
  indexing: boolean;
}

/** Configurações (F31, artboard `Settings`). */
export function SettingsScreen({ theme, onTheme, motion, onMotion, onScan, indexing }: Props) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  // F33: o botão clicado some ao confirmar; o foco vai para "Cancelar" e volta depois.
  const cancelClear = useRef<HTMLButtonElement>(null);
  const askClear = useRef<HTMLButtonElement>(null);
  const confirmed = useRef(false);
  useEffect(() => {
    if (confirming) {
      cancelClear.current?.focus();
    } else if (confirmed.current) {
      askClear.current?.focus();
    }
    confirmed.current = confirming;
  }, [confirming]);
  const [clearing, setClearing] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [current, setCurrent] = useState<string>(SECTIONS[0].id);

  const reload = useCallback(() => {
    invoke<Settings>("settings_overview")
      .then((value) => {
        setSettings(value);
        setFailure(null);
      })
      .catch((error) => setFailure(String(error)));
  }, []);

  useEffect(reload, [reload]);

  const setFlag = (key: string, on: boolean) => {
    setSettings((current) =>
      current === null ? current : { ...current, ...(key === VIDEO_COVERS ? { videoCovers: on } : { scanOnConnect: on }) },
    );
    invoke("settings_set_flag", { key, on }).catch((error) => {
      setNotice(`Não foi possível salvar: ${String(error)}`);
      reload();
    });
  };

  const setLimit = (bytes: number) => {
    setSettings((current) => (current === null ? current : { ...current, thumbnailLimitBytes: bytes }));
    invoke("settings_set_cache_limit", { bytes })
      .then(() => setNotice(null))
      .catch((error) => {
        setNotice(`Não foi possível salvar: ${String(error)}`);
        reload();
      });
  };

  const clear = () => {
    setClearing(true);
    invoke<number>("thumbnails_clear")
      .then((bytes) => setNotice(`${fileSize(bytes)} de miniaturas apagados. "Procurar arquivos novos" refaz as miniaturas.`))
      .catch((error) => setNotice(`As miniaturas não foram apagadas: ${String(error)}`))
      .finally(() => {
        setClearing(false);
        setConfirming(false);
        reload();
      });
  };

  const go = (id: string) => {
    setCurrent(id);
    document.getElementById(`settings-${id}`)?.scrollIntoView({ block: "start" });
  };

  const usage = settings === null ? null : cacheUsage(settings);
  const coversNote = settings === null ? null : videoCoversNote(settings);

  return (
    <main className="page" aria-labelledby="page-title">
      <header className="page-header">
        <h1 id="page-title">Configurações</h1>
      </header>
      <div className="page-body settings-body">
        <nav className="settings-nav" aria-label="Seções das configurações">
          {SECTIONS.map((section) => (
            <button key={section.id} type="button" aria-current={current === section.id} onClick={() => go(section.id)}>
              {section.label}
            </button>
          ))}
        </nav>

        <div className="settings">
          {failure !== null ? (
            <p className="settings-notice" role="alert">
              Não foi possível ler as configurações da biblioteca: {failure}
            </p>
          ) : null}
          {notice !== null ? (
            <p className="settings-notice" role="status">
              {notice}
            </p>
          ) : null}

          <section id="settings-biblioteca" className="card" aria-labelledby="settings-biblioteca-title">
            <h2 id="settings-biblioteca-title">Biblioteca</h2>
            <div className="settings-row">
              <div className="settings-what">
                <strong>{settings?.rootPath ?? "Disco não conectado"}</strong>
                <span>{settings === null ? " " : volumeLine(settings)}</span>
              </div>
              <button type="button" className="button-link" disabled title="Uma biblioteca por disco por enquanto (AD-011)">
                Trocar pasta
              </button>
            </div>
            <div className="settings-actions">
              <button type="button" className="button-solid" disabled={onScan === undefined} onClick={onScan}>
                Procurar arquivos novos
              </button>
              <button type="button" className="button-line" disabled title="Chega numa próxima versão; a busca incremental já relê o que mudou">
                Reindexar tudo
              </button>
            </div>
          </section>

          <section id="settings-dados" className="card" aria-labelledby="settings-dados-title">
            <h2 id="settings-dados-title">Dados e cache</h2>
            <dl className="settings-paths">
              <dt>Banco</dt>
              <dd>{settings === null ? " " : databaseLine(settings)}</dd>
              <dt>Lixeira</dt>
              <dd className="settings-later">Chega com as operações de arquivo</dd>
            </dl>
            <div className="settings-cache">
              <div className="settings-cache-head">
                <label htmlFor="cache-limit">Limite do cache de miniaturas</label>
                <span>{usage?.text ?? " "}</span>
              </div>
              <div className="settings-meter" aria-hidden="true">
                <div style={{ width: `${(usage?.fraction ?? 0) * 100}%` }} />
              </div>
              <div className="settings-actions">
                <select
                  id="cache-limit"
                  value={settings?.thumbnailLimitBytes ?? ""}
                  disabled={settings === null}
                  onChange={(event) => setLimit(Number(event.target.value))}
                >
                  {settings === null
                    ? null
                    : cacheOptions(settings).map((option) => (
                        <option key={option.bytes} value={option.bytes}>
                          {option.label}
                        </option>
                      ))}
                </select>
                {confirming ? (
                  <>
                    <button type="button" className="button-danger" disabled={clearing} onClick={clear}>
                      {clearing ? "Apagando…" : `Apagar ${fileSize(settings?.thumbnailBytes ?? 0)} de miniaturas`}
                    </button>
                    <button ref={cancelClear} type="button" className="button-line" disabled={clearing} onClick={() => setConfirming(false)}>
                      Cancelar
                    </button>
                  </>
                ) : (
                  <button
                    ref={askClear}
                    type="button"
                    className="button-line"
                    disabled={settings === null || indexing || settings.thumbnailBytes === 0}
                    title={indexing ? "Espere a indexação terminar" : undefined}
                    onClick={() => setConfirming(true)}
                  >
                    Limpar miniaturas
                  </button>
                )}
              </div>
              {confirming ? (
                <p className="settings-hint">As miniaturas somem até a próxima "Procurar arquivos novos", que refaz todas (no seu HD levou cerca de 40 minutos).</p>
              ) : null}
            </div>
          </section>

          <section id="settings-indexacao" className="card" aria-labelledby="settings-indexacao-title">
            <h2 id="settings-indexacao-title">Indexação</h2>
            <div className="settings-switches">
              <label className="settings-switch">
                <input
                  type="checkbox"
                  role="switch"
                  checked={settings?.videoCovers ?? true}
                  disabled={settings === null}
                  onChange={(event) => setFlag(VIDEO_COVERS, event.target.checked)}
                />
                <span>Gerar capa dos vídeos</span>
              </label>
              {coversNote !== null ? <p className="settings-hint">{coversNote}</p> : null}
              <label className="settings-switch">
                <input
                  type="checkbox"
                  role="switch"
                  checked={settings?.scanOnConnect ?? false}
                  disabled={settings === null}
                  onChange={(event) => setFlag(SCAN_ON_CONNECT, event.target.checked)}
                />
                <span>Procurar arquivos novos ao conectar o HD</span>
              </label>
            </div>
            <div className="settings-ignored">
              <span>Pastas sempre ignoradas</span>
              <div>
                {(settings?.ignoredFolders ?? []).map((name) => (
                  <code key={name}>{name}</code>
                ))}
              </div>
            </div>
          </section>

          <section id="settings-aparencia" className="card" aria-labelledby="settings-aparencia-title">
            <h2 id="settings-aparencia-title">Aparência</h2>
            <fieldset className="choice-row">
              <legend>Tema</legend>
              {THEME_OPTIONS.map((option) => (
                <label key={option.choice} className="choice" data-on={option.choice === theme ? "" : undefined}>
                  <input type="radio" name="theme" checked={option.choice === theme} onChange={() => onTheme(option.choice)} />
                  {option.label}
                </label>
              ))}
            </fieldset>
            <div className="settings-selects">
              <label>
                Animações
                <select value={motion} onChange={(event) => onMotion(event.target.value as MotionChoice)}>
                  {MOTION_OPTIONS.map((option) => (
                    <option key={option.choice} value={option.choice}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                Idioma
                <select value="pt-BR" disabled>
                  <option value="pt-BR">Português (Brasil)</option>
                </select>
              </label>
            </div>
          </section>

          <section id="settings-privacidade" className="card settings-privacy" aria-labelledby="settings-privacidade-title">
            <span className="settings-privacy-icon" aria-hidden="true">
              <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
                <path d="M12 3l8 3v6c0 4.5-3.4 8-8 9-4.6-1-8-4.5-8-9V6z" />
                <path d="M9 12l2 2 4-4" />
              </svg>
            </span>
            <div>
              <h2 id="settings-privacidade-title">Privacidade</h2>
              <span>O My Photos não faz nenhuma conexão de rede. Fotos, índice e miniaturas nunca saem deste computador.</span>
            </div>
          </section>
        </div>
      </div>
    </main>
  );
}
