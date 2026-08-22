import { describe, expect, it } from "vitest";
import { useConfirm } from "./useConfirm";

describe("useConfirm", () => {
  it("part visible=false avec les valeurs par défaut", () => {
    const c = useConfirm();
    expect(c.visible.value).toBe(false);
    expect(c.title.value).toBe("Confirmation");
    expect(c.message.value).toBe("");
  });

  it("confirm() ouvre la boîte et résout à true quand on valide", async () => {
    const c = useConfirm();
    const p = c.confirm({ message: "Supprimer ?" });
    expect(c.visible.value).toBe(true);
    expect(c.title.value).toBe("Confirmation"); // défaut
    expect(c.message.value).toBe("Supprimer ?");

    let resolu = false;
    void p.then((v) => {
      resolu = v;
    });
    c.resolve(true);
    await Promise.resolve();
    expect(resolu).toBe(true);
    expect(c.visible.value).toBe(false); // refermée après résolution
  });

  it("honore un titre personnalisé", async () => {
    const c = useConfirm();
    void c.confirm({ title: "Attention", message: "Sûr ?" }).then(() => {});
    expect(c.title.value).toBe("Attention");
  });

  it("résout à false quand on annule, et ignore un second resolve orphelin", async () => {
    const c = useConfirm();
    const p = c.confirm({ message: "m" }).then((v) => v);
    c.resolve(false);
    await Promise.resolve();
    expect(await p).toBe(false);

    // Un resolve sans confirm en cours ne doit rien casser.
    expect(() => c.resolve(true)).not.toThrow();
  });

  it("permet d'enchaîner deux confirmations", async () => {
    const c = useConfirm();
    const premier = c.confirm({ message: "1" }).then((v) => v);
    c.resolve(true);
    expect(await premier).toBe(true);

    const second = c.confirm({ title: "Deuxième", message: "2" }).then((v) => v);
    c.resolve(false);
    expect(await second).toBe(false);
  });
});
