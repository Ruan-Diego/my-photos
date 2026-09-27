import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { healthView, type AppHealth, type HealthScreen } from "./health/view";
import { linkedMotionLibraries } from "./motion/libraries";
import "./App.css";

function errorText(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

function App() {
  const [screen, setScreen] = useState<HealthScreen | null>(null);

  useEffect(() => {
    let active = true;

    async function load() {
      try {
        linkedMotionLibraries();
        const health = await invoke<AppHealth>("app_health");
        if (active) {
          setScreen(healthView(health, null));
        }
      } catch (error) {
        if (active) {
          setScreen(healthView(null, errorText(error)));
        }
      }
    }

    void load();
    return () => {
      active = false;
    };
  }, []);

  if (screen === null) {
    return null;
  }

  return (
    <main>
      {screen.failure !== null ? <p>{screen.failure}</p> : null}
      {screen.ready !== null ? <h1>{screen.ready}</h1> : null}
      {screen.sqlite !== null ? <p>{screen.sqlite}</p> : null}
      {screen.libraries !== null ? <p>{screen.libraries}</p> : null}
    </main>
  );
}

export default App;
