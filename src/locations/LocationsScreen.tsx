import { useEffect, useLayoutEffect, useMemo, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { thumbnailUrl } from "../viewer/media";
import {
  clampScale,
  fitView,
  locationsSubtitle,
  pinLabel,
  pins,
  pinSize,
  placeHash,
  placeMeta,
  placeName,
  type Locations,
  type Place,
  type View,
} from "./locations";
import { WORLD_PATH } from "./world";
import { formatCount, t } from "../i18n/i18n";

const LIST_LIMIT = 100;

function useLocations(): [Locations | null, string | null] {
  const [locations, setLocations] = useState<Locations | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  useEffect(() => {
    invoke<Locations>("locations_overview")
      .then(setLocations)
      .catch((error) => setFailure(String(error)));
  }, []);
  return [locations, failure];
}

function useSize(element: HTMLElement | null): { width: number; height: number } {
  const [size, setSize] = useState({ width: 0, height: 0 });
  useLayoutEffect(() => {
    if (element === null) {
      return;
    }
    const measure = () => setSize({ width: element.clientWidth, height: element.clientHeight });
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [element]);
  return size;
}

/** Mapa offline: contorno dos continentes, alfinetes agrupados, arrastar e zoom. */
function PlacesMap({ places }: { places: Place[] }) {
  const [frame, setFrame] = useState<HTMLElement | null>(null);
  const { width, height } = useSize(frame);
  const [view, setView] = useState<View | null>(null);
  const drag = useRef<{ x: number; y: number; view: View; moved: boolean } | null>(null);

  // A primeira vista enquadra todos os lugares.
  useEffect(() => {
    if (view === null && width > 0 && height > 0) {
      setView(fitView(places, width, height));
    }
  }, [view, width, height, places]);

  const current = view ?? fitView(places, width, height);
  const shown = useMemo(() => pins(places, current.scale), [places, current.scale]);
  const toScreen = (x: number, y: number) => ({
    left: width / 2 + (x - current.x) * current.scale,
    top: height / 2 + (y - current.y) * current.scale,
  });
  const zoom = (factor: number, center?: { x: number; y: number }) =>
    setView((before) => {
      const base = before ?? current;
      const scale = clampScale(base.scale * factor);
      return { x: center?.x ?? base.x, y: center?.y ?? base.y, scale };
    });

  const onPointerDown = (event: ReactPointerEvent<HTMLElement>) => {
    if ((event.target as HTMLElement).closest("button") !== null) {
      return;
    }
    drag.current = { x: event.clientX, y: event.clientY, view: current, moved: false };
    event.currentTarget.setPointerCapture(event.pointerId);
  };
  const onPointerMove = (event: ReactPointerEvent<HTMLElement>) => {
    const start = drag.current;
    if (start === null) {
      return;
    }
    const dx = event.clientX - start.x;
    const dy = event.clientY - start.y;
    start.moved ||= Math.hypot(dx, dy) > 3;
    setView({ ...start.view, x: start.view.x - dx / start.view.scale, y: start.view.y - dy / start.view.scale });
  };
  const onPointerUp = () => {
    drag.current = null;
  };

  // Roda do mouse: zoom em volta do ponteiro.
  useEffect(() => {
    if (frame === null) {
      return;
    }
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      const box = frame.getBoundingClientRect();
      setView((before) => {
        if (before === null) {
          return before;
        }
        const factor = event.deltaY < 0 ? 1.25 : 0.8;
        const scale = clampScale(before.scale * factor);
        const px = event.clientX - box.left - box.width / 2;
        const py = event.clientY - box.top - box.height / 2;
        // O ponto sob o ponteiro fica onde está.
        return {
          scale,
          x: before.x + px / before.scale - px / scale,
          y: before.y + py / before.scale - py / scale,
        };
      });
    };
    frame.addEventListener("wheel", onWheel, { passive: false });
    return () => frame.removeEventListener("wheel", onWheel);
  }, [frame]);

  const origin = toScreen(0, 0);
  return (
    <section
      ref={setFrame}
      className="locations-map"
      aria-label={t("locations.map")}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
    >
      {width > 0 ? (
        <svg className="locations-land" width={width} height={height} aria-hidden="true">
          <path
            d={WORLD_PATH}
            transform={`translate(${origin.left} ${origin.top}) scale(${current.scale})`}
            vectorEffect="non-scaling-stroke"
          />
        </svg>
      ) : null}
      {width > 0
        ? shown.map((pin) => {
            const { left, top } = toScreen(pin.x, pin.y);
            if (left < -40 || top < -40 || left > width + 40 || top > height + 40) {
              return null;
            }
            const size = pinSize(pin.count);
            const single = pin.places.length === 1;
            const name = single ? placeName(pin.places[0].label) : t("locations.andMore", { name: placeName(pin.places[0].label), count: pin.places.length - 1 });
            return (
              <button
                key={pin.places[0].label}
                type="button"
                className="locations-pin"
                style={{ left, top, width: size, height: size }}
                aria-label={t("locations.pinLabel", { name, items: t("count.items", { n: pin.count }) })}
                title={name}
                onClick={() => {
                  if (single) {
                    window.location.hash = placeHash(pin.places[0].label);
                  } else {
                    zoom(2.5, { x: pin.x, y: pin.y });
                  }
                }}
              >
                {pinLabel(pin.count)}
              </button>
            );
          })
        : null}
      <div className="locations-zoom">
        <button type="button" aria-label={t("locations.zoomIn")} onClick={() => zoom(1.6)}>
          +
        </button>
        <button type="button" aria-label={t("locations.zoomOut")} onClick={() => zoom(1 / 1.6)}>
          −
        </button>
      </div>
      <span className="locations-credit">Natural Earth · GeoNames</span>
    </section>
  );
}

function PlaceRow({ place }: { place: Place }) {
  const [broken, setBroken] = useState(false);
  return (
    <a className="locations-place" href={placeHash(place.label)} draggable={false}>
      <span className="locations-cover" aria-hidden="true">
        {place.cover !== null && !broken ? (
          <img src={thumbnailUrl(place.cover)} alt="" loading="lazy" decoding="async" draggable={false} onError={() => setBroken(true)} />
        ) : null}
      </span>
      <span className="locations-place-text">
        <strong>{placeName(place.label)}</strong>
        <span>{placeMeta(place)}</span>
      </span>
      <span className="locations-count">{formatCount(place.count)}</span>
    </a>
  );
}

/** F24: lugares das fotos (artboard `Locations`); cada um abre a busca filtrada. */
export function LocationsScreen() {
  const [locations, failure] = useLocations();
  const subtitle =
    failure !== null ? t("locations.readFailed") : locations === null ? " " : locationsSubtitle(locations.located, locations.unlocated);

  return (
    <main className="page locations" aria-labelledby="locations-title">
      <header className="page-header">
        <div className="people-heading">
          <h1 id="locations-title">{t("section.locations")}</h1>
          <span>{subtitle}</span>
        </div>
      </header>
      {failure !== null ? (
        <div className="page-body">
          <div className="page-empty">
            <p className="page-empty-title">{t("locations.readFailed")}</p>
            <p className="page-empty-detail">{failure}</p>
          </div>
        </div>
      ) : locations !== null && locations.places.length === 0 ? (
        <div className="page-body">
          <div className="page-empty">
            <p className="page-empty-title">{t("locations.emptyTitle")}</p>
            <p className="page-empty-detail">
              {t("locations.emptyDetail")}
            </p>
          </div>
        </div>
      ) : locations !== null ? (
        <div className="locations-body">
          <PlacesMap places={locations.places} />
          <aside className="locations-list" aria-label={t("locations.places")}>
            <h2>{t("locations.topPlaces")}</h2>
            {locations.places.slice(0, LIST_LIMIT).map((place) => (
              <PlaceRow key={place.label} place={place} />
            ))}
          </aside>
        </div>
      ) : null}
    </main>
  );
}
