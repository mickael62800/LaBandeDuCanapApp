<script setup lang="ts">
import { ref, watch } from "vue";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useAuth } from "@/composables/useAuth";
import { nexusGrandSalonService as api, type GrandSalonProfile, type GrandSalonMotion, type GrandSalonArticle, type GrandSalonCercle, type GrandSalonDossier } from "@/services/nexusGrandSalonService";

const { selectedGuildId, selectedGuild } = useGuildSelector();
const { user } = useAuth();
const profile=ref<GrandSalonProfile|null>(null), motions=ref<GrandSalonMotion[]>([]), articles=ref<GrandSalonArticle[]>([]);
const cercles=ref<GrandSalonCercle[]>([]), dossiers=ref<GrandSalonDossier[]>([]);
const loading=ref(false), error=ref(""), titre=ref(""), texte=ref(""), cercleNom=ref(""), cercleDevise=ref(""), sujet=ref("");
async function load(){const g=selectedGuildId.value,u=user.value;if(!g||!u)return;loading.value=true;error.value="";try{const [p,m,a,c,d]=await Promise.all([api.profile(g,u.id).catch(()=>null),api.motions(g),api.gazette(g),api.cercles(g),api.dossiers(g,u.id).catch(()=>[])]);profile.value=p;motions.value=m;articles.value=a;cercles.value=c;dossiers.value=d}catch(e){error.value=e instanceof Error?e.message:"Chargement impossible"}finally{loading.value=false}}
async function join(){const g=selectedGuildId.value,u=user.value;if(!g||!u)return;profile.value=await api.join(g,u.id,u.global_name||u.username);await load()}
async function daily(){const g=selectedGuildId.value,u=user.value;if(!g||!u)return;try{profile.value=await api.daily(g,u.id)}catch(e){error.value=e instanceof Error?e.message:"Participation impossible"}}
async function propose(){const g=selectedGuildId.value,u=user.value;if(!g||!u||!titre.value.trim()||!texte.value.trim())return;await api.propose(g,u.id,titre.value,texte.value);titre.value="";texte.value="";await load()}
async function vote(id:string,choice:boolean){const g=selectedGuildId.value,u=user.value;if(!g||!u)return;await api.vote(g,id,u.id,choice);await load()}
async function createCercle(){const g=selectedGuildId.value,u=user.value;if(!g||!u||!cercleNom.value.trim())return;await api.createCercle(g,u.id,cercleNom.value,cercleDevise.value);cercleNom.value="";cercleDevise.value="";await load()}
async function investigate(){const g=selectedGuildId.value,u=user.value;if(!g||!u||!sujet.value.trim())return;await api.investigate(g,u.id,sujet.value);sujet.value="";await load()}
async function reveal(id:string){const g=selectedGuildId.value,u=user.value;if(!g||!u)return;await api.reveal(g,id,u.id);await load()}
const fmt=(n:number)=>n.toLocaleString("fr-FR");
watch([selectedGuildId,user],load,{immediate:true});
</script>

<template><AdminPageShell title="Le Grand Salon" :subtitle="selectedGuild?.name ?? 'La Bande du Canapé'">
  <p v-if="error" class="gs-error">{{error}}</p><p v-if="loading">Chargement…</p>
  <section v-if="!profile&&!loading" class="gs-welcome"><h2>Prends place sur le canapé</h2><p>Deviens un habitué, défends des motions et fais parler de toi dans la Gazette.</p><button @click="join">Entrer dans le Grand Salon</button></section>
  <template v-else-if="profile"><button class="gs-daily" @click="daily">Participer aujourd’hui (+ ressources)</button><div class="gs-stats"><article><span>Rayonnement</span><b>{{fmt(profile.rayonnement)}}</b></article><article><span>Jetons canapé</span><b>{{fmt(profile.jetons)}}</b></article><article><span>Réputation</span><b>{{fmt(profile.reputation)}}</b></article><article><span>Bons plans</span><b>{{fmt(profile.bons_plans)}}</b></article><article><span>Réseau</span><b>{{fmt(profile.reseau)}}</b></article></div>
    <div class="gs-grid"><section><h2>Nouvelle motion</h2><input v-model="titre" maxlength="120" placeholder="Titre"/><textarea v-model="texte" maxlength="2000" placeholder="Ce que tu proposes au Salon"></textarea><button @click="propose">Soumettre au Salon</button><h2>Motions</h2><article v-for="m in motions" :key="m.id" class="gs-card"><small>{{m.status}} · clôture {{new Date(m.closes_at).toLocaleString('fr-FR')}}</small><h3>{{m.titre}}</h3><p>{{m.texte}}</p><div v-if="m.status==='en_vote'" class="gs-actions"><button @click="vote(m.id,true)">Pour</button><button class="secondary" @click="vote(m.id,false)">Contre</button></div></article></section>
    <section><h2>Les cercles</h2><input v-model="cercleNom" placeholder="Nom de ta bande"/><input v-model="cercleDevise" placeholder="Devise"/><button @click="createCercle">Fonder un cercle</button><article v-for="c in cercles" :key="c.id" class="gs-card"><small>{{c.kind}} · {{fmt(c.rayonnement)}} rayonnement</small><h3>{{c.name}}</h3><p>{{c.devise}}</p></article><h2>Dossiers</h2><input v-model="sujet" placeholder="Sujet à enquêter"/><button @click="investigate">Ouvrir un dossier</button><article v-for="d in dossiers" :key="d.id" class="gs-card"><h3>{{d.subject}}</h3><p>{{d.verified?'Information recoupée':'Rumeur à confirmer'}}</p><button v-if="d.verified&&!d.revealed_at" @click="reveal(d.id)">Révéler à la Gazette</button></article><h2>Gazette du Canapé</h2><p v-if="!articles.length">La Gazette attend son premier scoop.</p><article v-for="a in articles" :key="a.id" class="gs-card"><small>{{new Date(a.published_at).toLocaleString('fr-FR')}}</small><h3>{{a.headline}}</h3><p>{{a.body}}</p></article></section></div>
  </template></AdminPageShell></template>

<style scoped>.gs-error{color:var(--danger)}.gs-welcome,.gs-card,.gs-stats article{background:var(--bg-secondary);border:1px solid var(--border);border-radius:12px;padding:18px}.gs-stats{display:grid;grid-template-columns:repeat(auto-fit,minmax(130px,1fr));gap:12px;margin-bottom:18px}.gs-stats span{display:block;color:var(--text-secondary);font-size:12px}.gs-stats b{font-size:24px}.gs-grid{display:grid;grid-template-columns:1.4fr 1fr;gap:18px}.gs-card{margin:10px 0}.gs-card small{color:var(--text-secondary)}input,textarea{box-sizing:border-box;width:100%;margin:6px 0;padding:10px;background:var(--bg-primary);color:var(--text-primary);border:1px solid var(--border);border-radius:8px}textarea{min-height:100px}button{background:var(--universe-accent);color:white;border:0;border-radius:8px;padding:9px 14px;cursor:pointer}.secondary{background:var(--bg-tertiary)}.gs-actions{display:flex;gap:8px}@media(max-width:800px){.gs-grid{grid-template-columns:1fr}}</style>
