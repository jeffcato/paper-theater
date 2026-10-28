import React, { useCallback, useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { createTheaterWorld } from "@paper-theater/player";
import type { TheaterWorld } from "@paper-theater/player";
import "@paper-theater/player/fonts/noto-serif.css";
import { readiness, TICKETS, ticketState } from "./tickets.js";
import type { Catalog, TicketStatus } from "./tickets.js";
import "./styles.css";

// One full-viewport multiplane world (shared player) is the whole visible
// interface: box office, aisle and house are 2D planes in Z-space, and the
// playback controls are signs fixed to the viewport. The DOM below it is a
// visually hidden, focusable mirror of the same controls for keyboard and
// screen-reader users; focusing a proxy lights its counterpart in the world.
// No accepted package exists yet, so every seat shows an honest closed
// curtain and no diagnostic fixture is ever loaded on this path.

type CatalogStatus = "loading" | "ready" | "error";
type Route = { view: "booth" } | { view: "seat"; id: string };

async function fetchCatalog(): Promise<Catalog> {
  const res = await fetch("tickets/catalog.json", { cache: "no-store" });
  if (!res.ok) throw new Error(`catalog unavailable: ${res.status}`);
  const data = (await res.json()) as Catalog;
  if (!data || !Array.isArray(data.packages)) throw new Error("catalog malformed");
  return data;
}

function parseRoute(hash: string): Route {
  const match = hash.match(/^#\/theater\/([A-Za-z0-9-]+)\/?$/);
  return match && TICKETS.some((t) => t.id === match[1]) ? { view: "seat", id: match[1]! } : { view: "booth" };
}

function useHashRoute(): Route {
  const [route, setRoute] = useState(() => parseRoute(window.location.hash));
  useEffect(() => {
    const onChange = () => setRoute(parseRoute(window.location.hash));
    window.addEventListener("hashchange", onChange);
    return () => window.removeEventListener("hashchange", onChange);
  }, []);
  return route;
}

function useReducedMotion(): boolean {
  const [reduced, setReduced] = useState(() => window.matchMedia("(prefers-reduced-motion: reduce)").matches);
  useEffect(() => {
    const query = window.matchMedia("(prefers-reduced-motion: reduce)");
    const onChange = (e: MediaQueryListEvent) => setReduced(e.matches);
    query.addEventListener("change", onChange);
    return () => query.removeEventListener("change", onChange);
  }, []);
  return reduced;
}

/** Visually hidden button that lights its world counterpart on focus. */
function Proxy(props: {
  id: string;
  world: TheaterWorld | null;
  onActivate: (id: string) => void;
  disabled?: boolean;
  pressed?: boolean;
  children: React.ReactNode;
}): React.JSX.Element {
  const { id, world } = props;
  return (
    <button
      type="button"
      data-pick={id}
      aria-disabled={props.disabled || undefined}
      aria-pressed={props.pressed}
      onFocus={() => world?.sink.setHighlight(id)}
      onBlur={() => world?.sink.setHighlight(null)}
      onClick={() => props.onActivate(id)}
    >
      {props.children}
    </button>
  );
}

function App(): React.JSX.Element {
  const route = useHashRoute();
  const reducedMotion = useReducedMotion();
  const [catalog, setCatalog] = useState<Catalog>({ packages: [] });
  const [status, setStatus] = useState<CatalogStatus>("loading");
  const [inspecting, setInspecting] = useState<string | null>(null);
  const [punched, setPunched] = useState<string | null>(null);
  const [readAlong, setReadAlong] = useState(false);
  const [curtainOpen, setCurtainOpen] = useState(false);
  const [announce, setAnnounce] = useState("");
  const [world, setWorld] = useState<TheaterWorld | null>(null);
  const [graphics, setGraphics] = useState<"pending" | "ready" | "failed">("pending");
  const hostRef = useRef<HTMLDivElement | null>(null);
  const plaqueRef = useRef<HTMLDivElement | null>(null);
  const initialRoute = useRef(route);
  const activate = useRef<(id: string) => void>(() => {});

  const load = useCallback(() => {
    setStatus("loading");
    fetchCatalog().then(
      (next) => {
        setCatalog(next);
        setStatus("ready");
      },
      () => {
        setCatalog({ packages: [] });
        setStatus("error");
      },
    );
  }, []);
  useEffect(load, [load]);

  // The world: created once, disposed on unmount (StrictMode-safe).
  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;
    let cancelled = false;
    let created: TheaterWorld | null = null;

    async function mountWorld() {
      let houseArt = null;
      if (import.meta.env.DEV) {
        try {
          const { loadDevHouseArt } = await import("./dev-art.js");
          houseArt = await loadDevHouseArt();
        } catch {
          // Dev house art is optional; fall back to procedural cards.
        }
      }
      if (cancelled) return;

      const w = await createTheaterWorld({
        container: host!,
        plaque: plaqueRef.current,
        journey: initialRoute.current.view === "seat" ? 1 : 0,
        reducedMotion,
        houseArt,
        onPick: (pick) => activate.current(pick.id),
        onJourney: (t) => (document.documentElement.dataset.journey = t.toFixed(3)),
        onArrive: (t) => (document.documentElement.dataset.journey = String(t)),
      });

      if (cancelled) {
        w.dispose();
        return;
      }
      created = w;
      document.documentElement.dataset.journey = String(w.journey());
      setWorld(w);
      setGraphics("ready");
    }

    mountWorld().catch(() => !cancelled && setGraphics("failed"));

    return () => {
      cancelled = true;
      created?.dispose();
      setWorld(null);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const states = ticketState(catalog);
  const stateOf = (id: string): TicketStatus => (status === "ready" ? states[id] ?? "unavailable" : "unavailable");
  const seatTicket = route.view === "seat" ? TICKETS.find((t) => t.id === route.id) ?? null : null;
  const inspected = TICKETS.find((t) => t.id === inspecting) ?? null;

  useEffect(() => world?.setReducedMotion(reducedMotion), [world, reducedMotion]);
  useEffect(() => {
    if (!world) return;
    const notice = status === "error" ? "Program board unreachable — tap to retry" : status === "loading" ? "Checking today’s program…" : null;
    world.sink.setTickets(
      TICKETS.map((t) => ({ id: t.id, title: t.title, subtitle: t.subtitle, ready: stateOf(t.id) === "ready", punched: punched === t.id })),
      notice,
    );
  }, [world, status, catalog, punched]);
  useEffect(() => {
    if (!world) return;
    world.sink.setInspect(inspected ? {
      id: inspected.id,
      title: inspected.title,
      subtitle: inspected.subtitle,
      synopsis: inspected.synopsis,
      excerpt: inspected.excerpt,
      readiness: readiness(stateOf(inspected.id)).label,
      seatLabel: readiness(stateOf(inspected.id)).playable ? "Take Your Seat" : "",
    } : null);
  }, [world, inspected, status, catalog]);
  useEffect(() => {
    if (!world) return;
    const seated = route.view === "seat";
    world.sink.setControls({
      enabled: false,
      playing: seated && curtainOpen,
      readAlong,
      progress: 0,
      fullscreen: false,
      rehearsal: seated,
    });
    world.setRehearsalCurtain(seated && curtainOpen);
  }, [world, readAlong, route.view, curtainOpen]);
  useEffect(() => {
    if (route.view !== "seat") setCurtainOpen(false);
  }, [route.view]);
  useEffect(() => {
    if (!world) return;
    if (seatTicket) {
      const ready = stateOf(seatTicket.id) === "ready";
      world.sink.setTitle({ heading: seatTicket.title, sub: ready ? seatTicket.subtitle : "Under rehearsal — not yet published" });
      world.setIdleTitle(true);
      setInspecting(null);
      void world.travel(1);
    } else {
      world.setIdleTitle(false);
      void world.travel(0);
    }
  }, [world, route.view, seatTicket?.id, status, catalog]);

  // Keep keyboard focus inside the current view when proxies unmount.
  const lastInspected = useRef<string | null>(null);
  useEffect(() => {
    const focus = (id: string) =>
      document.querySelector<HTMLElement>(`[data-pick="${id}"]`)?.focus({ preventScroll: true });
    if (route.view === "seat") focus("rail:play");
    else if (inspecting) focus("inspect:seat");
    else if (lastInspected.current && document.activeElement === document.body) focus(`ticket:${lastInspected.current}`);
    lastInspected.current = inspecting;
  }, [route.view, inspecting]);

  activate.current = (id: string) => {
    if (id.startsWith("ticket:")) {
      const ticket = TICKETS.find((t) => `ticket:${t.id}` === id);
      if (!ticket) return;
      setInspecting(ticket.id);
      setAnnounce(`${ticket.title}. ${readiness(stateOf(ticket.id)).label}.`);
    } else if (id === "inspect:back") {
      setInspecting(null);
    } else if (id === "inspect:seat" && inspected) {
      setPunched(inspected.id);
      setAnnounce("Ticket punched. Walking down the aisle to your seat.");
      window.location.hash = inspected.href;
    } else if (id === "notice") {
      load();
    } else if (id === "rail:return") {
      setAnnounce("Returning to the box office.");
      window.location.hash = "#/";
    } else if (id === "rail:readalong") {
      setReadAlong((v) => !v);
    } else if (id === "rail:play") {
      setCurtainOpen((open) => {
        const next = !open;
        setAnnounce(next
          ? "Curtain opening on the empty stage. This performance is not yet published."
          : "Curtain closing. This performance is not yet published.");
        return next;
      });
    } else if (id.startsWith("rail:")) {
      setAnnounce("This performance is not yet published.");
    }
  };
  const act = (id: string) => activate.current(id);

  if (graphics === "failed") {
    return (
      <main className="fallback">
        <h1>Paper Theater</h1>
        <p role="alert">This browser cannot draw the theater (WebGL is unavailable). The program is listed below.</p>
        <ul>
          {TICKETS.map((t) => (
            <li key={t.id}><cite>{t.title}</cite> — {t.subtitle}: {readiness(stateOf(t.id)).label}.</li>
          ))}
        </ul>
      </main>
    );
  }

  return (
    <>
      <div ref={hostRef} className="world" aria-hidden="true">
        <div ref={plaqueRef} className="plaque" hidden />
      </div>
      <main className="a11y">
        <p role="status" aria-live="polite">{announce}</p>
        {route.view === "booth" ? (
          <section aria-label="Box office">
            <h1>Paper Theater box office</h1>
            {status === "loading" && <p>Checking today’s program…</p>}
            {status === "error" && <Proxy id="notice" world={world} onActivate={act}>Retry the program board</Proxy>}
            {!inspected && TICKETS.map((t) => (
              <Proxy key={t.id} id={`ticket:${t.id}`} world={world} onActivate={act}>
                Inspect ticket: {t.title}, {t.subtitle}. {readiness(stateOf(t.id)).label}
              </Proxy>
            ))}
            {inspected && (
              <section aria-label={`Ticket: ${inspected.title}`}>
                <h2>{inspected.title}</h2>
                <p>{inspected.subtitle}</p>
                <p>{inspected.synopsis}</p>
                <blockquote>{inspected.excerpt}</blockquote>
                <p>{readiness(stateOf(inspected.id)).label}</p>
                <Proxy id="inspect:seat" world={world} onActivate={act}>Take your seat</Proxy>
                <Proxy id="inspect:back" world={world} onActivate={act}>Return ticket</Proxy>
              </section>
            )}
          </section>
        ) : (
          <section aria-label="Your seat">
            <h1>{seatTicket?.title ?? "Paper Theater"} — stalls</h1>
            <p>{seatTicket ? readiness(stateOf(seatTicket.id)).label : ""}</p>
            <div role="toolbar" aria-label="Playback controls">
              <Proxy id="rail:play" world={world} onActivate={act} pressed={curtainOpen}>{curtainOpen ? "Pause" : "Play"}</Proxy>
              <Proxy id="rail:restart" world={world} onActivate={act} disabled>Restart</Proxy>
              <Proxy id="rail:seek" world={world} onActivate={act} disabled>Seek</Proxy>
              <Proxy id="rail:readalong" world={world} onActivate={act} pressed={readAlong}>Read along</Proxy>
              <Proxy id="rail:return" world={world} onActivate={act}>Back to the box office</Proxy>
            </div>
          </section>
        )}
        <footer>
          <h2>Sources &amp; credits</h2>
          <ul>
            <li><cite>The Wonderful Wizard of Oz</cite>, L. Frank Baum (Project Gutenberg #55)</li>
            <li><cite>The War of the Worlds</cite>, Book One Chapter I, H. G. Wells (Project Gutenberg #36)</li>
          </ul>
          <p>Playback uses the shared Paper Theater player. No account needed.</p>
        </footer>
      </main>
    </>
  );
}

createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
