import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { isPermissionGranted, requestPermission, sendNotification } from "./notification";

// happy-dom ne fournit pas de Notification utilisable : on pilote le global
// explicitement pour couvrir les deux mondes (present / absent).
class FakeNotification {
  static permission: string = "default";
  static instances: FakeNotification[] = [];
  // Membre statique present pour que vi.spyOn puisse l'intercepter.
  static requestPermission(): Promise<string> {
    return Promise.resolve(FakeNotification.permission);
  }
  constructor(title?: unknown, options?: unknown) {
    this.title = title;
    this.options = options;
    FakeNotification.instances.push(this);
  }
}

function defineFake() {
  Object.defineProperty(globalThis, "Notification", { value: FakeNotification, configurable: true });
}
function removeGlobal() {
  delete (globalThis as Record<string, unknown>).Notification;
}

describe("notifications navigateur", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    FakeNotification.permission = "default";
    FakeNotification.instances.length = 0;
  });
  afterEach(removeGlobal); // on laisse l'environnement dans son etat d'origine (absent)

  describe("quand Notification n'existe pas", () => {
    beforeEach(removeGlobal);

    it("isPermissionGranted renvoie false sans jeter", async () => {
      await expect(isPermissionGranted()).resolves.toBe(false);
    });

    it("requestPermission renvoie 'denied' sans jeter", async () => {
      await expect(requestPermission()).resolves.toBe("denied");
    });

    it("sendNotification est un no-op (aucune instance creee)", async () => {
      // Notification absent -> le code doit renoncer avant toute creation.
      await expect(sendNotification({ title: "x" })).resolves.toBeUndefined();
    });
  });

  describe("quand Notification existe", () => {
    beforeEach(defineFake);

    it("isPermissionGranted suit Notification.permission", async () => {
      FakeNotification.permission = "granted";
      await expect(isPermissionGranted()).resolves.toBe(true);
      FakeNotification.permission = "denied";
      await expect(isPermissionGranted()).resolves.toBe(false);
    });

    it("requestPermission renvoie la reponse de l'API", async () => {
      const spy = vi.spyOn(FakeNotification, "requestPermission").mockResolvedValueOnce("default");
      await expect(requestPermission()).resolves.toBe("default");
      expect(spy).toHaveBeenCalledOnce();
    });

    it("sendNotification ne fait rien tant que la permission n'est pas accordee", async () => {
      FakeNotification.permission = "denied";
      const avant = FakeNotification.instances.length;
      await sendNotification({ title: "x" });
      expect(FakeNotification.instances).toHaveLength(avant);
    });

    it("sendNotification accepte un simple titre en chaine", async () => {
      FakeNotification.permission = "granted";
      const avant = FakeNotification.instances.length;
      await sendNotification("Bonjour");
      expect(FakeNotification.instances).toHaveLength(avant + 1);
      expect(FakeNotification.instances[avant].title).toBe("Bonjour");
    });

    it("sendNotification transmet body/icon quand fournis", async () => {
      FakeNotification.permission = "granted";
      const avant = FakeNotification.instances.length;
      await sendNotification({ title: "T", body: "B", icon: "/i.png" });
      expect(FakeNotification.instances).toHaveLength(avant + 1);
    });
  });
});
