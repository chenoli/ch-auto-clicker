import { useEffect, useState } from "react";

import { useTranslation } from "react-i18next";

import { DEFAULT_LOCALE, applyLocale, isLocale } from "../../i18n";

import { THEMES } from "../../styles/tokens";

import { SCREENS } from "./screens";

import type { Locale } from "../../i18n";
import type { ThemeId } from "../../styles/tokens";
import type { ReactElement } from "react";

export interface GalleryRoute {
  screen: string | null;
  theme: ThemeId;
  lang: Locale;
}

export function parseGalleryHash(hash: string): GalleryRoute {
  const [path = "", query = ""] = hash.replace(/^#\/?/, "").split("?");
  const params = new URLSearchParams(query);
  const theme = params.get("theme");
  const lang = params.get("lang");
  const known = THEMES.find((t) => t.id === theme);
  return { screen: path || null, theme: known ? known.id : "grafite", lang: isLocale(lang) ? lang : DEFAULT_LOCALE };
}

export function Gallery(): ReactElement {
  const { t } = useTranslation();
  const [route, setRoute] = useState(() => parseGalleryHash(window.location.hash));

  useEffect(() => {
    const onChange = () => setRoute(parseGalleryHash(window.location.hash));
    window.addEventListener("hashchange", onChange);
    return () => window.removeEventListener("hashchange", onChange);
  }, []);

  useEffect(() => {
    document.documentElement.dataset.theme = route.theme;
  }, [route.theme]);

  useEffect(() => applyLocale(route.lang), [route.lang]);

  const visible = route.screen ? SCREENS.filter((s) => s.id === route.screen) : SCREENS;

  return (
    <div className="min-h-screen p-10">
      {!route.screen && (
        <h1 className="mt-0 mb-8 font-display text-28 font-extrabold tracking-display">{t("gallery.title")}</h1>
      )}
      <div className="flex flex-col gap-12">
        {visible.map((screen) => (
          <figure key={screen.id} className="m-0 flex flex-col gap-3">
            {!route.screen && (
              <figcaption className="text-10 font-bold tracking-label text-fg-3 uppercase">
                <a href={`#/${screen.id}?theme=${route.theme}&lang=${route.lang}`}>{screen.title}</a>
              </figcaption>
            )}
            <div data-testid={`screen-${screen.id}`} style={{ width: screen.width, height: screen.height }}>
              {screen.render()}
            </div>
          </figure>
        ))}
      </div>
    </div>
  );
}
