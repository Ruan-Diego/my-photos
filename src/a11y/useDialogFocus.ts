import { useEffect, useState, type RefObject } from "react";
import { FOCUSABLE, wrapTarget } from "./focus";

/**
 * F33: ao abrir, o foco entra no diálogo (no elemento com `autofocus`, senão no primeiro
 * focável, senão no próprio diálogo); ao fechar, volta para onde estava. `trap` prende o Tab
 * (diálogos modais); `onEscape` fecha com Esc.
 */
export function useDialogFocus(
  ref: RefObject<HTMLElement | null>,
  { trap = false, onEscape }: { trap?: boolean; onEscape?: () => void } = {},
) {
  // Lido na primeira renderização: o `autoFocus` de um campo já moveu o foco quando o efeito roda.
  const [before] = useState(() => (document.activeElement instanceof HTMLElement ? document.activeElement : null));
  useEffect(() => {
    const dialog = ref.current;
    if (dialog === null) {
      return;
    }
    if (trap && !dialog.hasAttribute("tabindex")) {
      // Clicar numa área sem foco não devolve o foco para a tela de trás.
      dialog.tabIndex = -1;
    }
    if (!dialog.contains(document.activeElement)) {
      const first = dialog.querySelector<HTMLElement>("[autofocus]") ?? dialog.querySelector<HTMLElement>(FOCUSABLE);
      if (first !== null) {
        first.focus();
      } else {
        dialog.tabIndex = -1;
        dialog.focus();
      }
    }
    return () => {
      if (before !== null && before.isConnected) {
        before.focus({ preventScroll: true });
      }
    };
    // Só ao abrir e fechar.
  }, []);

  useEffect(() => {
    const dialog = ref.current;
    if (dialog === null || (!trap && onEscape === undefined)) {
      return;
    }
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && onEscape !== undefined) {
        event.preventDefault();
        onEscape();
        return;
      }
      if (event.key !== "Tab" || !trap) {
        return;
      }
      const items = [...dialog.querySelectorAll<HTMLElement>(FOCUSABLE)].filter((item) => item.offsetParent !== null || item === document.activeElement);
      const target = wrapTarget(items.indexOf(document.activeElement as HTMLElement), items.length, event.shiftKey);
      if (target !== null) {
        event.preventDefault();
        items[target].focus();
      }
    };
    dialog.addEventListener("keydown", onKey);
    return () => dialog.removeEventListener("keydown", onKey);
  }, [ref, trap, onEscape]);
}
