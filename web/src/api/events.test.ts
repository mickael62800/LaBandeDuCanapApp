import { describe, expect, it } from "vitest";

import { emit, on } from "./events";

describe("bus d'événements local (emit/on)", () => {
  it("emit sans abonné est un no-op", () => {
    // Ne doit ni jeter ni produire d'effet.
    expect(() => emit("aucun-abonne", { x: 1 })).not.toThrow();
  });

  it("livre le payload à tous les abonnés du même événement, dans l'ordre", () => {
    const appels: unknown[] = [];
    on("bus:test", (ev) => appels.push(["a", ev.payload]));
    on("bus:test", (ev) => appels.push(["b", ev.payload]));

    emit("bus:test", 42);

    expect(appels).toEqual([["a", 42], ["b", 42]]);
  });

  it("une exception dans un abonné n'empêche pas les suivants d'être appelés", () => {
    const appels: string[] = [];
    on("bus:erreur", () => { throw new Error("boom"); });
    on("bus:erreur", (ev) => appels.push(String(ev.payload)));

    emit("bus:erreur", "ok");

    expect(appels).toEqual(["ok"]);
  });

  it("les événements distincts sont isolés les uns des autres", () => {
    const recu: string[] = [];
    on("bus:a", (ev) => recu.push(String(ev.payload)));

    emit("bus:b", "autre");

    expect(recu).toEqual([]);
  });

  it("on() renvoie un unsubscribe qui retire l'abonné", () => {
    let compteur = 0;
    const unlisten = on("bus:off", () => { compteur += 1; });

    emit("bus:off", null);
    expect(compteur).toBe(1);

    unlisten();
    // Double unsubscribe : toléré, sans effet.
    unlisten();
    emit("bus:off", null);
    expect(compteur).toBe(1);
  });
});
