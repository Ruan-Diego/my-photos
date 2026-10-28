import { Fragment, type ReactNode } from "react";

/** F34: texto do catálogo com pedaços de React no meio: `"Fica em {path}."` + `{ path: <code>…</code> }`. */
export function rich(text: string, nodes: Record<string, ReactNode>): ReactNode {
  return text.split(/(\{\w+\})/).map((part, index) => {
    const name = /^\{(\w+)\}$/.exec(part)?.[1];
    return <Fragment key={index}>{name !== undefined && name in nodes ? nodes[name] : part}</Fragment>;
  });
}
