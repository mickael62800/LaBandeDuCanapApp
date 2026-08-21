// Amorçage commun des tests : rend `localStorage` et `sessionStorage`
// utilisables quelle que soit la version de Node.
//
// LE PROBLÈME. Depuis Node 24, la plateforme expose un `localStorage` natif
// (Web Storage) sur `globalThis`, via un accesseur. Il l'emporte sur celui
// que happy-dom installe dans son `window`, et il refuse de fonctionner sans
// l'option `--localstorage-file` : le premier `localStorage.clear()` d'un test
// lève « clear is not a function ». Sous Node 22 — la version de la CI — le
// natif n'existe pas et tout se passait bien, d'où des tests verts en
// intégration continue et rouges sur une machine à jour.
//
// LE CHOIX. On réinstalle explicitement le stockage du DOM sur `globalThis`.
// Les tests s'exécutent alors sur la même implémentation que le navigateur,
// et le comportement ne dépend plus de la version de Node — un test ne doit
// pas raconter une histoire différente selon la machine qui le lance.

/// Stockage minimal, utilisé si happy-dom n'en fournit pas.
///
/// Volontairement complet : un objet partiel passerait les tests d'aujourd'hui
/// et casserait sur le premier appel à `key()` ou `length` écrit demain.
function stockageEnMemoire(): Storage {
  const donnees = new Map<string, string>();
  return {
    get length() {
      return donnees.size;
    },
    clear: () => donnees.clear(),
    getItem: (cle: string) => (donnees.has(cle) ? (donnees.get(cle) as string) : null),
    key: (index: number) => Array.from(donnees.keys())[index] ?? null,
    removeItem: (cle: string) => void donnees.delete(cle),
    setItem: (cle: string, valeur: string) => void donnees.set(cle, String(valeur)),
  } as Storage;
}

function utilisable(candidat: unknown): candidat is Storage {
  return typeof (candidat as Storage | undefined)?.clear === "function";
}

function installer(nom: "localStorage" | "sessionStorage"): void {
  // `window` est absent si l'environnement de test n'est pas un DOM : on
  // retombe alors sur le stockage en memoire plutot que d'echouer au
  // chargement, ce qui rendrait tous les tests illisibles.
  const depuisLeDom = typeof window === "undefined" ? undefined : window[nom];
  const stockage = utilisable(depuisLeDom) ? depuisLeDom : stockageEnMemoire();

  // `configurable: true` parce que Node définit le sien par un accesseur :
  // une affectation simple (`globalThis.localStorage = ...`) serait ignorée.
  Object.defineProperty(globalThis, nom, {
    value: stockage,
    configurable: true,
    writable: true,
  });
}

installer("localStorage");
installer("sessionStorage");
