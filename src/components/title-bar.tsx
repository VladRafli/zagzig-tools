import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, Minus, Square, X } from "lucide-react";
import { useTranslation } from "react-i18next";

import { cn } from "@/lib/utils";

const appWindow = getCurrentWindow();

// Native window decorations are off (see tauri.conf.json) so the app can
// draw its own title bar instead — this is that bar. `data-tauri-drag-region`
// is only on the title/spacer section, deliberately not wrapping the button
// group, so clicks on the buttons register as clicks rather than starting a
// window drag.
export function TitleBar() {
  const { t } = useTranslation();
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    let cancelled = false;

    appWindow.isMaximized().then((value) => {
      if (!cancelled) setIsMaximized(value);
    });

    const unlisten = appWindow.onResized(() => {
      appWindow.isMaximized().then((value) => {
        if (!cancelled) setIsMaximized(value);
      });
    });

    return () => {
      cancelled = true;
      unlisten.then((fn) => fn());
    };
  }, []);

  return (
    <div className="flex h-9 shrink-0 items-stretch border-b bg-sidebar text-sidebar-foreground">
      <div
        data-tauri-drag-region
        className="flex flex-1 items-center px-3 text-xs font-medium text-muted-foreground select-none"
        onDoubleClick={() => appWindow.toggleMaximize()}
      >
        {t("app.title")}
      </div>
      <div className="flex items-stretch">
        <TitleBarButton
          label={t("app.titleBar.minimize")}
          onClick={() => appWindow.minimize()}
        >
          <Minus className="size-3.5" />
        </TitleBarButton>
        <TitleBarButton
          label={isMaximized ? t("app.titleBar.restore") : t("app.titleBar.maximize")}
          onClick={() => appWindow.toggleMaximize()}
        >
          {isMaximized ? <Copy className="size-3" /> : <Square className="size-3" />}
        </TitleBarButton>
        <TitleBarButton
          label={t("app.titleBar.close")}
          onClick={() => appWindow.close()}
          destructive
        >
          <X className="size-3.5" />
        </TitleBarButton>
      </div>
    </div>
  );
}

function TitleBarButton({
  children,
  onClick,
  label,
  destructive,
}: {
  children: React.ReactNode;
  onClick: () => void;
  label: string;
  destructive?: boolean;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      className={cn(
        "flex w-11 items-center justify-center text-muted-foreground transition-colors hover:bg-muted hover:text-foreground",
        destructive && "hover:bg-destructive hover:text-white",
      )}
    >
      {children}
    </button>
  );
}
