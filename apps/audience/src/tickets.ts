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
  },
  {
    id: "wells-book-one-chapter-1",
    title: "The War of the Worlds",
    subtitle: "H. G. Wells — Book One, Chapter I",
    href: "#/theater/wells-book-one-chapter-1",
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
