import { useRef, type FormEvent, type ReactNode } from "react";
import { useDialogFocus } from "./useDialogFocus";

/** F33: diálogo sobre a tela (não modal): recebe o foco, fecha com Esc e devolve o foco. */
export function Dialog({
  label,
  className,
  onEscape,
  onSubmit,
  children,
}: {
  label: string;
  className: string;
  onEscape: () => void;
  onSubmit?: (event: FormEvent) => void;
  children: ReactNode;
}) {
  const ref = useRef<HTMLElement>(null);
  useDialogFocus(ref, { onEscape });
  return onSubmit === undefined ? (
    <div ref={(element) => void (ref.current = element)} role="dialog" aria-label={label} className={className}>
      {children}
    </div>
  ) : (
    <form ref={(element) => void (ref.current = element)} role="dialog" aria-label={label} className={className} onSubmit={onSubmit}>
      {children}
    </form>
  );
}
