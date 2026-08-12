import { describe, expect, it } from "vitest";
import { installTextInputPolicy } from "./inputPolicy";

describe("text input policy", () => {
  it("disables system correction on existing and dynamically added controls", async () => {
    const input = document.createElement("input");
    document.body.append(input);
    const dispose = installTextInputPolicy();
    const textarea = document.createElement("textarea");
    document.body.append(textarea);
    await Promise.resolve();

    for (const control of [input, textarea]) {
      expect(control).toHaveAttribute("autocorrect", "off");
      expect(control).toHaveAttribute("autocapitalize", "none");
      expect(control).toHaveAttribute("autocomplete", "off");
      expect(control.spellcheck).toBe(false);
    }
    dispose();
  });
});
