import { motion } from "motion/react";
import { hashFor, SECTIONS, type Section, type SectionId } from "./sections";
import type { DiskCard } from "./library";
import { t } from "../i18n/i18n";

function Icon({ d }: { d: string }) {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d={d} />
    </svg>
  );
}

function Item({ section, active }: { section: Section; active: boolean }) {
  return (
    <a className="side-item" href={hashFor(section.id)} aria-current={active ? "page" : undefined}>
      {active ? (
        // O destaque desliza entre os itens: continuidade, não enfeite (AD-006).
        <motion.span className="side-item-bg" layoutId="side-active" transition={{ type: "spring", stiffness: 520, damping: 42 }} />
      ) : null}
      <Icon d={section.icon} />
      <span className="side-item-label">{section.label}</span>
    </a>
  );
}

export function Logo({ size = 26 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 28 28" fill="none" aria-hidden="true">
      <rect x="2" y="6" width="18" height="18" rx="4" stroke="currentColor" strokeWidth="2" />
      <rect x="8" y="2" width="18" height="18" rx="4" className="logo-fill" />
    </svg>
  );
}

export function Sidebar({
  active,
  disk,
  onDiskAction,
  update = null,
}: {
  active: SectionId;
  disk: DiskCard | null;
  /** Quando dado, o link do cartão vira botão (tentar a indexação de novo). */
  onDiskAction?: () => void;
  /** F37: versão nova achada ao abrir, com a busca de atualizações ligada. */
  update?: string | null;
}) {
  const main = SECTIONS.filter((section) => section.group === "main");
  const care = SECTIONS.filter((section) => section.group === "care");
  return (
    <nav className="side" aria-label={t("sidebar.label")}>
      <a className="side-logo" href={hashFor("timeline")}>
        <Logo />
        My Photos
      </a>

      <div className="side-group">
        {main.map((section) => (
          <Item key={section.id} section={section} active={section.id === active} />
        ))}
      </div>

      <div className="side-group">
        <span className="side-heading">{t("sidebar.care")}</span>
        {care.map((section) => (
          <Item key={section.id} section={section} active={section.id === active} />
        ))}
      </div>

      {update !== null ? (
        <div className="side-card" role="status">
          <div className="side-card-title">
            <span className="side-dot side-dot-ok" aria-hidden="true" />
            {t("settings.updates.available", { version: update })}
          </div>
          <a className="side-card-action" href={hashFor("settings")}>
            {t("settings.updates.install")}
          </a>
        </div>
      ) : null}

      {disk !== null ? (
        <div className="side-card" role="status">
          <div className="side-card-title">
            <span className={`side-dot side-dot-${disk.tone}`} aria-hidden="true" />
            {disk.title}
          </div>
          <span className="side-card-detail">{disk.detail}</span>
          {disk.action === null ? null : onDiskAction ? (
            <button type="button" className="side-card-action" onClick={onDiskAction}>
              {disk.action}
            </button>
          ) : (
            // Disco desconectado (F11) ainda não tem tela: o texto já é o do canvas.
            <span className="side-card-action">{disk.action}</span>
          )}
        </div>
      ) : null}
    </nav>
  );
}
