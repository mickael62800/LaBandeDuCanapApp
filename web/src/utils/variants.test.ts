import { describe, expect, it } from "vitest";
import {
  actionLabel,
  actionVariant,
  eventIcon,
  eventLabel,
  eventVariant,
  ideaStatusVariant,
  infractionTypeVariant,
  levelVariant,
  priorityVariant,
  severityVariant,
  statusVariant,
  typeLabel,
} from "./variants";

describe("severityVariant", () => {
  it("mappe chaque gravite connue", () => {
    expect(severityVariant("critical")).toBe("danger");
    expect(severityVariant("urgent")).toBe("danger");
    expect(severityVariant("high")).toBe("warning");
    expect(severityVariant("medium")).toBe("info");
    expect(severityVariant("low")).toBe("default");
  });

  it("retourne default pour une gravite inconnue", () => {
    expect(severityVariant("inconnu")).toBe("default");
  });
});

describe("actionVariant", () => {
  it("mappe les actions de moderation", () => {
    expect(actionVariant("ban")).toBe("danger");
    expect(actionVariant("ban_permanent")).toBe("danger");
    expect(actionVariant("ban_temp")).toBe("danger");
    expect(actionVariant("lockdown")).toBe("danger");
    expect(actionVariant("mute")).toBe("warning");
    expect(actionVariant("mute_permanent")).toBe("warning");
    expect(actionVariant("mute_temp")).toBe("warning");
    expect(actionVariant("delete")).toBe("warning");
    expect(actionVariant("warn")).toBe("info");
    expect(actionVariant("none")).toBe("default");
    expect(actionVariant("unban")).toBe("success");
    expect(actionVariant("unmute")).toBe("success");
    expect(actionVariant("autre")).toBe("default");
  });
});

describe("statusVariant", () => {
  it("mappe les statuts", () => {
    expect(statusVariant("open")).toBe("info");
    expect(statusVariant("pending")).toBe("warning");
    expect(statusVariant("closed")).toBe("success");
    expect(statusVariant("autre")).toBe("default");
  });
});

describe("priorityVariant", () => {
  it("mappe les priorites", () => {
    expect(priorityVariant("urgent")).toBe("danger");
    expect(priorityVariant("high")).toBe("warning");
    expect(priorityVariant("medium")).toBe("info");
    expect(priorityVariant("low")).toBe("default");
    expect(priorityVariant("autre")).toBe("default");
  });
});

describe("levelVariant", () => {
  it("mappe les niveaux de log", () => {
    expect(levelVariant("info")).toBe("info");
    expect(levelVariant("warn")).toBe("warning");
    expect(levelVariant("error")).toBe("danger");
    expect(levelVariant("debug")).toBe("default");
  });
});

describe("infractionTypeVariant", () => {
  it("mappe les types d'infraction", () => {
    expect(infractionTypeVariant("ban")).toBe("danger");
    expect(infractionTypeVariant("mute")).toBe("warning");
    expect(infractionTypeVariant("warn")).toBe("info");
    expect(infractionTypeVariant("autre")).toBe("default");
  });
});

describe("actionLabel", () => {
  it("traduit les actions connues", () => {
    expect(actionLabel("ban")).toBe("Bannissement");
    expect(actionLabel("mute")).toBe("Sourdine");
    expect(actionLabel("delete")).toBe("Suppression");
    expect(actionLabel("warn")).toBe("Avertissement");
    expect(actionLabel("lockdown")).toBe("Verrouillage");
    expect(actionLabel("none")).toBe("Seul : rien");
  });

  it("rejoue l'action en l'etat pour une action inconnue", () => {
    expect(actionLabel("kick")).toBe("kick");
  });
});

describe("typeLabel", () => {
  it("traduit les types connus", () => {
    expect(typeLabel("spam")).toBe("Spam");
    expect(typeLabel("phishing")).toBe("Hameconnage");
    expect(typeLabel("harassment")).toBe("Harcelement");
  });

  it("rejoue le type en l'etat pour un type inconnu", () => {
    expect(typeLabel("custom")).toBe("custom");
  });
});

describe("eventVariant", () => {
  it("mappe les evenements", () => {
    expect(eventVariant("member_ban")).toBe("danger");
    expect(eventVariant("channel_delete")).toBe("danger");
    expect(eventVariant("member_leave")).toBe("warning");
    expect(eventVariant("message_delete")).toBe("warning");
    expect(eventVariant("member_roles_update")).toBe("warning");
    expect(eventVariant("member_join")).toBe("success");
    expect(eventVariant("member_unban")).toBe("success");
    expect(eventVariant("channel_create")).toBe("success");
    expect(eventVariant("voice_join")).toBe("info");
    expect(eventVariant("voice_leave")).toBe("info");
    expect(eventVariant("voice_move")).toBe("info");
    expect(eventVariant("autre")).toBe("default");
  });
});

describe("eventLabel", () => {
  it("traduit les evenements connus", () => {
    expect(eventLabel("message_delete")).toBe("Message supprimé");
    expect(eventLabel("member_join")).toBe("Arrivée d'un membre");
    expect(eventLabel("voice_join")).toBe("Entrée en vocal");
    expect(eventLabel("raid_detected")).toBe("Raid détecté");
    expect(eventLabel("ticket_create")).toBe("Ticket créé");
    expect(eventLabel("invite_use")).toBe("Invitation utilisée");
  });

  it("humanise un event_type inconnu", () => {
    expect(eventLabel("member_ban_xyz")).toBe("Member ban xyz");
    // Cas limite : un type ne contenant que des underscores revient tel quel.
    expect(eventLabel("___")).toBe("___");
  });
});

describe("eventIcon", () => {
  it("retourne l'icone de l'evenement ou ? par defaut", () => {
    expect(eventIcon("message_delete")).toBe("X");
    expect(eventIcon("member_join")).toBe("+");
    expect(eventIcon("voice_move")).toBe("M");
    expect(eventIcon("channel_create")).toBe("#");
    expect(eventIcon("autre")).toBe("?");
  });
});

describe("ideaStatusVariant", () => {
  it("mappe les statuts d'idee", () => {
    expect(ideaStatusVariant("nouvelle")).toBe("info");
    expect(ideaStatusVariant("en_discussion")).toBe("warning");
    expect(ideaStatusVariant("acceptee")).toBe("success");
    expect(ideaStatusVariant("refusee")).toBe("danger");
    expect(ideaStatusVariant("realisee")).toBe("success");
    expect(ideaStatusVariant("autre")).toBe("default");
  });
});
