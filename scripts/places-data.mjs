// F24: gera os dados de lugares embutidos no app (sem rede; AD-007/RNF-005).
//
//   npm i --no-save all-the-cities@3.1.0 world-atlas@2.0.2 topojson-client@3.1.0
//   node scripts/places-data.mjs
//
// - src-tauri/data/places.tsv: cidades com 5 mil habitantes ou mais (GeoNames, via
//   all-the-cities), sem bairros (PPLX): nome, país, estado, latitude, longitude.
// - src-tauri/data/countries.tsv: código do país → nome em português (Intl).
// - src/locations/world.ts: contorno dos continentes (Natural Earth 1:110m, via
//   world-atlas) em projeção equiretangular, 360 × 180.
//
// Dados de cidades: GeoNames (https://www.geonames.org), CC BY 4.0.
// Contornos: Natural Earth (https://www.naturalearthdata.com), domínio público.

import { writeFileSync } from "node:fs";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const cities = require("all-the-cities");
const land = require("world-atlas/land-110m.json");
const { feature } = require("topojson-client");

const MIN_POPULATION = 5000;
const clean = (text) => String(text).replace(/[\t\r\n]/g, " ").trim();

const rows = cities
  .filter((city) => city.population >= MIN_POPULATION && city.featureCode !== "PPLX")
  .sort((a, b) => b.population - a.population)
  .map((city) => {
    const [lon, lat] = city.loc.coordinates;
    return [clean(city.name), city.country, clean(city.adminCode ?? ""), lat.toFixed(4), lon.toFixed(4)].join("\t");
  });
writeFileSync(
  new URL("../src-tauri/data/places.tsv", import.meta.url),
  `# GeoNames (geonames.org), CC BY 4.0; gerado por scripts/places-data.mjs\n${rows.join("\n")}\n`,
);

const names = new Intl.DisplayNames(["pt-BR"], { type: "region" });
const codes = [...new Set(cities.map((city) => city.country))].filter((code) => /^[A-Z]{2}$/.test(code)).sort();
writeFileSync(
  new URL("../src-tauri/data/countries.tsv", import.meta.url),
  `${codes.map((code) => `${code}\t${names.of(code)}`).join("\n")}\n`,
);

// Um path SVG: x = longitude + 180, y = 90 − latitude, uma casa decimal.
const shapes = feature(land, land.objects.land).features.flatMap((shape) =>
  shape.geometry.type === "Polygon" ? [shape.geometry.coordinates] : shape.geometry.coordinates,
);
// Um anel que cruza o antimeridiano (Fiji, Rússia) recomeça do outro lado em vez de
// riscar o mapa de ponta a ponta.
const ringPath = (ring) =>
  ring
    .map(([lon, lat], index) => {
      const jump = index > 0 && Math.abs(lon - ring[index - 1][0]) > 180;
      return `${index === 0 || jump ? "M" : "L"}${(lon + 180).toFixed(1)} ${(90 - lat).toFixed(1)}`;
    })
    .join("");
const path = shapes.flatMap((polygon) => polygon.map(ringPath)).join("");
writeFileSync(
  new URL("../src/locations/world.ts", import.meta.url),
  `// Gerado por scripts/places-data.mjs. Natural Earth 1:110m (domínio público), via world-atlas.\n` +
    `// Projeção equiretangular: x = longitude + 180, y = 90 − latitude (360 × 180).\n` +
    `export const WORLD_PATH =\n  "${path}";\n`,
);
console.log(`${rows.length} cidades, ${codes.length} países, mapa com ${path.length} caracteres`);
