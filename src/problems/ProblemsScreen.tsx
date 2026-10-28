import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  diagnosticLines,
  filterProblems,
  fullPath,
  reportCards,
  shortPath,
  STATUS_LABEL,
  statusFilters,
  TABS,
  type Overview,
  type StatusFilter,
  type Tab,
} from "./problems";
import { formatCount } from "../i18n/i18n";

/** Linhas na tabela; o relatório exportado tem todas. */
const ROW_LIMIT = 500;

function useOverview(): [Overview | null, string | null] {
  const [overview, setOverview] = useState<Overview | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  useEffect(() => {
    invoke<Overview>("problems_overview")
      .then(setOverview)
      .catch((error) => setFailure(String(error)));
  }, []);
  return [overview, failure];
}

function Items({ overview }: { overview: Overview }) {
  const [filter, setFilter] = useState<StatusFilter>("all");
  const rows = filterProblems(overview.problems, filter);
  const root = overview.diagnostics.rootPath;
  return (
    <section role="tabpanel" aria-label="Itens não reconhecidos" className="problems-panel">
      <div className="problems-chips">
        {statusFilters(overview.problems).map((chip) => (
          <button key={chip.id} type="button" className="problems-chip" aria-pressed={filter === chip.id} onClick={() => setFilter(chip.id)}>
            {chip.label}
          </button>
        ))}
      </div>
      {rows.length === 0 ? (
        <div className="page-empty">
          <p className="page-empty-title">Nada aqui</p>
          <p className="page-empty-detail">
            {overview.problems.length === 0 ? "A última indexação reconheceu todos os arquivos." : "Nenhum item com esse status."}
          </p>
        </div>
      ) : (
        <div className="problems-table">
          <table>
            <thead>
              <tr>
                <th scope="col">Caminho</th>
                <th scope="col">Motivo</th>
                <th scope="col">O que o app fez</th>
                <th scope="col">Status</th>
              </tr>
            </thead>
            <tbody>
              {rows.slice(0, ROW_LIMIT).map((row) => (
                <tr key={`${row.path}|${row.reason}`}>
                  <td className="problems-path" title={fullPath(root, row.path)}>
                    {shortPath(row.path)}
                  </td>
                  <td className="problems-reason">{row.reason}</td>
                  <td className="problems-action">{row.action}</td>
                  <td>
                    <span className={`problems-status problems-status-${row.status}`}>{STATUS_LABEL[row.status]}</span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {rows.length > ROW_LIMIT ? (
        <p className="problems-note">
          Mostrando {formatCount(ROW_LIMIT)} de {formatCount(rows.length)}. O relatório exportado tem todos.
        </p>
      ) : null}
    </section>
  );
}

function ReportPanel({ overview }: { overview: Overview }) {
  return (
    <section role="tabpanel" aria-label="Relatório da indexação" className="problems-panel problems-report">
      {reportCards(overview.report).map((card) => (
        <div key={card.label} className="problems-card">
          <span>{card.label}</span>
          <strong>{card.value}</strong>
        </div>
      ))}
    </section>
  );
}

function DiagnosticsPanel({ overview }: { overview: Overview }) {
  // `undefined`: ainda não verificado; `null`: íntegro.
  const [integrity, setIntegrity] = useState<string | null | undefined>(undefined);
  const [checking, setChecking] = useState(false);
  const check = () => {
    setChecking(true);
    invoke<string | null>("database_check")
      .then(setIntegrity)
      .catch((error) => setIntegrity(String(error)))
      .finally(() => setChecking(false));
  };
  return (
    <section role="tabpanel" aria-label="Diagnóstico" className="problems-panel problems-diagnostics">
      {diagnosticLines(overview.diagnostics, integrity).map((line) => (
        <div key={line.label} className="problems-line">
          <span className={`problems-dot problems-dot-${line.tone}`} aria-hidden="true" />
          <span className="problems-line-label">{line.label}</span>
          <span className="problems-line-value">{line.value}</span>
        </div>
      ))}
      <button type="button" className="problems-button" onClick={check} disabled={checking}>
        {checking ? "Verificando…" : "Verificar integridade do banco"}
      </button>
    </section>
  );
}

/** F28: o que a indexação não reconheceu, o relatório e o diagnóstico (artboard `Problems`). */
export function ProblemsScreen() {
  const [overview, failure] = useOverview();
  const [tab, setTab] = useState<Tab>("items");
  const [exported, setExported] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);
  const exportReport = () => {
    setExporting(true);
    invoke<string>("problems_export")
      .then((path) => setExported(`Relatório salvo em ${path}`))
      .catch((error) => setExported(`Não foi possível salvar o relatório: ${String(error)}`))
      .finally(() => setExporting(false));
  };

  return (
    <main className="page problems" aria-labelledby="problems-title">
      <header className="page-header problems-header">
        <div className="problems-top">
          <div className="people-heading">
            <h1 id="problems-title">Problemas</h1>
            <span>Nenhum deles parou a indexação. Arquivos .zip e .html do Takeout são ignorados e não aparecem aqui.</span>
          </div>
          <button type="button" className="problems-button" onClick={exportReport} disabled={overview === null || exporting}>
            Exportar relatório
          </button>
        </div>
        {exported !== null ? (
          <p className="problems-exported" role="status">
            {exported}
          </p>
        ) : null}
        <div role="tablist" aria-label="Seções" className="problems-tabs">
          {TABS.map((option) => (
            <button key={option.id} type="button" role="tab" aria-selected={tab === option.id} onClick={() => setTab(option.id)}>
              {option.label}
            </button>
          ))}
        </div>
      </header>
      <div className="page-body">
        {failure !== null ? (
          <div className="page-empty">
            <p className="page-empty-title">Não foi possível ler os problemas</p>
            <p className="page-empty-detail">{failure}</p>
          </div>
        ) : overview === null ? null : tab === "items" ? (
          <Items overview={overview} />
        ) : tab === "report" ? (
          <ReportPanel overview={overview} />
        ) : (
          <DiagnosticsPanel overview={overview} />
        )}
      </div>
    </main>
  );
}
