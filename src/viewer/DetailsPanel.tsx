import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Item } from "../timeline/timeline";
import type { MediaInfo } from "./media";
import { t } from "../i18n/i18n";
import { coordinates, dateSource, DETAILS_KEY, fileRows, fullDate, technicalRows, type MediaPanel } from "./panel";

function loadOpen(): boolean {
  try {
    return window.localStorage.getItem(DETAILS_KEY) === "open";
  } catch {
    return false;
  }
}

/** Painel aberto ou fechado, lembrado entre fotos, vídeos e sessões. */
export function useDetailsOpen(): [boolean, () => void] {
  const [open, setOpen] = useState(loadOpen);
  const toggle = () => {
    setOpen((current) => {
      try {
        window.localStorage.setItem(DETAILS_KEY, current ? "closed" : "open");
      } catch {
        // Sem armazenamento: só não fica para a próxima vez.
      }
      return !current;
    });
  };
  return [open, toggle];
}

function useMediaPanel(id: number): MediaPanel | null {
  const [panel, setPanel] = useState<MediaPanel | null>(null);
  useEffect(() => {
    let current = true;
    setPanel(null);
    invoke<MediaPanel | null>("media_panel_details", { id })
      .then((found) => {
        if (current) {
          setPanel(found);
        }
      })
      .catch(() => undefined);
    return () => {
      current = false;
    };
  }, [id]);
  return panel;
}

function Rows({ rows }: { rows: [string, string][] }) {
  return (
    <dl className="details-rows">
      {rows.map(([label, value]) => (
        <div key={label} className="details-row">
          <dt>{label}</dt>
          <dd className={label === "SHA-256" ? "details-mono" : undefined}>{value}</dd>
        </div>
      ))}
    </dl>
  );
}

/** F21: painel Detalhes do artboard `Viewer`, ao lado da foto ou do vídeo. */
export function DetailsPanel({ item, info }: { item: Item; info: MediaInfo | null }) {
  const panel = useMediaPanel(item.id);
  const ready = panel !== null && panel.id === item.id;
  const when = fullDate(item);
  const source = ready ? dateSource(panel.dateSource) : null;
  const place = ready ? coordinates(panel.latitude, panel.longitude) : null;

  return (
    <aside className="details" aria-label={t("details.title")}>
      <h2>{t("details.title")}</h2>

      <section className="details-section">
        <span className="details-label">{t("details.dateAndTime")}</span>
        <strong className="details-value">{when ?? t("timeline.undated")}</strong>
        {source !== null ? (
          <span className={`details-badge details-badge-${source.tone}`}>
            {source.tone === "ok" ? (
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <path d="M5 12l5 5 9-10" />
              </svg>
            ) : null}
            {source.label}
          </span>
        ) : when === null ? (
          <span className="details-muted">{t("timeline.undatedDetail")}</span>
        ) : null}
      </section>

      <section className="details-section">
        <span className="details-label">{t("details.place")}</span>
        {place !== null ? (
          <>
            <div className="details-map" aria-hidden="true">
              <span className="details-pin" />
            </div>
            <strong className="details-value details-place">{place}</strong>
          </>
        ) : (
          <span className="details-muted">{ready ? t("details.noPlace") : " "}</span>
        )}
      </section>

      <section className="details-section">
        <span className="details-label">{t("details.people")}</span>
        {ready && panel.people.length > 0 ? (
          <ul className="details-people">
            {panel.people.map((name) => (
              <li key={name} className="details-person">
                <span aria-hidden="true">{name.slice(0, 1).toUpperCase()}</span>
                {name}
              </li>
            ))}
          </ul>
        ) : (
          <span className="details-muted">{ready ? t("details.noPeople") : " "}</span>
        )}
      </section>

      <section className="details-section">
        <span className="details-label">{t("details.file")}</span>
        {ready && info !== null && info.id === item.id ? <Rows rows={fileRows(info, panel)} /> : null}
      </section>

      {ready ? (
        <details className="details-technical">
          <summary>{t("details.technical")}</summary>
          <Rows rows={technicalRows(panel)} />
        </details>
      ) : null}
    </aside>
  );
}
