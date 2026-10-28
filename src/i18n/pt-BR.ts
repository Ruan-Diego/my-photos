// F34: textos em português do Brasil, o idioma padrão e a referência das outras traduções.
// Plural: `{ one, other }`, `one` só para exatamente 1 ("0 itens", como o app sempre disse);
// `{n}` é o número formatado. Uma língua com mais formas de plural amplia isto.

export const ptBR = {
  "calendar.months": "Janeiro|Fevereiro|Março|Abril|Maio|Junho|Julho|Agosto|Setembro|Outubro|Novembro|Dezembro",
  "calendar.weekdays": "Dom|Seg|Ter|Qua|Qui|Sex|Sáb",

  "count.items": { one: "{n} item", other: "{n} itens" },
  "count.media": { one: "{n} mídia", other: "{n} mídias" },
  "count.results": { one: "{n} resultado", other: "{n} resultados" },
  "count.thumbnailsLeft": { one: "falta {n} miniatura", other: "faltam {n} miniaturas" },

  "section.timeline": "Fotos",
  "section.folders": "Pastas",
  "section.albums": "Álbuns",
  "section.people": "Pessoas",
  "section.locations": "Locais",
  "section.favorites": "Favoritos",
  "section.duplicates": "Duplicatas",
  "section.problems": "Problemas",
  "section.trash": "Lixeira",
  "section.search": "Buscar",
  "section.settings": "Configurações",

  "language.name": "Português (Brasil)",
} as const;

export type Key = keyof typeof ptBR;
export type Message = string | { one: string; other: string };
/** Outra língua: as mesmas chaves, cada uma com a sua tradução. */
export type Messages = Record<Key, Message>;
