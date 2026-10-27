import { memo, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  DEFAULT_SIZE,
  duration,
  emptyText,
  FILTERS,
  group,
  layout,
  rowHeight,
  visible,
  scrollSpace,
  yearAt,
  years as yearsOf,
  basis,
  clampRatio,
  GAP,
  type Filter,
  type Item,
  type Line,
  type TimelineData,
} from "./timeline";
import { Viewer } from "../viewer/Viewer";
import { SEARCH_HASH } from "../search/query";
import { WHEEL_STEP_MS, wheelSize } from "../shortcuts/shortcuts";
import { indexOf, openedId, order, viewerHash } from "../viewer/navigation";

const SIZE_KEY = "myphotos.timeline.size";

function loadSize(): number {
  try {
    const stored = Number(window.localStorage.getItem(SIZE_KEY));
    return stored >= 1 && stored <= 5 ? stored : DEFAULT_SIZE;
  } catch {
    return DEFAULT_SIZE;
  }
}

function saveSize(size: number) {
  try {
    window.localStorage.setItem(SIZE_KEY, String(size));
  } catch {
    // Sem armazenamento: o tamanho só não fica para a próxima vez.
  }
}

/** `http://thumb.localhost/1066` no Windows: o protocolo `thumb` do Rust. */
function thumbnailUrl(id: number): string {
  return convertFileSrc(String(id), "thumb");
}

// F32: memo nas peças da grade; rolar só monta as linhas que entram na tela.
const Tile = memo(function Tile({ item, height, grow }: { item: Item; height: number; grow: boolean }) {
  const [loaded, setLoaded] = useState(false);
  const size = basis(item.ratio, height);
  return (
    <a
      className="tile"
      href={viewerHash(item.id)}
      aria-label={item.video ? "Vídeo" : "Foto"}
      draggable={false}
      style={{ flex: `${grow ? clampRatio(item.ratio) : 0} 1 ${size}px`, height }}
    >
      <img
        src={thumbnailUrl(item.id)}
        alt=""
        decoding="async"
        draggable={false}
        className={loaded ? "tile-image tile-image-loaded" : "tile-image"}
        onLoad={() => setLoaded(true)}
      />
      {item.video ? (
        <span className="tile-duration">
          <svg width="12" height="12" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M7 4l13 8-13 8z" />
          </svg>
          {item.duration === null ? "Vídeo" : duration(item.duration)}
        </span>
      ) : null}
      {item.favorite ? (
        <svg className="tile-favorite" width="20" height="20" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M12 20s-7-4.4-7-10a4 4 0 0 1 7-2.6A4 4 0 0 1 19 10c0 5.6-7 10-7 10z" />
        </svg>
      ) : null}
    </a>
  );
});

/** `offset`: onde a linha do tempo começa na área de rolagem (ver `scrollSpace`). */
const TimelineLine = memo(function TimelineLine({ line, height, offset }: { line: Line; height: number; offset: number }) {
  const style = { top: line.top + offset, height: line.height };
  switch (line.kind) {
    case "month":
      return (
        <h2 className="timeline-month" style={style}>
          {line.title}
        </h2>
      );
    case "day":
      return (
        <div className="timeline-day" style={style}>
          <h3>{line.label}</h3>
          <span>{line.detail}</span>
        </div>
      );
    case "tiles":
      return (
        <div className="timeline-row" style={{ ...style, gap: GAP }}>
          {line.items.map((item) => (
            <Tile key={item.id} item={item} height={height} grow={!line.last} />
          ))}
        </div>
      );
  }
});

/** F19: `#/fotos/<id>` abre o visualizador por cima da grade, que fica onde estava. */
function useOpened(): number | null {
  const [opened, setOpened] = useState(() => openedId(window.location.hash));
  useEffect(() => {
    const follow = () => setOpened(openedId(window.location.hash));
    window.addEventListener("hashchange", follow);
    return () => window.removeEventListener("hashchange", follow);
  }, []);
  return opened;
}

export function TimelineScreen({ data, failure }: { data: TimelineData | null; failure: string | null }) {
  const opened = useOpened();
  const [filter, setFilter] = useState<Filter>("all");
  const [size, setSize] = useState(loadSize);
  const scroller = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  const [viewport, setViewport] = useState(0);
  const [scrollTop, setScrollTop] = useState(0);

  useLayoutEffect(() => {
    const element = scroller.current;
    if (element === null) {
      return;
    }
    const measure = () => {
      const styles = window.getComputedStyle(element);
      const padding = parseFloat(styles.paddingLeft) + parseFloat(styles.paddingRight);
      setWidth(element.clientWidth - padding);
      setViewport(element.clientHeight);
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const height = rowHeight(size);
  const months = useMemo(() => (data === null ? [] : group(data, filter)), [data, filter]);
  const years = useMemo(() => yearsOf(months), [months]);
  const { lines, height: total, yearTops } = useMemo(() => layout(months, width, height), [months, width, height]);
  const space = scrollSpace(total, viewport);
  const position = scrollTop * space.ratio;
  const [start, end] = visible(lines, position, viewport);
  const currentYear = yearAt(lines, position);

  // Filtro novo, lista nova: volta ao topo.
  useEffect(() => {
    scroller.current?.scrollTo({ top: 0 });
    setScrollTop(0);
  }, [filter]);

  const jumpTo = useCallback(
    (year: number) => {
      const top = yearTops.get(year);
      if (top !== undefined) {
        scroller.current?.scrollTo({ top: top / space.ratio });
      }
    },
    [yearTops, space.ratio],
  );

  const changeSize = (value: number) => {
    // Mantém o mesmo ponto da biblioteca no topo ao mudar a altura das linhas.
    const element = scroller.current;
    const fraction = element !== null && element.scrollHeight > 0 ? element.scrollTop / element.scrollHeight : 0;
    setSize(value);
    saveSize(value);
    window.requestAnimationFrame(() => {
      if (element !== null) {
        element.scrollTop = fraction * element.scrollHeight;
      }
    });
  };

  // Ctrl + roda do mouse muda o tamanho, como o controle "Tamanho" (F30).
  const wheelAt = useRef(0);
  const resize = useRef(changeSize);
  resize.current = changeSize;
  const sizeNow = useRef(size);
  sizeNow.current = size;
  useEffect(() => {
    const element = scroller.current;
    if (element === null) {
      return;
    }
    const onWheel = (event: WheelEvent) => {
      if (!event.ctrlKey) {
        return;
      }
      event.preventDefault();
      if (event.timeStamp - wheelAt.current < WHEEL_STEP_MS) {
        return;
      }
      const value = wheelSize(sizeNow.current, event.deltaY);
      if (value !== sizeNow.current) {
        wheelAt.current = event.timeStamp;
        resize.current(value);
      }
    };
    element.addEventListener("wheel", onWheel, { passive: false });
    return () => element.removeEventListener("wheel", onWheel);
  }, []);

  const entries = useMemo(() => order(months), [months]);
  const openedIndex = opened === null ? -1 : indexOf(entries, opened);

  const empty = data !== null && months.length === 0 ? emptyText(filter, data.ids.length) : null;

  return (
    <main className="page timeline" aria-labelledby="page-title">
      <header className="page-header">
        <h1 id="page-title">Fotos</h1>
        <a className="timeline-search" href={SEARCH_HASH}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" aria-hidden="true">
            <path d="M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM20 20l-4-4" />
          </svg>
          <span>Buscar por pessoa, local, data, pasta…</span>
          <kbd>Ctrl F</kbd>
        </a>
        <div role="group" aria-label="Mostrar" className="segmented">
          {FILTERS.map((option) => (
            <button key={option.id} type="button" aria-pressed={filter === option.id} onClick={() => setFilter(option.id)}>
              {option.label}
            </button>
          ))}
        </div>
        <label className="timeline-size" title="Também com Ctrl + roda do mouse">
          Tamanho
          <input type="range" min={1} max={5} step={1} value={size} onChange={(event) => changeSize(Number(event.target.value))} />
        </label>
        <button type="button" className="timeline-select" disabled title="A seleção chega com as operações de arquivo">
          Selecionar
        </button>
      </header>

      <div className="timeline-body">
        <div
          ref={scroller}
          className="timeline-scroll"
          onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
          tabIndex={0}
          aria-label="Linha do tempo"
        >
          {failure !== null ? (
            <div className="page-empty">
              <p className="page-empty-title">Não foi possível ler a biblioteca</p>
              <p className="page-empty-detail">{failure}</p>
            </div>
          ) : empty !== null ? (
            <div className="page-empty">
              <p className="page-empty-title">{empty.title}</p>
              <p className="page-empty-detail">{empty.detail}</p>
            </div>
          ) : (
            <div className="timeline-canvas" style={{ height: space.canvas }}>
              {lines.slice(start, end).map((line) => (
                <TimelineLine key={line.key} line={line} height={height} offset={scrollTop - position} />
              ))}
            </div>
          )}
        </div>

        {years.length > 0 ? (
          <nav aria-label="Saltar para o ano" className="timeline-years">
            {years.map((year) => (
              <a
                key={year}
                href={`#/fotos`}
                aria-current={year === currentYear ? "true" : undefined}
                onClick={(event) => {
                  event.preventDefault();
                  jumpTo(year);
                }}
              >
                {year}
              </a>
            ))}
          </nav>
        ) : null}
      </div>
      {openedIndex >= 0 ? <Viewer entries={entries} index={openedIndex} /> : null}
    </main>
  );
}
