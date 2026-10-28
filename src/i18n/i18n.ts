// F34: o idioma do app. Hoje só pt-BR; outro idioma é um arquivo com as mesmas chaves
// (`Messages`) registrado em `LOCALES`. Sem React: testável no Node.
import { ptBR, type Key, type Message, type Messages } from "./pt-BR.ts";

export type { Key, Messages };

export const LOCALES: Record<string, Messages> = { "pt-BR": ptBR };
export const DEFAULT_LOCALE = "pt-BR";

let locale = DEFAULT_LOCALE;
let messages: Messages = ptBR;
let numbers = new Intl.NumberFormat(locale);

/** Troca o idioma (os componentes leem o novo na próxima renderização). */
export function setLocale(next: string) {
  const found = LOCALES[next];
  if (found === undefined) {
    return;
  }
  locale = next;
  messages = found;
  numbers = new Intl.NumberFormat(locale);
}

export function currentLocale(): string {
  return locale;
}

/** "12.345" */
export function formatCount(value: number): string {
  return numbers.format(value);
}

export function numberFormat(options: Intl.NumberFormatOptions): Intl.NumberFormat {
  return new Intl.NumberFormat(locale, options);
}

export function collator(options: Intl.CollatorOptions): Intl.Collator {
  return new Intl.Collator(locale, options);
}

export function dateText(date: Date, options: Intl.DateTimeFormatOptions): string {
  return date.toLocaleString(locale, options);
}

/** `t("count.items", { n: 3 })` → "3 itens"; `{nome}` vira o valor de `nome`. */
export function t(key: Key, params: Record<string, string | number> = {}): string {
  const message: Message = messages[key] ?? ptBR[key];
  const text = typeof message === "string" ? message : message[params.n === 1 ? "one" : "other"];
  return text.replace(/\{(\w+)\}/g, (whole, name: string) => {
    const value = params[name];
    if (value === undefined) {
      return whole;
    }
    return typeof value === "number" ? numbers.format(value) : value;
  });
}

/** Para chaves montadas em tempo de execução (um código do Rust): `undefined` se não houver texto. */
export function maybe(key: string, params: Record<string, string | number> = {}): string | undefined {
  return key in ptBR ? t(key as Key, params) : undefined;
}

/** Listas separadas por `|` (meses, dias da semana). */
export function list(key: Key): string[] {
  return t(key).split("|");
}

/** Os `{nomes}` de uma mensagem, para conferir que toda tradução tem os mesmos. */
export function placeholders(message: Message): string[] {
  const texts = typeof message === "string" ? [message] : [message.one, message.other];
  return [...new Set(texts.flatMap((text) => [...text.matchAll(/\{(\w+)\}/g)].map((match) => match[1])))].sort();
}
