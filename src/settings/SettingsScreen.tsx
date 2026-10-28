import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { THEME_OPTIONS, type ThemeChoice } from "../theme/theme";
import { fileSize } from "../viewer/navigation";
import { currentLocale, LOCALES, t } from "../i18n/i18n";
import { exportOptions, importText, lastBackupLine, type BackupOverview, type ImportOutcome } from "./backup";
import { UpdatesBlock } from "./UpdatesBlock";
import {
  privacyNote,
  UPDATE_CHECKS,
  cacheOptions,
  cacheUsage,
  databaseLine,
  trashLine,
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

  // F35: cópias do banco e exportação de favoritos e notas.
  const [backups, setBackups] = useState<BackupOverview | null>(null);
  const [chosenExport, setChosenExport] = useState("");
  const [backingUp, setBackingUp] = useState(false);
  const reloadBackups = useCallback(() => {
    invoke<BackupOverview>("backup_overview")
      .then((value) => {
        const names = exportOptions(value).map((option) => option.name);
        setBackups(value);
        setChosenExport((current) => (names.includes(current) ? current : (names[0] ?? "")));
      })
      .catch(() => setBackups(null));
  }, []);
  useEffect(reloadBackups, [reloadBackups]);
  const backupAction = (work: Promise<string>) => {
    setBackingUp(true);
    work
      .then((text) => setNotice(text))
      .catch((error) => setNotice(t("settings.actionFailed", { error: String(error) })))
      .finally(() => {
        setBackingUp(false);
        reloadBackups();
      });
  };

  const setFlag = (key: string, on: boolean) => {
    const field = ({ [VIDEO_COVERS]: "videoCovers", [SCAN_ON_CONNECT]: "scanOnConnect", [UPDATE_CHECKS]: "updateChecks" } as const)[key];
    if (field !== undefined) {
      setSettings((current) => (current === null ? current : { ...current, [field]: on }));
    }
    invoke("settings_set_flag", { key, on }).catch((error) => {
      setNotice(t("settings.saveFailed", { error: String(error) }));
      reload();
    });
  };

  const setLimit = (bytes: number) => {
    setSettings((current) => (current === null ? current : { ...current, thumbnailLimitBytes: bytes }));
    invoke("settings_set_cache_limit", { bytes })
      .then(() => setNotice(null))
      .catch((error) => {
        setNotice(t("settings.saveFailed", { error: String(error) }));
        reload();
      });
  };

  const clear = () => {
    setClearing(true);
    invoke<number>("thumbnails_clear")
      .then((bytes) => setNotice(t("settings.thumbnailsCleared", { size: fileSize(bytes), action: t("disk.scanNew") })))
      .catch((error) => setNotice(t("settings.clearFailed", { error: String(error) })))
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
        <h1 id="page-title">{t("section.settings")}</h1>
      </header>
      <div className="page-body settings-body">
        <nav className="settings-nav" aria-label={t("settings.sectionsLabel")}>
          {SECTIONS.map((section) => (
            <button key={section.id} type="button" aria-current={current === section.id} onClick={() => go(section.id)}>
              {section.label}
            </button>
          ))}
        </nav>

        <div className="settings">
          {failure !== null ? (
            <p className="settings-notice" role="alert">
              {t("settings.readFailed", { error: failure })}
            </p>
          ) : null}
          {notice !== null ? (
            <p className="settings-notice" role="status">
              {notice}
            </p>
          ) : null}

          <section id="settings-biblioteca" className="card" aria-labelledby="settings-biblioteca-title">
            <h2 id="settings-biblioteca-title">{t("settings.sections.library")}</h2>
            <div className="settings-row">
              <div className="settings-what">
                <strong>{settings?.rootPath ?? t("settings.diskNotConnected")}</strong>
                <span>{settings === null ? " " : volumeLine(settings)}</span>
              </div>
              <button type="button" className="button-link" disabled title={t("settings.oneLibrary")}>
                {t("onboarding.changeFolder")}
              </button>
            </div>
            <div className="settings-actions">
              <button type="button" className="button-solid" disabled={onScan === undefined} onClick={onScan}>
                {t("disk.scanNew")}
              </button>
              <button type="button" className="button-line" disabled title={t("settings.reindexLater")}>
                {t("settings.reindexAll")}
              </button>
            </div>
          </section>

          <section id="settings-dados" className="card" aria-labelledby="settings-dados-title">
            <h2 id="settings-dados-title">{t("settings.sections.data")}</h2>
            <dl className="settings-paths">
              <dt>{t("settings.database")}</dt>
              <dd>{settings === null ? " " : databaseLine(settings)}</dd>
              <dt>{t("section.trash")}</dt>
              <dd>{settings === null ? " " : trashLine(settings)}</dd>
            </dl>
            <div className="settings-cache">
              <div className="settings-cache-head">
                <label htmlFor="cache-limit">{t("settings.cacheLimit")}</label>
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
                      {clearing ? t("settings.clearing") : t("settings.clearThumbnails", { size: fileSize(settings?.thumbnailBytes ?? 0) })}
                    </button>
                    <button ref={cancelClear} type="button" className="button-line" disabled={clearing} onClick={() => setConfirming(false)}>
                      {t("common.cancel")}
                    </button>
                  </>
                ) : (
                  <button
                    ref={askClear}
                    type="button"
                    className="button-line"
                    disabled={settings === null || indexing || settings.thumbnailBytes === 0}
                    title={indexing ? t("settings.waitIndexing") : undefined}
                    onClick={() => setConfirming(true)}
                  >
                    {t("settings.clearThumbnailsShort")}
                  </button>
                )}
              </div>
              {confirming ? (
                <p className="settings-hint">{t("settings.clearHint")}</p>
              ) : null}
            </div>
          </section>

          <section id="settings-backup" className="card" aria-labelledby="settings-backup-title">
            <h2 id="settings-backup-title">{t("settings.sections.backup")}</h2>
            <dl className="settings-paths">
              <dt>{t("details.folder")}</dt>
              <dd>{backups?.folder ?? " "}</dd>
              <dt>{t("settings.lastBackup")}</dt>
              <dd className="settings-backup-last">{backups === null ? " " : lastBackupLine(backups)}</dd>
            </dl>
            <div className="settings-actions">
              <button
                type="button"
                className="button-line"
                disabled={backups === null || backingUp}
                onClick={() => backupAction(invoke<string>("backup_now").then(() => t("settings.backupDone")))}
              >
                {t("settings.backupNow")}
              </button>
              <button
                type="button"
                className="button-line"
                disabled={backups === null || backingUp}
                onClick={() =>
                  backupAction(invoke<string>("appdata_export").then((file) => t("settings.exported", { file })))
                }
              >
                {t("settings.exportData")}
              </button>
            </div>
            <div className="settings-import">
              <label htmlFor="backup-import">{t("settings.importFrom")}</label>
              <div className="settings-actions">
                <select
                  id="backup-import"
                  value={chosenExport}
                  disabled={backups === null || exportOptions(backups).length === 0}
                  onChange={(event) => setChosenExport(event.target.value)}
                >
                  {backups === null || exportOptions(backups).length === 0 ? (
                    <option value="">{t("settings.noExports")}</option>
                  ) : (
                    exportOptions(backups).map((option) => (
                      <option key={option.name} value={option.name}>
                        {option.label}
                      </option>
                    ))
                  )}
                </select>
                <button
                  type="button"
                  className="button-line"
                  disabled={chosenExport === "" || backingUp || indexing}
                  title={indexing ? t("settings.waitIndexing") : undefined}
                  onClick={() => backupAction(invoke<ImportOutcome>("appdata_import", { name: chosenExport }).then(importText))}
                >
                  {t("settings.import")}
                </button>
              </div>
              <p className="settings-hint">
                {t("settings.importHint")}
              </p>
            </div>
          </section>

          <section id="settings-indexacao" className="card" aria-labelledby="settings-indexacao-title">
            <h2 id="settings-indexacao-title">{t("settings.sections.indexing")}</h2>
            <div className="settings-switches">
              <label className="settings-switch">
                <input
                  type="checkbox"
                  role="switch"
                  checked={settings?.videoCovers ?? true}
                  disabled={settings === null}
                  onChange={(event) => setFlag(VIDEO_COVERS, event.target.checked)}
                />
                <span>{t("settings.videoCovers")}</span>
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
                <span>{t("settings.scanOnConnect")}</span>
              </label>
            </div>
            <div className="settings-ignored">
              <span>{t("settings.ignoredFolders")}</span>
              <div>
                {(settings?.ignoredFolders ?? []).map((name) => (
                  <code key={name}>{name}</code>
                ))}
              </div>
            </div>
          </section>

          <section id="settings-aparencia" className="card" aria-labelledby="settings-aparencia-title">
            <h2 id="settings-aparencia-title">{t("settings.sections.appearance")}</h2>
            <fieldset className="choice-row">
              <legend>{t("settings.theme")}</legend>
              {THEME_OPTIONS.map((option) => (
                <label key={option.choice} className="choice" data-on={option.choice === theme ? "" : undefined}>
                  <input type="radio" name="theme" checked={option.choice === theme} onChange={() => onTheme(option.choice)} />
                  {option.label}
                </label>
              ))}
            </fieldset>
            <div className="settings-selects">
              <label>
                {t("settings.motion")}
                <select value={motion} onChange={(event) => onMotion(event.target.value as MotionChoice)}>
                  {MOTION_OPTIONS.map((option) => (
                    <option key={option.choice} value={option.choice}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                {t("settings.language")}
                {/* F34: um idioma por arquivo em src/i18n; com um só, a escolha fica travada. */}
                <select value={currentLocale()} disabled={Object.keys(LOCALES).length < 2}>
                  {Object.entries(LOCALES).map(([code, messages]) => (
                    <option key={code} value={code}>
                      {String(messages["language.name"])}
                    </option>
                  ))}
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
              <h2 id="settings-privacidade-title">{t("settings.sections.privacy")}</h2>
              <span>{privacyNote(settings)}</span>
              <UpdatesBlock settings={settings} onFlag={(on) => setFlag(UPDATE_CHECKS, on)} />
            </div>
          </section>
        </div>
      </div>
    </main>
  );
}
