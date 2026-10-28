// F21: o que o painel Detalhes (artboard `Viewer`) diz. Sem React: testável no Node.

import { calendar, duration, MONTHS, type Item } from "../timeline/timeline.ts";
import { fileSize } from "./navigation.ts";
import { numberFormat, t } from "../i18n/i18n.ts";

/** O que o comando `media_panel_details` devolve. */
export interface MediaPanel {
  id: number;
  dateSource: "takeout_json" | "exif" | "filesystem" | null;
  latitude: number | null;
  longitude: number | null;
  people: string[];
  folder: string;
  durationMs: number | null;
  jsonFilename: string | null;
  jsonMatch: "exact" | "heuristic" | "none" | null;
  camera: string | null;
  sha256: string | null;
}

/** "10 de agosto de 2022, 14:37"; `null` sem data. */
export function fullDate(item: Item): string | null {
  if (item.time === null) {
    return null;
  }
  const when = calendar(item.time, item.offset);
  const hour = `${String(when.hour).padStart(2, "0")}:${String(when.minute).padStart(2, "0")}`;
  return t("details.dateTime", { day: when.day, month: MONTHS[when.month].toLowerCase(), year: String(when.year), hour });
}

/** O selo embaixo da data: de onde ela veio. A data do disco é a menos confiável. */
export function dateSource(source: MediaPanel["dateSource"]): { label: string; tone: "ok" | "warn" } | null {
  switch (source) {
    case "takeout_json":
      return { label: t("details.fromJson"), tone: "ok" };
    case "exif":
      return { label: t("details.fromExif"), tone: "ok" };
    case "filesystem":
      return { label: t("details.fromFile"), tone: "warn" };
    default:
      return null;
  }
}

const degrees = numberFormat({ minimumFractionDigits: 4, maximumFractionDigits: 4 });

/** "38,7139° N · 9,1394° O"; `null` sem GPS. */
export function coordinates(latitude: number | null, longitude: number | null): string | null {
  if (latitude === null || longitude === null) {
    return null;
  }
  const north = `${degrees.format(Math.abs(latitude))}° ${latitude < 0 ? t("details.south") : t("details.north")}`;
  const east = `${degrees.format(Math.abs(longitude))}° ${longitude < 0 ? t("details.west") : t("details.east")}`;
  return `${north} · ${east}`;
}

export function matchLabel(match: MediaPanel["jsonMatch"]): string {
  switch (match) {
    case "exact":
      return t("details.matchExact");
    case "heuristic":
      return t("details.matchHeuristic");
    default:
      return t("details.none");
  }
}

/** As linhas de "Arquivo", na ordem do canvas. O álbum entra com a F25. */
export function fileRows(
  info: { filename: string; width: number | null; height: number | null; sizeBytes: number | null },
  panel: MediaPanel,
): [string, string][] {
  const rows: [string, string][] = [
    [t("details.name"), info.filename],
    [t("details.folder"), panel.folder],
  ];
  if (info.width !== null && info.height !== null) {
    rows.push([t("details.dimensions"), `${info.width} × ${info.height}`]);
  }
  if (info.sizeBytes !== null) {
    rows.push([t("details.size"), fileSize(info.sizeBytes)]);
  }
  if (panel.durationMs !== null) {
    rows.push([t("details.duration"), duration(panel.durationMs)]);
  }
  return rows;
}

/** As linhas de "Técnico". */
export function technicalRows(panel: MediaPanel): [string, string][] {
  return [
    [t("details.pairedJson"), panel.jsonFilename ?? t("details.none")],
    [t("details.matching"), matchLabel(panel.jsonMatch)],
    [t("details.camera"), panel.camera ?? t("details.cameraUnknown")],
    ["SHA-256", panel.sha256 ?? t("details.hashPending")],
  ];
}

/** O painel aberto ou fechado fica para a próxima foto e a próxima vez. */
export const DETAILS_KEY = "myphotos.viewer.details";
