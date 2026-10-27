import assert from "node:assert/strict";
import { test } from "node:test";
import { filterPeople, initials, itemsLabel, peopleSubtitle, personFromHash, personHash, type PersonCard } from "./people.ts";

const PEOPLE: PersonCard[] = [
  { id: 1, name: "Ana Lúcia Souza", count: 812, cover: 40 },
  { id: 2, name: "Bruno", count: 1, cover: null },
  { id: 3, name: "Lucas", count: 3, cover: 7 },
];

test("PPLUI-01 the filter finds names without accents, case or word order", () => {
  assert.deepEqual(filterPeople(PEOPLE, "  ").map((p) => p.id), [1, 2, 3]);
  assert.deepEqual(filterPeople(PEOPLE, "lucia").map((p) => p.id), [1]);
  assert.deepEqual(filterPeople(PEOPLE, "souza ana").map((p) => p.id), [1]);
  assert.deepEqual(filterPeople(PEOPLE, "luc").map((p) => p.id), [1, 3]);
  assert.deepEqual(filterPeople(PEOPLE, "zé"), []);
});

test("PPLUI-02 texts read like the canvas and a person opens the search", () => {
  assert.equal(peopleSubtitle(28), "28 pessoas marcadas no Google Fotos, lidas dos JSONs do Takeout");
  assert.equal(peopleSubtitle(1), "1 pessoa marcada no Google Fotos, lida dos JSONs do Takeout");
  assert.equal(peopleSubtitle(0), "Ninguém marcado no Google Fotos");
  assert.equal(itemsLabel(812), "812 itens");
  assert.equal(itemsLabel(1234), "1.234 itens");
  assert.equal(itemsLabel(1), "1 item");
  assert.equal(personHash(7), "#/buscar?pessoa=7");
  assert.equal(personFromHash("#/buscar?pessoa=7"), 7);
  assert.equal(personFromHash("#/buscar"), null);
  assert.equal(personFromHash("#/buscar/7"), null);
  assert.equal(initials("Ana Lúcia Souza"), "AS");
  assert.equal(initials("bruno"), "B");
});
