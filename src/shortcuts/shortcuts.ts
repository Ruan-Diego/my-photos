/** Atalhos de teclado e mouse (F30, seção 15.4 do PRD). */

/** Foco numa caixa de texto: a tecla é do texto, não do atalho. */
export function typing(target: EventTarget | null): boolean {
  if (typeof HTMLElement === "undefined" || !(target instanceof HTMLElement)) {
    return false;
  }
  const tag = target.tagName;
  if (tag === "TEXTAREA" || tag === "SELECT" || target.isContentEditable) {
    return true;
  }
  if (tag !== "INPUT") {
    return false;
  }
  const type = (target as HTMLInputElement).type;
  return type !== "range" && type !== "checkbox" && type !== "radio" && type !== "button";
}

export const MIN_SIZE = 1;
export const MAX_SIZE = 5;

/** Ctrl + roda: para cima aumenta as miniaturas, para baixo diminui, um passo por vez. */
export function wheelSize(size: number, deltaY: number): number {
  if (deltaY === 0) {
    return size;
  }
  return Math.min(MAX_SIZE, Math.max(MIN_SIZE, size + (deltaY < 0 ? 1 : -1)));
}

/** Intervalo mínimo entre passos de Ctrl + roda, para um touchpad não pular do 1 ao 5. */
export const WHEEL_STEP_MS = 120;

export type ViewerAction = "previous" | "next" | "back" | "fullscreen";

/** Teclas da foto aberta: ← → andam, Esc volta (ou só sai da tela cheia), F alterna a tela cheia. */
export function viewerAction(key: string, modified: boolean, fullscreen: boolean): ViewerAction | null {
  if (modified) {
    return null;
  }
  switch (key) {
    case "ArrowLeft":
      return "previous";
    case "ArrowRight":
      return "next";
    case "Escape":
      return fullscreen ? null : "back";
    case "f":
    case "F":
      return "fullscreen";
    default:
      return null;
  }
}
