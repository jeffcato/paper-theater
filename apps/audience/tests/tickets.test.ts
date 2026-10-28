// Audience ticket catalog rules: two tickets, neutral labels, honest
// unavailable state until accepted packages exist. Diagnostic or synthetic
// packages never produce a working ticket.
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { readiness, ticketState, TICKETS } from "../src/tickets.js";

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

describe("audience: ticket inspection card", () => {
  const source = (name: string) =>
    readFileSync(new URL(`../../../../fixtures/generation/source/${name}`, import.meta.url), "utf8").replace(/\s+/g, " ");

  it("quotes each chapter's opening words verbatim from the frozen source", () => {
    const [oz, wells] = [TICKETS[0]!, TICKETS[1]!];
    assert.ok(oz.excerpt.startsWith("Dorothy lived in the midst"));
    assert.ok(source("pg55-clean.txt").includes(oz.excerpt));
    assert.ok(wells.excerpt.startsWith("No one would have believed"));
    assert.ok(source("pg36-clean.txt").includes(wells.excerpt));
  });

  it("keeps synopses free of later story facts", () => {
    for (const t of TICKETS) {
      assert.ok(t.synopsis.length > 20);
      assert.ok(!/cyclone|thunder|martian|pirate|witch|munchkin|scarecrow|cylinder|heat-ray|tripod|invad/i.test(t.synopsis), t.synopsis);
    }
  });

  it("states readiness honestly without implying a published release", () => {
    assert.match(readiness("unavailable").label, /under rehearsal/i);
    assert.match(readiness("unavailable").label, /not yet published/i);
    assert.equal(readiness("unavailable").playable, false);
    assert.equal(readiness("ready").playable, true);
  });
});
