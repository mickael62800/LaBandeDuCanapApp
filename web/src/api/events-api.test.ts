import { describe, expect, it } from "vitest";

import { emit, listen, once } from "./events-api";
import { emit as busEmit } from "./events";

describe("bus d'événements web (adaptateur events)", () => {
  it("listen livre le payload enveloppé dans l'enveloppe Event", async () => {
    const recu: unknown[] = [];
    const unlisten = await listen<{ n: number }>("ws:test", (ev) => recu.push(ev));

    busEmit("ws:test", { n: 7 });

    expect(recu).toEqual([{ event: "ws:test", windowLabel: "main", id: 0, payload: { n: 7 } }]);
    unlisten();
    busEmit("ws:test", { n: 8 });
    // Après désabonnement : plus rien.
    expect(recu).toHaveLength(1);
  });

  it("once ne se déclenche qu'une seule fois puis s'auto-désabonne", async () => {
    let appels = 0;
    await once<number>("ws:unique", (ev) => { appels += 1; expect(ev.payload).toBe(3); });

    busEmit("ws:unique", 3);
    busEmit("ws:unique", 4);

    expect(appels).toBe(1);
  });

  it("emit côté client est un no-op résolu", async () => {
    await expect(emit("n'importe-quoi", { x: 1 })).resolves.toBeUndefined();
  });
});
