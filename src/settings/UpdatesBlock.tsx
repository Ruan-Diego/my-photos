import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { t } from "../i18n/i18n";
import type { Settings, UpdateAvailable } from "./settings";

type State =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "current" }
  | { kind: "available"; update: UpdateAvailable }
  | { kind: "installing" }
  | { kind: "failed"; error: string };

/** F37: dentro do cartão Privacidade, porque é a única conexão de rede do app (F36). */
export function UpdatesBlock({ settings, onFlag }: { settings: Settings | null; onFlag: (on: boolean) => void }) {
  const [state, setState] = useState<State>({ kind: "idle" });
  if (settings === null) {
    return null;
  }

  const check = () => {
    setState({ kind: "checking" });
    invoke<UpdateAvailable | null>("app_update_check")
      .then((update) => setState(update === null ? { kind: "current" } : { kind: "available", update }))
      .catch((error) => setState({ kind: "failed", error: String(error) }));
  };

  const install = () => {
    setState({ kind: "installing" });
    invoke("app_update_install").catch((error) => setState({ kind: "failed", error: String(error) }));
  };

  return (
    <div className="settings-updates">
      <strong>
        {t("settings.updates.heading", { version: settings.version })}
      </strong>
      {settings.updaterConfigured ? (
        <>
          <label className="settings-switch">
            <input type="checkbox" role="switch" checked={settings.updateChecks} onChange={(event) => onFlag(event.target.checked)} />
            <span>{t("settings.updates.checks")}</span>
          </label>
          <p className="settings-hint">{t("settings.updates.checksHint")}</p>
          <div className="settings-actions" aria-live="polite">
            {state.kind === "available" ? (
              <button type="button" className="button-solid" onClick={install}>
                {t("settings.updates.install")}
              </button>
            ) : (
              <button type="button" className="button-line" disabled={state.kind === "checking" || state.kind === "installing"} onClick={check}>
                {state.kind === "checking" ? t("settings.updates.checking") : t("settings.updates.check")}
              </button>
            )}
            {state.kind === "current" ? <span>{t("settings.updates.upToDate")}</span> : null}
            {state.kind === "available" ? <span>{t("settings.updates.available", { version: state.update.version })}</span> : null}
            {state.kind === "installing" ? <span>{t("settings.updates.installing")}</span> : null}
            {state.kind === "failed" ? <span role="alert">{t("settings.updates.failed", { error: state.error })}</span> : null}
          </div>
        </>
      ) : (
        <p className="settings-hint">{t("settings.updates.dev")}</p>
      )}
    </div>
  );
}
