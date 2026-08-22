import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import StatusDot from "./StatusDot.vue";

describe("StatusDot", () => {
  for (const status of ["online", "offline", "warning"] as const) {
    it(`affiche le point pour l'etat ${status}`, () => {
      const wrapper = mount(StatusDot, { props: { status } });
      expect(wrapper.classes()).toContain("status-dot");
      expect(wrapper.classes()).toContain(`status-dot--${status}`);
    });
  }

  it("change de classe quand l'etat change", async () => {
    const wrapper = mount(StatusDot, { props: { status: "online" } });
    await wrapper.setProps({ status: "offline" });
    expect(wrapper.classes()).toContain("status-dot--offline");
    expect(wrapper.classes()).not.toContain("status-dot--online");
  });
});
