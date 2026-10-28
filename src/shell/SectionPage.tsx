import type { ReactNode } from "react";
import type { Section } from "./sections";
import { t } from "../i18n/i18n";

/** Cabeçalho das telas (26 px/700) e o corpo de cada seção. */
export function SectionPage({ section, children }: { section: Section; children?: ReactNode }) {
  return (
    <main className="page" aria-labelledby="page-title">
      <header className="page-header">
        <h1 id="page-title">{section.label}</h1>
      </header>
      <div className="page-body">
        {children ?? (
          <div className="page-empty">
            <p className="page-empty-title">{t("page.buildingTitle")}</p>
            <p className="page-empty-detail">{t("page.buildingDetail", { feature: section.feature })}</p>
          </div>
        )}
      </div>
    </main>
  );
}
