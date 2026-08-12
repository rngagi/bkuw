import { Languages } from "lucide-react";
import { useTranslation } from "react-i18next";
import { setLocale } from "../../i18n";

export function LocaleSelect() {
  const { i18n, t } = useTranslation();
  const value = i18n.resolvedLanguage === "zh-TW" ? "zh-TW" : "en";
  return (
    <label className="locale-select">
      <Languages aria-hidden="true" size={16} />
      <span className="sr-only">{t("locale.label")}</span>
      <select
        aria-label={t("locale.label")}
        value={value}
        onChange={(event) => void setLocale(event.target.value as "en" | "zh-TW")}
      >
        <option value="en">{t("locale.en")}</option>
        <option value="zh-TW">{t("locale.zhTW")}</option>
      </select>
    </label>
  );
}
