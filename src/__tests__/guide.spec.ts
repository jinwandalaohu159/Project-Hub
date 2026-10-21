import { mount } from "@vue/test-utils";
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { nextTick } from "vue";
import GuideReader from "../components/guide/GuideReader.vue";
import GuideHost from "../components/guide/GuideHost.vue";
import { guideFor, GUIDE_SECTION_IDS } from "../utils/guide";
import { parseInline } from "../utils/guide/inline";
import { useGuide } from "../composables/useGuide";
import { useI18n } from "../composables/useI18n";

describe("guide content", () => {
  it("has the same chapters, in the same order, in every language", () => {
    for (const locale of ["zh", "en"] as const) {
      expect(guideFor(locale).sections.map((section) => section.id), locale).toEqual([...GUIDE_SECTION_IDS]);
    }
  });

  it("gives every chapter a title, a summary and content", () => {
    for (const locale of ["zh", "en"] as const) {
      for (const section of guideFor(locale).sections) {
        expect(section.title && section.summary, `${locale}:${section.id}`).toBeTruthy();
        expect(section.blocks.length, `${locale}:${section.id}`).toBeGreaterThan(0);
      }
    }
  });
});

describe("parseInline", () => {
  it("splits bold and code runs from plain text", () => {
    expect(parseInline("Click **Save** or press `Ctrl+S`.")).toEqual([
      { kind: "text", text: "Click " },
      { kind: "strong", text: "Save" },
      { kind: "text", text: " or press " },
      { kind: "code", text: "Ctrl+S" },
      { kind: "text", text: "." },
    ]);
  });

  it("leaves unmatched markers as they are", () => {
    expect(parseInline("2 * 3 ** 4")).toEqual([{ kind: "text", text: "2 * 3 ** 4" }]);
  });
});

describe("the guide window", () => {
  beforeEach(() => {
    useI18n().setLocale("en");
    const guide = useGuide();
    guide.section.value = "welcome";
    guide.layout.value = "center";
    guide.open.value = false;
  });

  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("lists every chapter in its table of contents and follows a jump", async () => {
    const wrapper = mount(GuideReader, { props: { layout: "center" } });
    const items = wrapper.findAll(".guide-nav-item");
    expect(items).toHaveLength(GUIDE_SECTION_IDS.length);

    await items[3].trigger("click");
    expect(useGuide().section.value).toBe(GUIDE_SECTION_IDS[3]);
    expect(wrapper.findAll(".guide-nav-item")[3].classes()).toContain("active");
    wrapper.unmount();
  });

  it("goes on to the next chapter from the end of one", async () => {
    const wrapper = mount(GuideReader, { props: { layout: "center" } });
    await wrapper.find(".guide-next").trigger("click");
    expect(useGuide().section.value).toBe("quickStart");
    wrapper.unmount();
  });

  it("opens centered, docks on the right, and closes", async () => {
    const guide = useGuide();
    const wrapper = mount(GuideHost, { attachTo: document.body });
    guide.show();
    await nextTick();
    expect(document.querySelector(".guide-backdrop .guide-center")).not.toBeNull();

    guide.toggleLayout();
    await nextTick();
    expect(document.querySelector(".guide-backdrop")).toBeNull();
    expect(wrapper.find(".guide-dock .guide-docked").exists()).toBe(true);

    guide.close();
    await nextTick();
    expect(wrapper.find(".guide-dock").exists()).toBe(false);
    wrapper.unmount();
  });

  it("asks the app to open Settings from a guide button", async () => {
    const guide = useGuide();
    const wrapper = mount(GuideHost, { attachTo: document.body });
    guide.show("cloud");
    await nextTick();
    const button = Array.from(document.querySelectorAll<HTMLButtonElement>(".guide-action")).find((element) =>
      element.textContent?.includes("Settings"),
    );
    button!.click();
    await nextTick();
    expect(wrapper.emitted("openSettings")).toHaveLength(1);
    // Centered, the guide steps aside so Settings is not hidden behind it.
    expect(guide.open.value).toBe(false);
    wrapper.unmount();
  });
});
