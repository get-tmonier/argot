# ID: app/utils/language.ts:41
export async function applyLocale(
  locale: string | null | undefined,
  instance: i18n
) {
  // The database stores locales as en_US, but i18next wants en-US
  const localeBCP = locale ? unicodeCLDRtoBCP47(locale) : undefined;

  if (localeBCP && instance.languages?.[0] !== localeBCP) {
    await instance.changeLanguage(localeBCP);
    await Desktop.bridge?.setSpellCheckerLanguages(["en-US", localeBCP]);
  }

  if (typeof document !== "undefined") {
    document.documentElement.dir = isRTLLanguage(locale) ? "rtl" : "ltr";
  }
}
