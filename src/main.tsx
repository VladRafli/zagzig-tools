import React from "react";
import ReactDOM from "react-dom/client";
import { ThemeProvider } from "next-themes";
import App from "./App";
import { TooltipProvider } from "./components/ui/tooltip";
import { Toaster } from "./components/ui/sonner";
import { loadLanguagePacks } from "./i18n";
import "./index.css";

// Suppress the webview's browser-style context menu (Back / Reload / Inspect…),
// which makes the app feel like a web page. It stays for editable fields and
// for selected text, where Copy / Paste are genuinely useful.
document.addEventListener("contextmenu", (e) => {
  const target = e.target as HTMLElement | null;
  if (target?.closest("input, textarea, [contenteditable='true']")) return;
  if (window.getSelection()?.toString()) return;
  e.preventDefault();
});

// Custom languages live in the app's data folder; load them in the background.
// Until they arrive the app shows English, then switches to the saved language.
void loadLanguagePacks();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ThemeProvider attribute="class" defaultTheme="system" enableSystem>
      <TooltipProvider>
        <App />
        <Toaster />
      </TooltipProvider>
    </ThemeProvider>
  </React.StrictMode>,
);
