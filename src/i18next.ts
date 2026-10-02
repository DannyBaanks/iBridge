import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import LanguageDetector from "i18next-browser-languagedetector";

const languages = [
  ["az", "Azərbaycan"],
  ["en", "English"],
  ["el", "Ελληνικά"],
  ["am", "Հայերեն"],
  ["es", "Español"],
  ["it", "Italiano"],
  ["de", "Deutsch"],
  ["de_ch", "Schweizerdeutsch"],
  ["fr", "Français"],
  ["pl", "Polski"],
  ["nl", "Nederlands"],
  ["vi", "Tiếng Việt"],
  ["ru", "Русский"],
  ["ro", "Română"],
  ["ar", "العربية"],
  ["tr", "Türkçe"],
  ["zh_tw", "Traditional Chinese （繁體中文)"],
  ["zh_cn", "Simpified Chinese （简体中文)"],
  ["ko", "한국어"],
  ["zh_hk", "Cantonese （粵語)"],
  ["ja", "日本語"],
  ["cs_cz", "Čeština"],
  ["sv", "Svenska"],
  ["hu", "Magyar"],
  ["kh", "ភាសាខ្មែរ"],
  ["id", "Bahasa Indonesia"],
  ["pt_br", "Português (Brasileiro)"],
] as const;

export const sortedLanguages = [...languages].sort((a, b) =>
  a[0].localeCompare(b[0]),
);

type TranslationResource = Record<string, unknown>;
type ResourceMap = Record<string, { translation: TranslationResource }>;

const localeModules = import.meta.glob<{ default: TranslationResource }>(
  "./locales/*.json",
  {
    eager: true,
  },
);

const resources = Object.fromEntries(
  Object.entries(localeModules).flatMap(([path, module]) => {
    const lang = path.match(/\/([\w-]+)\.json$/)?.[1];
    if (!lang) return [];

    return [[lang, { translation: module.default }]];
  }),
) as ResourceMap;

const iBridgeCopy = {
  en: {
    app: { ibridge_mobile: "Install iBridge Mobile" },
    operations: {
      install_ibridge_mobile_title: "Installing iBridge Mobile",
      install_ibridge_mobile_success_title: "iBridge Mobile Installed!",
      install_ibridge_mobile_success_message:
        "Open iBridge on your iPhone. Pairing and account bootstrap are already in place.",
      install_ibridge_mobile_step_download: "Download iBridge Mobile",
      install_ibridge_mobile_step_install: "Sign & Install iBridge Mobile",
      install_ibridge_mobile_step_bootstrap: "Configure This iPhone",
      mobile_sideload_title: "Installing IPA",
      mobile_sideload_step_install: "Sign & Install IPA",
      mobile_refresh_title: "Refreshing iBridge",
      mobile_refresh_success_title: "iBridge Refreshed!",
      mobile_refresh_step_download: "Download iBridge Mobile",
      mobile_refresh_step_install: "Sign & Refresh iBridge",
    },
  },
  es: {
    app: { ibridge_mobile: "Instalar iBridge Mobile" },
    operations: {
      install_ibridge_mobile_title: "Instalando iBridge Mobile",
      install_ibridge_mobile_success_title: "¡iBridge Mobile instalado!",
      install_ibridge_mobile_success_message:
        "Abre iBridge en tu iPhone. El pairing y la cuenta ya quedaron configurados.",
      install_ibridge_mobile_step_download: "Descargar iBridge Mobile",
      install_ibridge_mobile_step_install: "Firmar e instalar iBridge Mobile",
      install_ibridge_mobile_step_bootstrap: "Configurar este iPhone",
      mobile_sideload_title: "Instalando IPA",
      mobile_sideload_step_install: "Firmar e instalar IPA",
      mobile_refresh_title: "Refrescando iBridge",
      mobile_refresh_success_title: "¡iBridge refrescado!",
      mobile_refresh_step_download: "Descargar iBridge Mobile",
      mobile_refresh_step_install: "Firmar y refrescar iBridge",
    },
  },
} satisfies Record<string, Record<string, Record<string, string>>>;

for (const [language, sections] of Object.entries(iBridgeCopy)) {
  const translation = resources[language]?.translation;
  if (!translation) continue;
  for (const [section, copy] of Object.entries(sections)) {
    translation[section] = {
      ...((translation[section] as Record<string, unknown> | undefined) ?? {}),
      ...copy,
    };
  }
}

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    fallbackLng: "en",
    interpolation: {
      escapeValue: false,
    },
    resources,
  });

export default i18n;
