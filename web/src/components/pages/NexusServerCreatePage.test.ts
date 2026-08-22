import { describe, expect, it, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { ref } from "vue";

const push = vi.fn();
const success = vi.fn();
const showError = vi.fn();

vi.mock("vue-router", () => ({
  useRouter: () => ({ push }),
  RouterLink: { name: "RouterLink", props: ["to"], template: "<a><slot /></a>" },
}));
vi.mock("@/services/nexusGamesService", () => ({
  nexusGamesService: {
    listTemplates: vi.fn(),
    create: vi.fn(),
    schedule: vi.fn(),
  },
  adresseServeur: vi.fn(),
}));
vi.mock("@/services/communityAdminService", () => ({
  communityAdminService: { createEvent: vi.fn() },
}));
vi.mock("../../composables/useGuildSelector", () => ({
  useGuildSelector: () => ({
    selectedGuildId: ref("g1"),
    selectedGuild: ref({ id: "g1", name: "Le Canap" }),
  }),
}));
vi.mock("../../composables/useAuth", () => ({
  useAuth: () => ({ user: ref({ id: "u1" }) }),
}));
vi.mock("../../composables/useToast", () => ({
  useToast: () => ({ success, error: showError }),
}));

import NexusServerCreatePage from "./NexusServerCreatePage.vue";
import { nexusGamesService } from "@/services/nexusGamesService";
import { communityAdminService } from "@/services/communityAdminService";

const listTemplates = vi.mocked(nexusGamesService.listTemplates);
const create = vi.mocked(nexusGamesService.create);
const schedule = vi.mocked(nexusGamesService.schedule);
const createEvent = vi.mocked(communityAdminService.createEvent);

const TEMPLATE = {
  id: "t1",
  slug: "palworld",
  name: "Palworld",
  container_port: 8211,
  default_memory_mb: 8192,
  min_memory_mb: 4096,
  max_memory_mb: 16384,
  supports_rcon: false,
  supports_mods: false,
  config_schema: [],
};

beforeEach(() => {
  vi.clearAllMocks();
  listTemplates.mockResolvedValue([TEMPLATE] as never);
  create.mockResolvedValue({ id: "s1", name: "palworld" } as never);
  schedule.mockResolvedValue(undefined as never);
  createEvent.mockResolvedValue(undefined as never);
});

/// Monte la page et choisit le jeu : sans jeu choisi, l'etape 2 n'existe pas.
async function monterEtChoisirLeJeu() {
  const wrapper = mount(NexusServerCreatePage);
  await flushPromises();
  await wrapper.find(".nc-game").trigger("click");
  await flushPromises();
  return wrapper;
}

/// Les deux boutons d'action, dans l'ordre du gabarit.
function boutons(wrapper: Awaited<ReturnType<typeof monterEtChoisirLeJeu>>) {
  const tous = wrapper.findAll(".nc-actions button");
  return { programmer: tous[0]!, sansAnnonce: tous[1]! };
}

describe("« Créer sans annoncer »", () => {
  it("crée le serveur sans rien publier sur Discord", async () => {
    // Tout l'objet du bouton : c'est la PROGRAMMATION qui publie
    // `game_server_scheduled` vers nexus-bot, donc qui fait creer les salons et
    // le panneau d'inscription. Ne pas l'appeler, c'est ne rien annoncer.
    const wrapper = await monterEtChoisirLeJeu();
    await boutons(wrapper).sansAnnonce.trigger("click");
    await flushPromises();

    expect(create).toHaveBeenCalledTimes(1);
    expect(schedule).not.toHaveBeenCalled();
    expect(createEvent).not.toHaveBeenCalled();
  });

  it("n'exige pas de dates", async () => {
    // C'est precisement l'interet : preparer un serveur avant de savoir quand
    // la soiree aura lieu.
    const wrapper = await monterEtChoisirLeJeu();
    const dates = wrapper.findAll('input[type="datetime-local"]');
    for (const champ of dates) await champ.setValue("");
    await flushPromises();

    expect(boutons(wrapper).sansAnnonce.attributes("disabled")).toBeUndefined();
    await boutons(wrapper).sansAnnonce.trigger("click");
    await flushPromises();
    expect(create).toHaveBeenCalledTimes(1);
  });

  it("dit a l'utilisateur qu'aucun salon n'a ete demande", async () => {
    // Sans cela, rien ne distingue les deux boutons apres coup, et l'on va
    // verifier sur Discord.
    const wrapper = await monterEtChoisirLeJeu();
    await boutons(wrapper).sansAnnonce.trigger("click");
    await flushPromises();

    expect(success).toHaveBeenCalledWith(expect.stringContaining("Discord"));
  });

  it("ouvre la page du serveur cree", async () => {
    const wrapper = await monterEtChoisirLeJeu();
    await boutons(wrapper).sansAnnonce.trigger("click");
    await flushPromises();

    expect(push).toHaveBeenCalledWith("/nexus/servers/s1");
  });

  it("signale l'echec sans naviguer", async () => {
    create.mockRejectedValueOnce(new Error("plus de port libre"));
    const wrapper = await monterEtChoisirLeJeu();
    await boutons(wrapper).sansAnnonce.trigger("click");
    await flushPromises();

    expect(showError).toHaveBeenCalledWith("plus de port libre");
    expect(push).not.toHaveBeenCalled();
  });
});

describe("« Créer et programmer la soirée »", () => {
  it("crée, programme, puis inscrit la soirée au calendrier", async () => {
    const wrapper = await monterEtChoisirLeJeu();
    await boutons(wrapper).programmer.trigger("click");
    await flushPromises();

    expect(create).toHaveBeenCalledTimes(1);
    expect(schedule).toHaveBeenCalledTimes(1);
    expect(createEvent).toHaveBeenCalledTimes(1);
  });

  it("reste possible meme si le calendrier refuse l'evenement", async () => {
    // Le serveur est deja cree et programme a ce stade : echouer bruyamment
    // ferait croire que rien n'a eu lieu, alors que les salons Discord ont ete
    // demandes.
    createEvent.mockRejectedValueOnce(new Error("403"));
    const wrapper = await monterEtChoisirLeJeu();
    await boutons(wrapper).programmer.trigger("click");
    await flushPromises();

    expect(schedule).toHaveBeenCalledTimes(1);
    expect(showError).not.toHaveBeenCalled();
    expect(push).toHaveBeenCalledWith("/nexus/servers/s1");
  });

  it("est refusé tant que les dates sont incomplètes", async () => {
    const wrapper = await monterEtChoisirLeJeu();
    for (const champ of wrapper.findAll('input[type="datetime-local"]')) {
      await champ.setValue("");
    }
    await flushPromises();

    expect(boutons(wrapper).programmer.attributes("disabled")).toBeDefined();
  });
});

describe("garde-fous communs aux deux boutons", () => {
  it("refuse un nom que la base rejetterait", async () => {
    // `chk_game_servers_name` n'accepte que lettres, chiffres, espaces, tirets
    // et underscores : laisser passer donnerait une erreur serveur opaque.
    const wrapper = await monterEtChoisirLeJeu();
    await wrapper.find('input[type="text"]').setValue("nom/invalide");
    await flushPromises();

    const { programmer, sansAnnonce } = boutons(wrapper);
    expect(programmer.attributes("disabled")).toBeDefined();
    expect(sansAnnonce.attributes("disabled")).toBeDefined();
  });

  it("refuse une mémoire hors des bornes du template", async () => {
    const wrapper = await monterEtChoisirLeJeu();
    const memoire = wrapper.findAll('input[type="number"]')[0]!;
    await memoire.setValue(TEMPLATE.max_memory_mb + 512);
    await flushPromises();

    expect(boutons(wrapper).sansAnnonce.attributes("disabled")).toBeDefined();
  });
});

describe("guide de ressources", () => {
  it("n'apparait qu'une fois le jeu choisi, et pour ce jeu", async () => {
    const wrapper = mount(NexusServerCreatePage);
    await flushPromises();
    expect(wrapper.find(".rg").exists()).toBe(false);

    await wrapper.find(".nc-game").trigger("click");
    await flushPromises();

    const noms = wrapper.findAll(".rg-game-name").map((n) => n.text());
    expect(noms).toEqual(["Palworld"]);
  });
});
