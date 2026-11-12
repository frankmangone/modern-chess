import { createEffect } from "solid-js";

const SITE = "BoardHub";

/** Names the browser tab after the page: "Chess · BoardHub", or just the site's name while the
 *  page has none. Without it every page is called the same, in the tab, history and screen readers. */
export function usePageTitle(name: () => string | undefined) {
  createEffect(() => {
    const page = name();
    document.title = page ? `${page} · ${SITE}` : SITE;
  });
}
