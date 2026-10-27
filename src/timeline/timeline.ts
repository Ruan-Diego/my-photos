// F18: a linha do tempo (artboard `Timeline`): agrupar por mês e dia, montar as linhas
// justificadas e saber quais linhas estão na tela. Sem React: testável no Node.

/** O que o comando `timeline` devolve: um vetor por campo, mais novo primeiro. */
export interface TimelineData {
  ids: number[];
  /** Segundos Unix (UTC); `null` sem data. */
  times: (number | null)[];
  /** Minutos a leste do UTC quando o arquivo disse onde foi; `null` usa o fuso do PC. */
  offsets: (number | null)[];
  ratios: number[];
  durations: (number | null)[];
  flags: number[];
}

export const VIDEO = 1;
export const FAVORITE = 2;

export const EMPTY: TimelineData = { ids: [], times: [], offsets: [], ratios: [], durations: [], flags: [] };

export type Filter = "all" | "photos" | "videos" | "favorites";

export const FILTERS: readonly { id: Filter; label: string }[] = [
  { id: "all", label: "Tudo" },
  { id: "photos", label: "Fotos" },
  { id: "videos", label: "Vídeos" },
  { id: "favorites", label: "Favoritos" },
];

export interface Item {
  id: number;
  ratio: number;
  video: boolean;
  favorite: boolean;
  /** ms, só de vídeo com duração conhecida. */
  duration: number | null;
  /** Segundos Unix e fuso, como no `TimelineData`. */
  time: number | null;
  offset: number | null;
}

export interface Day {
  key: string;
  label: string;
  items: Item[];
}

export interface Month {
  key: string;
  title: string;
  /** `null` no grupo "Sem data". */
  year: number | null;
  days: Day[];
}

export const MONTHS = ["Janeiro", "Fevereiro", "Março", "Abril", "Maio", "Junho", "Julho", "Agosto", "Setembro", "Outubro", "Novembro", "Dezembro"];
export const WEEKDAYS = ["Dom", "Seg", "Ter", "Qua", "Qui", "Sex", "Sáb"];
const count = new Intl.NumberFormat("pt-BR");

export interface Calendar {
  year: number;
  month: number;
  day: number;
  weekday: number;
  hour: number;
  minute: number;
}

/** O dia em que a foto foi tirada: no fuso dela quando conhecido, senão no do PC. */
export function calendar(time: number, offset: number | null): Calendar {
  if (offset !== null) {
    const date = new Date((time + offset * 60) * 1000);
    return {
      year: date.getUTCFullYear(),
      month: date.getUTCMonth(),
      day: date.getUTCDate(),
      weekday: date.getUTCDay(),
      hour: date.getUTCHours(),
      minute: date.getUTCMinutes(),
    };
  }
  const date = new Date(time * 1000);
  return {
    year: date.getFullYear(),
    month: date.getMonth(),
    day: date.getDate(),
    weekday: date.getDay(),
    hour: date.getHours(),
    minute: date.getMinutes(),
  };
}

/** "Agosto de 2022" */
export function monthTitle(year: number, month: number): string {
  return `${MONTHS[month]} de ${year}`;
}

/** "Qua, 10 de ago" */
export function dayLabel(when: Calendar): string {
  return `${WEEKDAYS[when.weekday]}, ${when.day} de ${MONTHS[when.month].slice(0, 3).toLowerCase()}`;
}

/** "14 itens". O canvas põe o local antes; o índice ainda não tem nome de lugar. */
export function dayDetail(items: number): string {
  return items === 1 ? "1 item" : `${count.format(items)} itens`;
}

/** "2:18", "1:02:03" */
export function duration(ms: number): string {
  const total = Math.round(ms / 1000);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = String(total % 60).padStart(2, "0");
  return hours > 0 ? `${hours}:${String(minutes).padStart(2, "0")}:${seconds}` : `${minutes}:${seconds}`;
}

function keep(flags: number, filter: Filter): boolean {
  switch (filter) {
    case "all":
      return true;
    case "photos":
      return (flags & VIDEO) === 0;
    case "videos":
      return (flags & VIDEO) !== 0;
    case "favorites":
      return (flags & FAVORITE) !== 0;
  }
}

export const UNDATED = "Sem data";

/** Meses e dias na ordem que chegam (mais novo primeiro); sem data por último. */
export function group(data: TimelineData, filter: Filter): Month[] {
  const months: Month[] = [];
  let month: Month | null = null;
  let day: Day | null = null;
  for (let index = 0; index < data.ids.length; index += 1) {
    const flags = data.flags[index];
    if (!keep(flags, filter)) {
      continue;
    }
    const time = data.times[index];
    let monthKey = "undated";
    let dayKey = "undated";
    let when: Calendar | null = null;
    if (time !== null) {
      when = calendar(time, data.offsets[index]);
      monthKey = `${when.year}-${String(when.month + 1).padStart(2, "0")}`;
      dayKey = `${monthKey}-${String(when.day).padStart(2, "0")}`;
    }
    if (month === null || month.key !== monthKey) {
      month = {
        key: monthKey,
        title: when === null ? UNDATED : monthTitle(when.year, when.month),
        year: when === null ? null : when.year,
        days: [],
      };
      months.push(month);
      day = null;
    }
    if (day === null || day.key !== dayKey) {
      day = { key: dayKey, label: when === null ? "Data não encontrada no arquivo nem no JSON" : dayLabel(when), items: [] };
      month.days.push(day);
    }
    day.items.push({
      id: data.ids[index],
      ratio: data.ratios[index],
      video: (flags & VIDEO) !== 0,
      favorite: (flags & FAVORITE) !== 0,
      duration: data.durations[index],
      time,
      offset: data.offsets[index],
    });
  }
  return months;
}

/** Anos com fotos, do mais novo ao mais antigo (trilho à direita). */
export function years(months: Month[]): number[] {
  const seen: number[] = [];
  for (const month of months) {
    if (month.year !== null && !seen.includes(month.year)) {
      seen.push(month.year);
    }
  }
  return seen;
}

/** Controle "Tamanho" (1–5); 3 é a altura do canvas, 176 px. */
export const ROW_HEIGHTS = [96, 128, 176, 232, 296] as const;
export const DEFAULT_SIZE = 3;

export function rowHeight(size: number): number {
  return ROW_HEIGHTS[Math.min(ROW_HEIGHTS.length, Math.max(1, Math.round(size))) - 1];
}

export const GAP = 4;
// Medidas do canvas: h2 do mês 22 px com 16 px acima; 14 px até o dia; h3 do dia 15 px
// com 8 px até a grade; 14 px entre dias; 28 px entre meses.
export const MONTH_HEIGHT = 16 + 30 + 14;
export const DAY_HEIGHT = 22 + 8;
export const DAY_GAP = 14;
export const MONTH_GAP = 28 - DAY_GAP;
export const TOP_PADDING = 8;
export const BOTTOM_PADDING = 40;

/** Panoramas e fotos muito estreitas não tomam a linha inteira. */
export function clampRatio(ratio: number): number {
  return Math.min(3, Math.max(0.5, ratio > 0 ? ratio : 1));
}

export function basis(ratio: number, height: number): number {
  return Math.round(clampRatio(ratio) * height);
}

/**
 * Quebra como o `flex-wrap` do canvas: cabe enquanto a soma das larguras-base mais os
 * espaços não passa da largura. Cada linha completa estica para preencher; a última
 * fica na proporção da foto.
 */
export function rows(items: Item[], width: number, height: number): Item[][] {
  const result: Item[][] = [];
  let row: Item[] = [];
  let used = 0;
  for (const item of items) {
    const size = basis(item.ratio, height);
    const next = row.length === 0 ? size : used + GAP + size;
    if (row.length > 0 && next > width) {
      result.push(row);
      row = [item];
      used = size;
    } else {
      row.push(item);
      used = next;
    }
  }
  if (row.length > 0) {
    result.push(row);
  }
  return result;
}

export type Line =
  | { kind: "month"; key: string; top: number; height: number; title: string; year: number | null }
  | { kind: "day"; key: string; top: number; height: number; label: string; detail: string }
  | { kind: "tiles"; key: string; top: number; height: number; items: Item[]; last: boolean };

export interface Layout {
  lines: Line[];
  height: number;
  /** Onde cada ano começa, para o trilho. */
  yearTops: Map<number, number>;
}

/** Tudo em linhas de altura conhecida: só as visíveis vão para o DOM. */
export function layout(months: Month[], width: number, height: number): Layout {
  const lines: Line[] = [];
  const yearTops = new Map<number, number>();
  let top = TOP_PADDING;
  months.forEach((month, monthIndex) => {
    if (monthIndex > 0) {
      top += MONTH_GAP;
    }
    if (month.year !== null && !yearTops.has(month.year)) {
      yearTops.set(month.year, top);
    }
    lines.push({ kind: "month", key: `m${month.key}`, top, height: MONTH_HEIGHT, title: month.title, year: month.year });
    top += MONTH_HEIGHT;
    month.days.forEach((day, dayIndex) => {
      if (dayIndex > 0) {
        top += DAY_GAP;
      }
      lines.push({ kind: "day", key: `d${day.key}`, top, height: DAY_HEIGHT, label: day.label, detail: dayDetail(day.items.length) });
      top += DAY_HEIGHT;
      const dayRows = rows(day.items, Math.max(1, width), height);
      dayRows.forEach((row, rowIndex) => {
        const last = rowIndex === dayRows.length - 1;
        const lineHeight = last ? height : height + GAP;
        lines.push({ kind: "tiles", key: `t${day.key}-${rowIndex}`, top, height: lineHeight, items: row, last });
        top += lineHeight;
      });
    });
  });
  return { lines, height: top + BOTTOM_PADDING, yearTops };
}

/** A primeira linha cujo fim passa de `y`. */
function firstEndingAfter(lines: Line[], y: number): number {
  let low = 0;
  let high = lines.length;
  while (low < high) {
    const middle = (low + high) >> 1;
    if (lines[middle].top + lines[middle].height <= y) {
      low = middle + 1;
    } else {
      high = middle;
    }
  }
  return low;
}

/** Índices [início, fim) das linhas entre `scrollTop` e o fim da janela, com folga. */
export function visible(lines: Line[], scrollTop: number, viewport: number, overscan = viewport): [number, number] {
  const start = firstEndingAfter(lines, scrollTop - overscan);
  let end = start;
  const bottom = scrollTop + viewport + overscan;
  while (end < lines.length && lines[end].top < bottom) {
    end += 1;
  }
  return [start, end];
}

/** O ano do mês no topo da tela (o trilho o destaca). */
export function yearAt(lines: Line[], scrollTop: number): number | null {
  const index = Math.min(lines.length - 1, firstEndingAfter(lines, scrollTop));
  for (let at = index; at >= 0; at -= 1) {
    const line = lines[at];
    if (line.kind === "month") {
      return line.year;
    }
  }
  return null;
}

export function emptyText(filter: Filter, total: number): { title: string; detail: string } {
  if (total === 0) {
    return { title: "Nenhuma foto ainda", detail: "Indexe a biblioteca para ver suas fotos aqui." };
  }
  switch (filter) {
    case "videos":
      return { title: "Nenhum vídeo", detail: "A biblioteca não tem vídeos indexados." };
    case "favorites":
      return { title: "Nenhum favorito", detail: "Toque no coração ao ver uma foto ou vídeo. As favoritas do Google Fotos também aparecem aqui." };
    default:
      return { title: "Nenhuma foto", detail: "A biblioteca só tem vídeos." };
  }
}
