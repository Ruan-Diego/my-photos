import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { Item } from "../timeline/timeline";
import { DetailsPanel, useDetailsOpen } from "./DetailsPanel";
import { useToggleFavorite } from "../favorites/FavoriteContext";
import { fit, icon, originalUrl, thumbnailUrl, useMediaInfo } from "./media";
import { backHash, backLabel, clock, dateLine, seek, SEEK_STEP, videoLine, videoProblem, type ViewerBase } from "./navigation";
import { useDialogFocus } from "../a11y/useDialogFocus";
import { t } from "../i18n/i18n";

const VOLUME_KEY = "myphotos.player.volume";
/** Largura máxima do quadro no canvas. */
const MAX_WIDTH = 1180;

function loadVolume(): number {
  try {
    const stored = Number(window.localStorage.getItem(VOLUME_KEY) ?? "70");
    return stored >= 0 && stored <= 100 ? stored : 70;
  } catch {
    return 70;
  }
}

function saveVolume(volume: number) {
  try {
    window.localStorage.setItem(VOLUME_KEY, String(volume));
  } catch {
    // Sem armazenamento: o volume só não fica para a próxima vez.
  }
}

/** Fim do trecho já carregado que contém `time`, em segundos. */
function bufferedEnd(video: HTMLVideoElement): number {
  const ranges = video.buffered;
  for (let index = 0; index < ranges.length; index += 1) {
    if (ranges.start(index) <= video.currentTime && video.currentTime <= ranges.end(index)) {
      return ranges.end(index);
    }
  }
  return video.currentTime;
}

/** F20: player do artboard `Video`. Toca o original direto do disco, em trechos. */
export function VideoPlayer({ item, base = "fotos" }: { item: Item; base?: ViewerBase }) {
  const info = useMediaInfo(item.id);
  const toggleFavorite = useToggleFavorite();
  const video = useRef<HTMLVideoElement>(null);
  const frame = useRef<HTMLDivElement>(null);
  const root = useRef<HTMLDivElement>(null);
  useDialogFocus(root, { trap: true });
  const stage = useRef<HTMLDivElement>(null);
  const [area, setArea] = useState({ width: 0, height: 0 });
  const [paused, setPaused] = useState(true);
  const [time, setTime] = useState(0);
  const [duration, setDuration] = useState(item.duration === null ? 0 : item.duration / 1000);
  const [buffered, setBuffered] = useState(0);
  const [volume, setVolume] = useState(loadVolume);
  const [failed, setFailed] = useState(false);
  const [fullscreen, setFullscreen] = useState(false);
  const [detailsOpen, toggleDetails] = useDetailsOpen();

  const problem = videoProblem(info, failed);
  const ready = info !== null && info.id === item.id && problem === null;

  useEffect(() => {
    if (video.current !== null) {
      video.current.volume = volume / 100;
    }
  }, [volume, ready]);

  useLayoutEffect(() => {
    const element = stage.current;
    if (element === null) {
      return;
    }
    const measure = () => {
      const styles = window.getComputedStyle(element);
      setArea({
        width: Math.min(MAX_WIDTH, element.clientWidth - parseFloat(styles.paddingLeft) - parseFloat(styles.paddingRight)),
        height: element.clientHeight - parseFloat(styles.paddingTop) - parseFloat(styles.paddingBottom),
      });
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    const follow = () => setFullscreen(document.fullscreenElement !== null);
    document.addEventListener("fullscreenchange", follow);
    return () => document.removeEventListener("fullscreenchange", follow);
  }, []);

  const toggle = () => {
    const element = video.current;
    if (element === null) {
      return;
    }
    if (element.paused) {
      void element.play().catch(() => setFailed(true));
    } else {
      element.pause();
    }
  };

  const toggleFullscreen = () => {
    if (document.fullscreenElement !== null) {
      void document.exitFullscreen();
    } else {
      void frame.current?.requestFullscreen();
    }
  };

  const jump = (to: number) => {
    const element = video.current;
    if (element !== null) {
      element.currentTime = to;
      setTime(to);
    }
  };

  // Espaço, ← →, F e Esc, como no rodapé do canvas.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.target instanceof HTMLInputElement && event.key !== "Escape") {
        return;
      }
      const element = video.current;
      if (event.key === "Escape") {
        if (document.fullscreenElement !== null) {
          return;
        }
        window.location.hash = backHash(base);
      } else if (!ready || element === null) {
        return;
      } else if (event.key === " ") {
        toggle();
      } else if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
        jump(seek(element.currentTime, event.key === "ArrowLeft" ? -SEEK_STEP : SEEK_STEP, element.duration));
      } else if (event.key === "f" || event.key === "F") {
        toggleFullscreen();
      } else {
        return;
      }
      event.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  const progress = duration > 0 ? Math.min(1, time / duration) : 0;
  const loaded = duration > 0 ? Math.min(1, buffered / duration) : 0;
  // A capa já vem na proporção certa (girada); sem ela, 16:9.
  const box = fit(item.ratio > 0 ? item.ratio : 16 / 9, area.width, area.height);

  return (
    <div ref={root} className="viewer player" role="dialog" aria-modal="true" aria-label={t("player.dialog")}>
      <div className="viewer-main">
        <header className="viewer-header">
          <a className="viewer-icon" href={backHash(base)} aria-label={backLabel(base)}>
            <svg width="22" height="22" viewBox="0 0 24 24" {...icon} aria-hidden="true">
              <path d="M19 12H5M11 6l-6 6 6 6" />
            </svg>
          </a>
          <div className="viewer-title">
            <span>{dateLine(item)}</span>
            <span>{info === null ? " " : videoLine(info)}</span>
          </div>
          <button
            type="button"
            className="viewer-icon viewer-favorite"
            aria-label={item.favorite ? t("viewer.unfavorite") : t("viewer.favorite")}
            title={item.favorite ? t("viewer.unfavorite") : t("viewer.favorite")}
            aria-pressed={item.favorite}
            disabled={toggleFavorite === null}
            onClick={() => toggleFavorite?.(item.id, !item.favorite)}
          >
            <svg width="22" height="22" viewBox="0 0 24 24" {...icon} fill={item.favorite ? "currentColor" : "none"} aria-hidden="true">
              <path d="M12 20s-7-4.4-7-10a4 4 0 0 1 7-2.6A4 4 0 0 1 19 10c0 5.6-7 10-7 10z" />
            </svg>
          </button>
          <button
            type="button"
            className="viewer-icon viewer-toggle"
            aria-label={detailsOpen ? t("viewer.hideDetails") : t("viewer.showDetails")}
            aria-pressed={detailsOpen}
            onClick={toggleDetails}
          >
            <svg width="22" height="22" viewBox="0 0 24 24" {...icon} aria-hidden="true">
              <circle cx="12" cy="12" r="9" />
              <path d="M12 11v6M12 7.5v.01" />
            </svg>
          </button>
        </header>

        <div ref={stage} className="player-stage">
          <div
            ref={frame}
            className={fullscreen ? "player-frame player-frame-full" : "player-frame"}
            style={fullscreen ? undefined : { width: box.width, height: box.height }}
          >
            {ready ? (
              <video
                ref={video}
                className="player-video"
                src={originalUrl(item.id)}
                poster={thumbnailUrl(item.id)}
                preload="metadata"
                playsInline
                onClick={toggle}
                onPlay={() => setPaused(false)}
                onPause={() => setPaused(true)}
                onTimeUpdate={(event) => setTime(event.currentTarget.currentTime)}
                onDurationChange={(event) => {
                  if (Number.isFinite(event.currentTarget.duration)) {
                    setDuration(event.currentTarget.duration);
                  }
                }}
                onProgress={(event) => setBuffered(bufferedEnd(event.currentTarget))}
                onError={() => setFailed(true)}
              />
            ) : (
              <img className="player-video" src={thumbnailUrl(item.id)} alt="" draggable={false} />
            )}
            {problem !== null ? <span className="viewer-note player-note">{problem}</span> : null}
            {ready && paused ? (
              <button type="button" className="player-big" aria-label={t("player.play")} onClick={toggle}>
                <svg width="34" height="34" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
                  <path d="M8 5l12 7-12 7z" />
                </svg>
              </button>
            ) : null}
            {ready ? (
              <div className="player-controls">
                <div className="player-track">
                  <div className="player-loaded" style={{ width: `${loaded * 100}%` }} />
                  <div className="player-played" style={{ width: `${progress * 100}%` }} />
                  <span className="player-knob" style={{ left: `${progress * 100}%` }} />
                  <input
                    type="range"
                    className="player-seek"
                    aria-label={t("player.progress")}
                    aria-valuetext={t("player.position", { time: clock(time), duration: clock(duration) })}
                    min={0}
                    max={duration || 0}
                    step={0.1}
                    value={Math.min(time, duration || 0)}
                    onChange={(event) => jump(Number(event.target.value))}
                  />
                </div>
                <div className="player-row">
                  <button type="button" className="player-button" aria-label={paused ? t("player.play") : t("player.pause")} onClick={toggle}>
                    <svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
                      <path d={paused ? "M8 5l12 7-12 7z" : "M7 5h4v14H7zM13 5h4v14h-4z"} />
                    </svg>
                  </button>
                  <span className="player-time">
                    {clock(time)} / {clock(duration)}
                  </span>
                  <span className="player-spacer" />
                  <label className="player-volume">
                    <svg width="20" height="20" viewBox="0 0 24 24" {...icon} aria-hidden="true">
                      <path d="M4 9h4l5-4v14l-5-4H4zM17 9a4 4 0 0 1 0 6" />
                    </svg>
                    <span className="visually-hidden">{t("player.volume")}</span>
                    <input
                      type="range"
                      min={0}
                      max={100}
                      value={volume}
                      onChange={(event) => {
                        const next = Number(event.target.value);
                        setVolume(next);
                        saveVolume(next);
                      }}
                    />
                  </label>
                  <button type="button" className="player-button" aria-label={fullscreen ? t("player.exitFullscreen") : t("player.fullscreen")} onClick={toggleFullscreen}>
                    <svg width="20" height="20" viewBox="0 0 24 24" {...icon} aria-hidden="true">
                      <path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5" />
                    </svg>
                  </button>
                </div>
              </div>
            ) : null}
          </div>
        </div>

        <footer className="player-footer">
          <span>{t("player.keys")}</span>
          <span>{ready ? t("player.playingOriginal") : " "}</span>
        </footer>
      </div>
      {detailsOpen ? <DetailsPanel item={item} info={info} /> : null}
    </div>
  );
}
