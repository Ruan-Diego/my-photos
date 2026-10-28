import { motion } from "motion/react";
import { Logo } from "../shell/Sidebar";
import { hashFor } from "../shell/sections";
import {
  countCards,
  headline,
  label,
  overall,
  percentLine,
  problemsLine,
  stepStates,
  STEPS,
  type LogLine,
  type Progress,
  type RecentThumbnail,
} from "./progress";
import { t } from "../i18n/i18n";

/** Casas no painel "Chegando agora" (artboard `Indexing`: 6 × 3). */
export const TILES = 18;

export function IndexingScreen({
  root,
  progress,
  paused,
  pausing,
  elapsedMs,
  thumbnails,
  lines,
  onPause,
  onResume,
  onBrowse,
}: {
  root: string | null;
  progress: Progress;
  paused: boolean;
  pausing: boolean;
  elapsedMs: number;
  thumbnails: RecentThumbnail[];
  lines: LogLine[];
  onPause: () => void;
  onResume: () => void;
  onBrowse: () => void;
}) {
  const fraction = overall(progress);
  const percent = Math.floor(fraction * 100);
  const problems = problemsLine(progress.counts.problems, paused);
  const states = stepStates(progress.stage);
  const tiles = Array.from({ length: TILES }, (_, index) => thumbnails[index] ?? null);
  return (
    <main className="indexing">
      <header className="indexing-header">
        <div className="welcome-brand">
          <Logo size={28} />
          My Photos
        </div>
        <div className="indexing-actions">
          {paused ? (
            <button type="button" className="button-line" onClick={onResume}>
              {t("indexing.continue")}
            </button>
          ) : (
            <button type="button" className="button-line" onClick={onPause} disabled={pausing}>
              {pausing ? t("indexing.pausing") : t("indexing.pause")}
            </button>
          )}
          <a className="button-solid" href={hashFor("timeline")} onClick={onBrowse}>
            {t("indexing.browse")}
          </a>
        </div>
      </header>

      <section className="indexing-run">
        <div className="indexing-head">
          <div className="indexing-title">
            <span className="welcome-label">{label(root, paused)}</span>
            <h1>{headline(progress)}</h1>
          </div>
          <span className="indexing-percent">{paused ? `${percent}%` : percentLine(fraction, elapsedMs)}</span>
        </div>
        <div className="indexing-bar" role="progressbar" aria-label={t("indexing.progressLabel")} aria-valuenow={percent} aria-valuemin={0} aria-valuemax={100}>
          <motion.div className="indexing-bar-fill" initial={false} animate={{ width: `${fraction * 100}%` }} transition={{ duration: 0.4, ease: "easeOut" }} />
        </div>
        <ol className="indexing-steps">
          {STEPS.map((step, index) => (
            <li key={step} className={`indexing-step indexing-step-${states[index]}`} aria-current={states[index] === "current" ? "step" : undefined}>
              <span className="indexing-step-mark" aria-hidden="true">
                {states[index] === "done" ? "✓" : index + 1}
              </span>
              {step}
            </li>
          ))}
        </ol>
      </section>

      <section className="indexing-detail">
        <div className="indexing-counts">
          {countCards(progress.counts).map((card) => (
            <div key={card.label} className="indexing-count">
              <span>{card.label}</span>
              <strong>{card.value}</strong>
            </div>
          ))}
          {problems === null ? null : (
            <a className="indexing-problems" href={hashFor("problems")} onClick={onBrowse}>
              <span>{problems}</span>
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <path d="M5 12h14M13 6l6 6-6 6" />
              </svg>
            </a>
          )}
        </div>

        <div className="indexing-arrivals">
          <div className="indexing-arrivals-head">
            <h2>{t("indexing.arriving")}</h2>
            <span>{t("indexing.arrivingHint")}</span>
          </div>
          <div className="indexing-tiles">
            {tiles.map((thumbnail, index) =>
              thumbnail === null ? (
                <div key={`empty-${index}`} className="indexing-tile indexing-tile-empty" />
              ) : (
                <motion.img
                  key={thumbnail.mediaId}
                  className="indexing-tile"
                  src={thumbnail.src}
                  alt=""
                  initial={{ opacity: 0, scale: 0.92 }}
                  animate={{ opacity: 1, scale: 1 }}
                  transition={{ duration: 0.25 }}
                />
              ),
            )}
          </div>
          <div className="indexing-console" aria-live="polite">
            {lines.length === 0 ? <div className="indexing-console-quiet">{t("indexing.waiting")}</div> : null}
            {lines.map((line, index) => (
              <div key={`${index}-${line.text}`} className={line.warn ? "indexing-console-warn" : undefined}>
                {line.text}
              </div>
            ))}
          </div>
        </div>
      </section>
    </main>
  );
}
