// Audience ticket catalog rules: two tickets, neutral labels, honest
// unavailable state until accepted packages exist. Diagnostic or synthetic
// packages never produce a working ticket.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { ticketState, TICKETS } from "../src/tickets.js";

describe("audience: two-ticket booth", () => {
  it("offers exactly two tickets with story-neutral labels", () => {
    assert.equal(TICKETS.length, 2);
    for (const t of TICKETS) {
      assert.ok(t.id && t.title && t.href.startsWith("#/theater/"));
      // Labels carry only title/author/chapter: no later story facts.
      assert.ok(!/cyclone|thunder|martian|pirate/i.test(`${t.title} ${t.subtitle}`));
    }
    const ids = TICKETS.map((t) => t.id).sort();
    assert.deepEqual(ids, ["oz-chapter-1", "wells-book-one-chapter-1"]);
  });

  it("shows honest unavailable state with zero accepted packages", () => {
    const state = ticketState({ packages: [] });
    for (const t of TICKETS) {
      assert.equal(state[t.id], "unavailable");
    }
  });

  it("never grants a ticket from diagnostic or synthetic packages", () => {
    const state = ticketState({
      packages: [
        { id: "oz-chapter-1", provenance: "diagnostic", accepted: false },
        { id: "wells-book-one-chapter-1", provenance: "synthetic", accepted: true },
      ],
    });
    assert.equal(state["oz-chapter-1"], "unavailable");
    assert.equal(state["wells-book-one-chapter-1"], "unavailable");
  });

  it("opens tickets only for accepted autonomous packages", () => {
    const state = ticketState({
      packages: [{ id: "oz-chapter-1", provenance: "autonomous", accepted: true }],
    });
    assert.equal(state["oz-chapter-1"], "ready");
    assert.equal(state["wells-book-one-chapter-1"], "unavailable");
  });

  it("ignores catalog entries for unknown performances", () => {
    const state = ticketState({
      packages: [{ id: "unknown-show", provenance: "autonomous", accepted: true }],
    });
    assert.equal(state["oz-chapter-1"], "unavailable");
    assert.equal(state["wells-book-one-chapter-1"], "unavailable");
  });
});
