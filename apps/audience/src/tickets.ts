// Audience ticket catalog (pure, dependency-free so the booth rules stay
// testable without a browser). Labels follow the no-spoilers contract: book
// title, author and chapter label only — never later scenes or characters.
export type TicketStatus = "ready" | "unavailable";
export type Provenance = "autonomous" | "imported" | "synthetic" | "diagnostic";

export interface Ticket {
  id: string;
  title: string;
  subtitle: string;
  href: string;
  /** Book-level framing only; never a later scene, character or event. */
  synopsis: string;
  /** The chapter's opening words, verbatim from the frozen source text. */
  excerpt: string;
}

export interface CatalogPackage {
  id: string;
  provenance: Provenance;
  accepted: boolean;
}
export interface Catalog {
  packages: CatalogPackage[];
}

export const TICKETS: readonly Ticket[] = [
  {
    id: "oz-chapter-1",
    title: "The Wonderful Wizard of Oz",
    subtitle: "L. Frank Baum — Chapter 1",
    href: "#/theater/oz-chapter-1",
    synopsis: "L. Frank Baum’s 1900 fairy tale, cut out and coloured for the toy-theatre stage.",
    excerpt: "Dorothy lived in the midst of the great Kansas prairies, with Uncle Henry, who was a farmer, and Aunt Em, who was the farmer’s wife.",
  },
  {
    id: "wells-book-one-chapter-1",
    title: "The War of the Worlds",
    subtitle: "H. G. Wells — Book One, Chapter I",
    href: "#/theater/wells-book-one-chapter-1",
    synopsis: "H. G. Wells’s 1898 novel, opening chapter, staged in paper and lamplight.",
    excerpt: "No one would have believed in the last years of the nineteenth century that this world was being watched keenly and closely by intelligences greater than man’s and yet as mortal as his own;",
  },
];

/**
 * Resolve each ticket to ready/unavailable. Only an accepted autonomous
 * package opens its ticket. Until accepted packages exist every ticket is
 * honestly unavailable; diagnostic/synthetic packages never populate a
 * working ticket, and unknown catalog entries are ignored.
 */
export function ticketState(catalog: Catalog): Record<string, TicketStatus> {
  const accepted = new Set<string>();
  for (const pkg of catalog.packages) {
    if (pkg.provenance === "autonomous" && pkg.accepted === true) {
      accepted.add(pkg.id);
    }
  }
  const state: Record<string, TicketStatus> = {};
  for (const ticket of TICKETS) {
    state[ticket.id] = accepted.has(ticket.id) ? "ready" : "unavailable";
  }
  return state;
}

/** Readiness wording for the inspection card: honest, never a release claim. */
export function readiness(status: TicketStatus): { label: string; playable: boolean } {
  return status === "ready"
    ? { label: "Performance ready — curtain up when you are seated", playable: true }
    : { label: "Under rehearsal — not yet published. Play opens the empty stage", playable: false };
}
