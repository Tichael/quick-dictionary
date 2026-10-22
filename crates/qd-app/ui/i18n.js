// Interface text in each supported language. To add a language, add a
// block with the same keys here and in src/i18n.rs, and an option in the
// settings window.
//
// In the HTML, data-i18n="key" sets an element's text, data-i18n-html its
// HTML (trusted strings from this file only), and data-i18n-placeholder /
// data-i18n-title those attributes. In scripts, t("key", { name }) fills
// "{name}" placeholders.

const STRINGS = {
  en: {
    // Lookup window
    "lookup.placeholder": "Type a word…",
    "lookup.settings": "Settings",
    "lookup.hide": "Hide (Esc)",
    "lookup.noPacks": "No dictionary is installed yet. <button class=\"link\" id=\"open-dictionaries\">Add one in the settings</button>.",
    "lookup.loading": "Opening dictionaries…",
    "lookup.noMatch": "No match for “{query}”.",
    "lookup.didYouMean": "No exact match for “{query}”. Did you mean {words}?",
    "lookup.copied": "Copied “{word}”",
    "lookup.copyFailed": "Could not copy",

    // Entries
    "entry.copy": "Copy",
    "entry.conjugation": "Conjugation",
    "entry.auxiliary": "auxiliary {aux}",
    "entry.showCompound": "Show compound tenses",
    "entry.forms": "Forms",
    "entry.otherForms": "Other forms",
    "entry.otherFormsCount": "Other forms ({n})",
    "entry.showAllSenses": "Show all {n} definitions",

    // Settings window
    "settings.title": "Quick Dictionary — Settings",
    "nav.general": "General",
    "nav.behavior": "Behavior",
    "nav.dictionaries": "Dictionaries",
    "nav.about": "About",
    "general.title": "General",
    "general.shortcut": "Shortcut",
    "general.shortcutHint": "Opens the lookup window from anywhere. Press it again to hide the window.",
    "general.shortcutButton": "Click, then press the new shortcut",
    "general.shortcutRecording": "Press a shortcut…",
    "general.shortcutNeedsModifier": "Add Ctrl, Alt or Win…",
    "general.startWithSystem": "Start with the system",
    "general.startWithSystemHint": "Runs Quick Dictionary in the tray when you log in, so the shortcut always works.",
    "general.language": "Interface language",
    "general.languageHint": "Definitions and conjugations stay in each word's own language.",
    "general.languageSystem": "Same as the system",
    "behavior.title": "Behavior",
    "behavior.hideOnBlur": "Hide when the window loses focus",
    "behavior.hideOnBlurHint": "Clicking elsewhere closes the lookup window, like a menu.",
    "behavior.clearOnShow": "Start with an empty search",
    "behavior.clearOnShowHint": "Otherwise the last word stays, selected, so typing replaces it.",
    "behavior.position": "Window position",
    "behavior.positionCenter": "Center of the screen",
    "behavior.positionRemember": "Where I last moved it",
    "behavior.theme": "Theme",
    "behavior.themeSystem": "Same as the system",
    "behavior.themeLight": "Light",
    "behavior.themeDark": "Dark",
    "behavior.maxResults": "Results per search",
    "dictionaries.title": "Dictionaries",
    "dictionaries.hint": "Every installed dictionary is searched at once. Download them below, or add a language pack file (<code>fr.db</code>, <code>en.db</code>…) built with the tools in the project's <code>pipeline</code> folder.",
    "dictionaries.add": "Add a dictionary file…",
    "dictionaries.openFolder": "Open the dictionaries folder",
    "dictionaries.reload": "Reload",
    "dictionaries.folder": "Folder: {dir}",
    "dictionaries.loading": "Opening dictionaries…",
    "dictionaries.packInfo": "{entries} entries · {size} · built {date} · {file}",
    "dictionaries.remove": "Remove",
    "dictionaries.none": "No dictionary installed. Download one below to start looking words up.",
    "dictionaries.installing": "Installing… (this can take a few seconds for a large dictionary)",
    "dictionaries.reloading": "Reloading…",
    "dictionaries.availableTitle": "Available to download",
    "dictionaries.checking": "Looking for dictionaries to download…",
    "dictionaries.checkAgain": "Check again",
    "dictionaries.noneAvailable": "No dictionaries are published yet.",
    "dictionaries.remoteInfo": "{entries} entries · {size} to download · built {date}",
    "dictionaries.download": "Download",
    "dictionaries.update": "Update",
    "dictionaries.upToDate": "Up to date",
    "dictionaries.downloading": "Downloading… {percent}%",
    "dictionaries.installingDownload": "Installing…",
    "size.mb": "{n} MB",
    "size.gb": "{n} GB",
    "about.title": "About",
    "about.version": "Quick Dictionary {version} — an offline dictionary and conjugator.",
    "about.licenses": "The code is MIT-licensed. Dictionary text comes from <a href=\"https://www.wiktionary.org/\">Wiktionary</a> contributors, via <a href=\"https://kaikki.org/\">wiktextract and kaikki.org</a>; word frequencies come from <a href=\"https://github.com/rspeer/wordfreq\">wordfreq</a>. Both are licensed <a href=\"https://creativecommons.org/licenses/by-sa/4.0/\">CC BY-SA 4.0</a>.",
  },

  fr: {
    "lookup.placeholder": "Tapez un mot…",
    "lookup.settings": "Paramètres",
    "lookup.hide": "Masquer (Échap)",
    "lookup.noPacks": "Aucun dictionnaire n’est installé. <button class=\"link\" id=\"open-dictionaries\">Ajoutez-en un dans les paramètres</button>.",
    "lookup.loading": "Ouverture des dictionnaires…",
    "lookup.noMatch": "Aucun résultat pour « {query} ».",
    "lookup.didYouMean": "Aucun résultat exact pour « {query} ». Vouliez-vous dire {words} ?",
    "lookup.copied": "« {word} » copié",
    "lookup.copyFailed": "Impossible de copier",

    "entry.copy": "Copier",
    "entry.conjugation": "Conjugaison",
    "entry.auxiliary": "auxiliaire {aux}",
    "entry.showCompound": "Afficher les temps composés",
    "entry.forms": "Formes",
    "entry.otherForms": "Autres formes",
    "entry.otherFormsCount": "Autres formes ({n})",
    "entry.showAllSenses": "Voir les {n} définitions",

    "settings.title": "Quick Dictionary — Paramètres",
    "nav.general": "Général",
    "nav.behavior": "Comportement",
    "nav.dictionaries": "Dictionnaires",
    "nav.about": "À propos",
    "general.title": "Général",
    "general.shortcut": "Raccourci",
    "general.shortcutHint": "Ouvre la fenêtre de recherche depuis n’importe où. Appuyez de nouveau pour la masquer.",
    "general.shortcutButton": "Cliquez, puis appuyez sur le nouveau raccourci",
    "general.shortcutRecording": "Appuyez sur un raccourci…",
    "general.shortcutNeedsModifier": "Ajoutez Ctrl, Alt ou Win…",
    "general.startWithSystem": "Lancer au démarrage",
    "general.startWithSystemHint": "Lance Quick Dictionary dans la zone de notification à l’ouverture de session, pour que le raccourci fonctionne toujours.",
    "general.language": "Langue de l’interface",
    "general.languageHint": "Les définitions et les conjugaisons restent dans la langue de chaque mot.",
    "general.languageSystem": "Celle du système",
    "behavior.title": "Comportement",
    "behavior.hideOnBlur": "Masquer quand la fenêtre perd le focus",
    "behavior.hideOnBlurHint": "Un clic ailleurs ferme la fenêtre de recherche, comme un menu.",
    "behavior.clearOnShow": "Commencer par une recherche vide",
    "behavior.clearOnShowHint": "Sinon, le dernier mot reste affiché et sélectionné : il suffit de taper pour le remplacer.",
    "behavior.position": "Position de la fenêtre",
    "behavior.positionCenter": "Au centre de l’écran",
    "behavior.positionRemember": "Là où je l’ai déplacée",
    "behavior.theme": "Thème",
    "behavior.themeSystem": "Celui du système",
    "behavior.themeLight": "Clair",
    "behavior.themeDark": "Sombre",
    "behavior.maxResults": "Résultats par recherche",
    "dictionaries.title": "Dictionnaires",
    "dictionaries.hint": "Tous les dictionnaires installés sont consultés en même temps. Téléchargez-les ci-dessous, ou ajoutez un fichier de langue (<code>fr.db</code>, <code>en.db</code>…) créé avec les outils du dossier <code>pipeline</code> du projet.",
    "dictionaries.add": "Ajouter un fichier de dictionnaire…",
    "dictionaries.openFolder": "Ouvrir le dossier des dictionnaires",
    "dictionaries.reload": "Recharger",
    "dictionaries.folder": "Dossier : {dir}",
    "dictionaries.loading": "Ouverture des dictionnaires…",
    "dictionaries.packInfo": "{entries} entrées · {size} · créé le {date} · {file}",
    "dictionaries.remove": "Supprimer",
    "dictionaries.none": "Aucun dictionnaire installé. Téléchargez-en un ci-dessous pour commencer à chercher des mots.",
    "dictionaries.installing": "Installation… (quelques secondes pour un gros dictionnaire)",
    "dictionaries.reloading": "Rechargement…",
    "dictionaries.availableTitle": "Disponibles au téléchargement",
    "dictionaries.checking": "Recherche des dictionnaires à télécharger…",
    "dictionaries.checkAgain": "Vérifier de nouveau",
    "dictionaries.noneAvailable": "Aucun dictionnaire n’est encore publié.",
    "dictionaries.remoteInfo": "{entries} entrées · {size} à télécharger · créé le {date}",
    "dictionaries.download": "Télécharger",
    "dictionaries.update": "Mettre à jour",
    "dictionaries.upToDate": "À jour",
    "dictionaries.downloading": "Téléchargement… {percent} %",
    "dictionaries.installingDownload": "Installation…",
    "size.mb": "{n} Mo",
    "size.gb": "{n} Go",
    "about.title": "À propos",
    "about.version": "Quick Dictionary {version} — un dictionnaire et conjugueur hors ligne.",
    "about.licenses": "Le code est sous licence MIT. Le texte des dictionnaires provient des contributeurs du <a href=\"https://fr.wiktionary.org/\">Wiktionnaire</a>, via <a href=\"https://kaikki.org/\">wiktextract et kaikki.org</a> ; les fréquences des mots proviennent de <a href=\"https://github.com/rspeer/wordfreq\">wordfreq</a>. Les deux sont sous licence <a href=\"https://creativecommons.org/licenses/by-sa/4.0/deed.fr\">CC BY-SA 4.0</a>.",
  },
};

let lang = "en";

/** The interface language now in use ("en", "fr"). */
export function uiLang() {
  return lang;
}

/** Looks up a string, filling "{name}" placeholders from `vars`. */
export function t(key, vars = {}) {
  const text = STRINGS[lang]?.[key] ?? STRINGS.en[key] ?? key;
  return text.replace(/\{(\w+)\}/g, (match, name) => (name in vars ? String(vars[name]) : match));
}

/** Sets the language and translates the page's marked elements. */
export function setLanguage(code) {
  lang = code in STRINGS ? code : "en";
  document.documentElement.lang = lang;
  for (const el of document.querySelectorAll("[data-i18n]")) el.textContent = t(el.dataset.i18n);
  for (const el of document.querySelectorAll("[data-i18n-html]")) el.innerHTML = t(el.dataset.i18nHtml);
  for (const el of document.querySelectorAll("[data-i18n-placeholder]")) el.placeholder = t(el.dataset.i18nPlaceholder);
  for (const el of document.querySelectorAll("[data-i18n-title]")) el.title = t(el.dataset.i18nTitle);
}

/** Asks the app which language to use (it resolves "same as the system"). */
export async function loadLanguage() {
  const { invoke } = window.__TAURI__.core;
  setLanguage(await invoke("ui_language").catch(() => navigator.language.slice(0, 2)));
}

export const LANGUAGES = Object.keys(STRINGS);

/** Every key of every language, for tests. */
export function keysByLanguage() {
  return Object.fromEntries(Object.entries(STRINGS).map(([code, strings]) => [code, Object.keys(strings)]));
}
