// Store cle/valeur persistant — stockage cle/valeur via localStorage.

export class Store {
  constructor(private file: string) {}
  static async load(file: string): Promise<Store> { return new Store(file); }
  private ns(key: string) { return `ds.store:${this.file}:${key}`; }
  async get<T>(key: string): Promise<T | null> {
    const storageKey = this.ns(key);
    const raw = localStorage.getItem(storageKey);
    if (!raw) return null;
    try {
      return JSON.parse(raw) as T;
    } catch {
      localStorage.removeItem(storageKey);
      return null;
    }
  }
  async set(key: string, value: unknown): Promise<void> {
    localStorage.setItem(this.ns(key), JSON.stringify(value));
  }
  async delete(key: string): Promise<boolean> {
    const k = this.ns(key);
    const had = localStorage.getItem(k) !== null;
    localStorage.removeItem(k);
    return had;
  }
  async save(): Promise<void> { /* no-op */ }
  async clear(): Promise<void> {
    const prefix = `ds.store:${this.file}:`;
    for (let i = localStorage.length - 1; i >= 0; i--) {
      const k = localStorage.key(i);
      if (k && k.startsWith(prefix)) localStorage.removeItem(k);
    }
  }
  async keys(): Promise<string[]> {
    const prefix = `ds.store:${this.file}:`;
    const out: string[] = [];
    for (let i = 0; i < localStorage.length; i++) {
      const k = localStorage.key(i);
      if (k && k.startsWith(prefix)) out.push(k.slice(prefix.length));
    }
    return out;
  }
}

export async function load(file: string): Promise<Store> { return Store.load(file); }
