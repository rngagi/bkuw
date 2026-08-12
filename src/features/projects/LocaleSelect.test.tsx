import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import i18n from "../../i18n";
import { LocaleSelect } from "./LocaleSelect";

describe("LocaleSelect", () => {
  beforeEach(async () => {
    window.localStorage.clear();
    await i18n.changeLanguage("en");
  });

  it("switches immediately to Taiwan Traditional Chinese and persists the choice", async () => {
    render(<LocaleSelect />);
    fireEvent.change(screen.getByLabelText("Interface language"), { target: { value: "zh-TW" } });
    expect(await screen.findByLabelText("介面語言")).toHaveValue("zh-TW");
    expect(window.localStorage.getItem("bkuw.locale")).toBe("zh-TW");
  });
});
