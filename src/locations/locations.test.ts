import assert from "node:assert/strict";
import { test } from "node:test";
import {
  fitView,
  locationsSubtitle,
  period,
  pinLabel,
  pins,
  pinSize,
  placeFromHash,
  placeHash,
  placeMeta,
  placeName,
  type Place,
} from "./locations.ts";

const place = (label: string, count: number, latitude: number, longitude: number, first: string | null = null, last = first): Place => ({
  label,
  count,
  latitude,
  longitude,
  first,
  last,
  cover: null,
});

const SAO_PAULO = place("São Paulo, SP, Brasil", 1812, -23.55, -46.63, "2015-03-01T10:00:00Z", "2024-05-02T10:00:00Z");
const LISBOA = place("Lisbon, Portugal", 124, 38.71, -9.14, "2022-08-10T10:00:00Z", "2022-08-14T10:00:00Z");
const SINTRA = place("Sintra, Portugal", 44, 38.8, -9.38, "2022-08-12T10:00:00Z", "2022-08-12T10:00:00Z");

test("LOCUI-01 texts read like the canvas", () => {
  assert.equal(locationsSubtitle(3402, 845), "3.402 itens com coordenadas · 845 sem localização");
  assert.equal(locationsSubtitle(1, 0), "1 item com coordenadas · 0 sem localização");
  assert.equal(placeName(SAO_PAULO.label), "São Paulo");
  assert.equal(placeMeta(SAO_PAULO), "SP, Brasil · 2015–2024");
  assert.equal(placeMeta(LISBOA), "Portugal · ago 2022");
  assert.equal(period("2021-01-03T00:00:00Z", "2021-11-03T00:00:00Z"), "2021");
  assert.equal(placeMeta(place("Porto, Portugal", 3, 41, -8)), "Portugal", "no dates, no period");
  assert.equal(pinLabel(640), "640");
  assert.equal(pinLabel(1812), "1,8 mil");
  assert.equal(pinLabel(12400), "12 mil");
  assert.ok(pinSize(1) === 36 && pinSize(5000) === 64 && pinSize(124) > 36);
});

test("LOCUI-02 near places share a pin until zoomed, and the view fits them all", () => {
  // Lisboa e Sintra ficam a ~22 km: juntos no mundo inteiro, separados de perto.
  const far = pins([LISBOA, SAO_PAULO, SINTRA], 4);
  assert.equal(far.length, 2);
  assert.deepEqual(far.find((pin) => pin.places[0] === LISBOA)?.places, [LISBOA, SINTRA]);
  assert.equal(far.find((pin) => pin.places[0] === LISBOA)?.count, 168);
  assert.equal(pins([LISBOA, SAO_PAULO, SINTRA], 400).length, 3);

  const view = fitView([SAO_PAULO, LISBOA], 800, 600);
  assert.ok(view.x > 180 - 47 && view.x < 180 - 9, "between both longitudes");
  const lonSpan = 46.63 - 9.14;
  assert.ok(view.scale * lonSpan <= 800 * 0.8 + 1);
  assert.ok(fitView([LISBOA], 800, 600).scale <= (600 * 0.8) / 6 + 1e-9, "one place does not zoom to the street");
  assert.deepEqual(fitView([], 720, 360), { x: 180, y: 90, scale: 2 }, "no place: the whole world");
});

test("LOCUI-03 a place opens the search", () => {
  const hash = placeHash(SAO_PAULO.label);
  assert.equal(hash, "#/buscar?local=S%C3%A3o%20Paulo%2C%20SP%2C%20Brasil");
  assert.equal(placeFromHash(hash), SAO_PAULO.label);
  assert.equal(placeFromHash("#/buscar"), null);
  assert.equal(placeFromHash("#/buscar?local=%E0%A4%A"), null, "broken escape");
});
