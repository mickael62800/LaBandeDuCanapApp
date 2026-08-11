// Helper pour recuperer l'URL de l'API backend depuis la config locale.

import { getApiConfig } from "@/api/config";

export async function getApiBaseUrl(): Promise<string> {
  const config = getApiConfig();
  // En prod, fallback URL relative -> passe par le proxy nginx.
  // En dev, fallback localhost:3000 -> hit l'API directement.
  const fallback = import.meta.env.PROD ? "" : "http://localhost:3000";
  return config?.api_url || import.meta.env.VITE_API_URL || fallback;
}
