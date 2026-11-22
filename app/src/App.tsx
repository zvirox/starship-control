import React, { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { CockpitView } from "./components/cockpit/CockpitView";
import { HudView } from "./components/hud/HudView";

export const App: React.FC = () => {
  const [windowLabel, setWindowLabel] = useState<string>("main");
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function detectWindow() {
      // Check URL query param first (fallback for browser testing / dev)
      const params = new URLSearchParams(window.location.search);
      const queryWindow = params.get("window");
      if (queryWindow) {
        setWindowLabel(queryWindow);
        setLoading(false);
        return;
      }

      try {
        const win = getCurrentWindow();
        setWindowLabel(win.label);
      } catch (e) {
        console.warn("Could not determine window label via Tauri API, falling back to 'main':", e);
        setWindowLabel("main");
      } finally {
        setLoading(false);
      }
    }

    detectWindow();
  }, []);

  if (loading) {
    return <div className="h-screen w-screen bg-transparent" />;
  }

  if (windowLabel === "hud") {
    return <HudView />;
  }

  return <CockpitView />;
};

export default App;
