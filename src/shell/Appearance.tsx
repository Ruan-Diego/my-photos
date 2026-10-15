import { THEME_OPTIONS, type ThemeChoice } from "../theme/theme";

/** Configurações › Aparência › Tema (artboard `Settings`). O resto da tela é a F31. */
export function Appearance({ theme, onChange }: { theme: ThemeChoice; onChange: (choice: ThemeChoice) => void }) {
  return (
    <section className="card" aria-labelledby="appearance-title">
      <h2 id="appearance-title">Aparência</h2>
      <fieldset className="choice-row">
        <legend>Tema</legend>
        {THEME_OPTIONS.map((option) => (
          <label key={option.choice} className="choice" data-on={option.choice === theme ? "" : undefined}>
            <input
              type="radio"
              name="theme"
              checked={option.choice === theme}
              onChange={() => onChange(option.choice)}
            />
            {option.label}
          </label>
        ))}
      </fieldset>
    </section>
  );
}
