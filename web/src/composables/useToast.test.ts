import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useToast } from "./useToast";

describe("useToast", () => {
  let toasts: ReturnType<typeof useToast>;

  beforeEach(() => {
    // Etat de module : on repart d'une liste vide a chaque test.
    toasts = useToast();
    for (const t of [...toasts.toasts.value]) toasts.remove(t.id);
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.runAllTimers(); // purge les timeouts en attente avant de repasser aux timers reels
    vi.useRealTimers();
  });

  it("show ajoute un toast avec id incremente et duree par defaut", () => {
    toasts.show("info", "bonjour");
    expect(toasts.toasts.value).toHaveLength(1);
    const t = toasts.toasts.value[0];
    expect(t.type).toBe("info");
    expect(t.message).toBe("bonjour");
    expect(t.duration).toBe(4000);

    // L'auto-suppression est planifiee a la duree annoncee.
    vi.advanceTimersByTime(3999);
    expect(toasts.toasts.value).toHaveLength(1);
    vi.advanceTimersByTime(1);
    expect(toasts.toasts.value).toHaveLength(0);

    // Deuxieme toast : id superieur au premier.
    toasts.show("info", "encore");
    expect(toasts.toasts.value[0].id).toBeGreaterThan(t.id);
  });

  it("duree = 0 : le toast persiste (pas de timer)", () => {
    toasts.show("warning", "persistant", 0);
    vi.advanceTimersByTime(1_000_000);
    expect(toasts.toasts.value).toHaveLength(1);

    // remove manuel fonctionne quand meme.
    const id = toasts.toasts.value[0].id;
    toasts.remove(id);
    expect(toasts.toasts.value).toHaveLength(0);
  });

  it("remove ignore un id inconnu sans casser la liste", () => {
    toasts.show("info", "a");
    const id = toasts.toasts.value[0].id;
    toasts.remove(99_999); // inexistant : aucun effet
    expect(toasts.toasts.value).toHaveLength(1);
    toasts.remove(id);
    expect(toasts.toasts.value).toHaveLength(0);
  });

  it("success / error / warning / info posent le bon type et la duree attendue", () => {
    toasts.success("ok");
    toasts.error("ko");
    toasts.warning("attenti");
    toasts.info("note");

    const [s, e, w, i] = toasts.toasts.value;
    expect(s.type).toBe("success");
    expect(e.type).toBe("error");
    expect(w.type).toBe("warning");
    expect(i.type).toBe("info");
    // error est plus long que success ; warning au milieu.
    expect(e.duration).toBeGreaterThan(s.duration);
    expect(w.duration).toBeGreaterThanOrEqual(s.duration);

    for (const t of toasts.toasts.value) toasts.remove(t.id);
  });
});
