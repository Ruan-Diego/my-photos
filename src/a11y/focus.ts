// F33: foco em diálogos e janelas por cima da tela. Sem React: testável no Node.

export const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), video[controls], [tabindex]:not([tabindex="-1"])';

/**
 * Para onde o Tab vai dentro de um diálogo modal: do último para o primeiro e, com Shift,
 * do primeiro para o último. `null` deixa o navegador seguir (ou não há nada focável).
 * `index` é -1 quando o foco está fora dos focáveis do diálogo.
 */
export function wrapTarget(index: number, count: number, backwards: boolean): number | null {
  if (count === 0) {
    return null;
  }
  if (index === -1) {
    return backwards ? count - 1 : 0;
  }
  if (backwards && index === 0) {
    return count - 1;
  }
  if (!backwards && index === count - 1) {
    return 0;
  }
  return null;
}
