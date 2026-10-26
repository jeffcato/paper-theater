import React, { useCallback, useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { PlaqueView } from "@paper-theater/player";
import type { ReadAlongGroup, ReadAlongWord } from "@paper-theater/player";
import { TICKETS, ticketState } from "./tickets.js";
import type { Catalog } from "./tickets.js";
import "./styles.css";

export interface PlaqueTrack {
  words: ReadAlongWord[];
  groups: ReadAlongGroup[];
}

/** Bind the shared player plaque to a container. The view is created only
 * once a loaded performance supplies a track; until then the toggle and
 * sample updates are safe no-ops. Loaded performances prefer the
 * mount-owned plaque instead (MountOptions.readAlong, fed by the sampled
 * clock and controlled by setReadAlong); this hook covers standalone
 * plaque containers. Host pages must include
 * `@paper-theater/player/fonts/noto-serif.css` before mounting, or the
 * view reports fallback metrics via fontReady instead of certifying them. */
export function usePlaque(
  track: PlaqueTrack | null,
  sample: number,
  enabled: boolean,
  scale: number,
): React.RefObject<HTMLDivElement | null> {
  const ref = useRef<HTMLDivElement | null>(null);
  const view = useRef<PlaqueView | null>(null);
  useEffect(() => {
    const el = ref.current;
    if (!el || !track) return;
    const plaque = new PlaqueView(el, track.words, track.groups, { scale });
    view.current = plaque;
    void plaque.ready.then(() => plaque.setSample(sample));
    return () => {
      plaque.dispose();
      view.current = null;
    };
  }, [track]);
  useEffect(() => {
    view.current?.setEnabled(enabled);
  }, [enabled]);
  useEffect(() => {
    view.current?.setScale(scale);
  }, [scale]);
  useEffect(() => {
    if (enabled) view.current?.setSample(sample);
  }, [sample, enabled]);
  return ref;
}

type CatalogStatus = "loading" | "ready" | "error";

async function fetchCatalog(): Promise<Catalog> {
  const res = await fetch("tickets/catalog.json", { cache: "no-store" });
  if (!res.ok) throw new Error(`catalog unavailable: ${res.status}`);
  const data = (await res.json()) as Catalog;
  if (!data || !Array.isArray(data.packages)) throw new Error("catalog malformed");
  return data;
}

function useHashRoute(): string {
  const [hash, setHash] = useState(() => window.location.hash || "#/");
  useEffect(() => {
    const onChange = () => setHash(window.location.hash || "#/");
    window.addEventListener("hashchange", onChange);
    return () => window.removeEventListener("hashchange", onChange);
  }, []);
  return hash;
}

function Booth(props: {
  status: CatalogStatus;
  catalog: Catalog;
  onRetry: () => void;
}): React.JSX.Element {
  const state = ticketState(props.catalog);
  return (
    <main className="booth">
      <header className="booth-sign">
        <h1>Paper Theater</h1>
        <p className="tagline">An illustrated miniature stage for classic books</p>
      </header>
      {props.status === "loading" && (
        <p role="status" className="notice">Checking today&rsquo;s program&hellip;</p>
      )}
      {props.status === "error" && (
        <div className="notice" role="alert">
          <p>The program board is unreachable right now.</p>
          <button type="button" onClick={props.onRetry}>Retry</button>
        </div>
      )}
      <nav aria-label="Performances" className="tickets">
        {TICKETS.map((ticket) => {
          const open = props.status === "ready" && state[ticket.id] === "ready";
          return (
            <div key={ticket.id} className="ticket">
              <h2>{ticket.title}</h2>
              <p>{ticket.subtitle}</p>
              {open ? (
                <a className="ticket-link" href={ticket.href}>Take me to the theater</a>
              ) : (
                <p className="unavailable">
                  {props.status === "loading"
                    ? "Checking availability…"
                    : "Unavailable — this performance is still in preparation."}
                </p>
              )}
            </div>
          );
        })}
      </nav>
      <footer className="credits">
        <h2>Sources &amp; credits</h2>
        <ul>
          <li><cite>The Wonderful Wizard of Oz</cite>, L. Frank Baum (Gutenberg #55)</li>
          <li><cite>The War of the Worlds</cite>, Book One Chapter I, H. G. Wells (Gutenberg #36)</li>
        </ul>
        <p>Playback uses the shared Paper Theater player. No account needed.</p>
      </footer>
    </main>
  );
}

function Theater(props: { performanceId: string; onBack: () => void }): React.JSX.Element {
  const ticket = TICKETS.find((t) => t.id === props.performanceId);
  const [status, setStatus] = useState<CatalogStatus>("loading");
  const [readAlong, setReadAlong] = useState(false);
  const [reducedMotion, setReducedMotion] = useState(false);
  const [textScale, setTextScale] = useState(1);
  // No accepted package exists yet, so no track or samples flow; the hook
  // stays dormant until a loaded performance supplies them.
  const [track] = useState<PlaqueTrack | null>(null);
  const [sample] = useState(0);
  const plaqueRef = usePlaque(track, sample, readAlong, textScale);

  useEffect(() => {
    const query = window.matchMedia("(prefers-reduced-motion: reduce)");
    setReducedMotion(query.matches);
    const onChange = (e: MediaQueryListEvent) => setReducedMotion(e.matches);
    query.addEventListener("change", onChange);
    return () => query.removeEventListener("change", onChange);
  }, []);

  const check = useCallback(() => {
    setStatus("loading");
    fetchCatalog().then(
      (catalog) => {
        setStatus(ticketState(catalog)[props.performanceId] === "ready" ? "ready" : "error");
      },
      () => setStatus("error"),
    );
  }, [props.performanceId]);
  useEffect(check, [check]);

  const toggleFullscreen = useCallback(() => {
    if (document.fullscreenElement) {
      void document.exitFullscreen().catch(() => {});
    } else {
      void document.documentElement.requestFullscreen().catch(() => {});
    }
  }, []);

  if (!ticket) {
    return (
      <main className="theater">
        <p role="alert">Unknown performance.</p>
        <button type="button" onClick={props.onBack}>Back to the booth</button>
      </main>
    );
  }

  return (
    <main className="theater">
      <div className="slate" aria-label={`${ticket.title}, ${ticket.subtitle}`}>
        <h1>{ticket.title}</h1>
        <p>{ticket.subtitle}</p>
      </div>
      <div className="stage" role="region" aria-label="Stage">
        {status === "loading" && <p role="status">Loading the performance…</p>}
        {status !== "loading" && (
          <div className="unavailable-stage" role="alert">
            <p>This performance is not yet published. The audience shows an honest
              unavailable state until an accepted package exists — diagnostic
              demonstrations never appear here.</p>
            <button type="button" onClick={check}>Retry</button>
          </div>
        )}
      </div>
      <div
        ref={plaqueRef}
        className="plaque"
        hidden={!readAlong || track === null}
      />
      <div className="controls" role="toolbar" aria-label="Playback controls">
        <button type="button" disabled title="Available once the performance loads">Play</button>
        <button type="button" disabled title="Available once the performance loads">Restart</button>
        <label className="seek">
          Seek
          <input type="range" min={0} max={100} defaultValue={0} disabled aria-label="Seek" />
        </label>
        <button
          type="button"
          aria-pressed={readAlong}
          onClick={() => setReadAlong((v) => !v)}
        >
          Read along
        </button>
        <label className="text-size">
          Text size
          <select
            aria-label="Read-along text size"
            value={textScale}
            disabled={track === null}
            onChange={(e) => setTextScale(Number(e.target.value))}
          >
            <option value={1}>100%</option>
            <option value={1.5}>150%</option>
            <option value={2}>200%</option>
          </select>
        </label>
        <button type="button" onClick={toggleFullscreen}>Fullscreen</button>
        <span className="motion" aria-live="polite">
          {reducedMotion ? "Reduced motion on" : ""}
        </span>
      </div>
      <nav className="return">
        <button type="button" onClick={props.onBack}>Back to the booth</button>
      </nav>
    </main>
  );
}

function App(): React.JSX.Element {
  const hash = useHashRoute();
  const [catalog, setCatalog] = useState<Catalog>({ packages: [] });
  const [status, setStatus] = useState<CatalogStatus>("loading");

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

  const goBooth = useCallback(() => {
    window.location.hash = "#/";
  }, []);

  const match = hash.match(/^#\/theater\/([A-Za-z0-9-]+)\/?$/);
  if (match) {
    return <Theater performanceId={match[1]!} onBack={goBooth} />;
  }
  return <Booth status={status} catalog={catalog} onRetry={load} />;
}

createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
