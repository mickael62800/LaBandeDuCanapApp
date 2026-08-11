import { nexusGet, nexusPost } from "@/api/nexusHttp";

export interface GrandSalonProfile { id:string; guild_id:string; user_id:string; display_name:string; rayonnement:number; jetons:number; reputation:number; bons_plans:number; reseau:number; joined_at:string }
export interface GrandSalonMotion { id:string; titre:string; texte:string; status:"en_vote"|"adoptee"|"rejetee"; closes_at:string; soutien_pour:number; soutien_contre:number }
export interface GrandSalonArticle { id:string; headline:string; body:string; published_at:string }
export interface GrandSalonCercle { id:string; kind:string; name:string; devise:string; caisse:number; reputation:number; rayonnement:number }
export interface GrandSalonDossier { id:string; subject:string; verified:boolean; revealed_at:string|null }

const root = (guildId:string) => `/api/grand-salon/${encodeURIComponent(guildId)}`;
export const nexusGrandSalonService = {
  membership: (g:string,u:string) => nexusGet<GrandSalonProfile|null>(`${root(g)}/membership/${encodeURIComponent(u)}`,g),
  profile: (g:string,u:string) => nexusGet<GrandSalonProfile>(`${root(g)}/habitues/${encodeURIComponent(u)}`,g),
  join: (g:string,u:string,name:string) => nexusPost<GrandSalonProfile>(`${root(g)}/habitues/${encodeURIComponent(u)}`,g,{display_name:name}),
  daily: (g:string,u:string) => nexusPost<GrandSalonProfile>(`${root(g)}/habitues/${encodeURIComponent(u)}/daily`,g),
  motions: (g:string) => nexusGet<GrandSalonMotion[]>(`${root(g)}/motions`,g),
  propose: (g:string,userId:string,titre:string,texte:string) => nexusPost<GrandSalonMotion>(`${root(g)}/motions`,g,{user_id:userId,titre,texte}),
  vote: (g:string,motionId:string,userId:string,choice:boolean) => nexusPost<void>(`${root(g)}/motions/${motionId}/vote`,g,{user_id:userId,choice}),
  gazette: (g:string) => nexusGet<GrandSalonArticle[]>(`${root(g)}/gazette`,g),
  cercles: (g:string) => nexusGet<GrandSalonCercle[]>(`${root(g)}/cercles`,g),
  createCercle: (g:string,userId:string,name:string,devise:string) => nexusPost<GrandSalonCercle>(`${root(g)}/cercles`,g,{user_id:userId,kind:"bande",name,devise}),
  dossiers: (g:string,userId:string) => nexusGet<GrandSalonDossier[]>(`${root(g)}/dossiers/${encodeURIComponent(userId)}`,g),
  investigate: (g:string,userId:string,subject:string) => nexusPost<GrandSalonDossier>(`${root(g)}/dossiers`,g,{user_id:userId,subject}),
  reveal: (g:string,id:string,userId:string) => nexusPost<void>(`${root(g)}/dossiers/${id}/reveal`,g,{user_id:userId}),
};
