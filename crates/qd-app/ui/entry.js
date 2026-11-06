// Renders search results (see qd_core::SearchResult) as HTML.
//
// Labels for parts of speech, tags and tenses are shown in the entry's own
// language: French entries get French tense names ("Passé simple"). The rest
// (headings, buttons) follows the interface language.

import { t } from "./i18n.js";

export function esc(text) {
  return String(text ?? "").replace(/[&<>"']/g, (c) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  })[c]);
}

const POS = {
  fr: {
    noun: "nom", verb: "verbe", adj: "adjectif", adv: "adverbe", name: "nom propre",
    pron: "pronom", prep: "préposition", conj: "conjonction", intj: "interjection",
    det: "déterminant", article: "article", num: "numéral", phrase: "locution",
    prefix: "préfixe", suffix: "suffixe", abbrev: "abréviation", particle: "particule",
  },
  en: {
    noun: "noun", verb: "verb", adj: "adjective", adv: "adverb", name: "proper noun",
    pron: "pronoun", prep: "preposition", conj: "conjunction", intj: "interjection",
    det: "determiner", article: "article", num: "numeral", phrase: "phrase",
    prefix: "prefix", suffix: "suffix", abbrev: "abbreviation", particle: "particle",
  },
};

const TAGS = {
  fr: {
    masculine: "masculin", feminine: "féminin", singular: "singulier", plural: "pluriel",
    invariable: "invariable", transitive: "transitif", intransitive: "intransitif",
    pronominal: "pronominal", impersonal: "impersonnel", figuratively: "figuré",
    familiar: "familier", informal: "familier", slang: "argot", archaic: "archaïque",
    obsolete: "désuet", dated: "vieilli", rare: "rare", literary: "littéraire",
    formal: "soutenu", vulgar: "vulgaire", alternative: "variante",
    "first-person": "1re personne", "second-person": "2e personne", "third-person": "3e personne",
    "masculine-feminine": "masculin et féminin",
    auxiliary: "auxiliaire", perfective: "perfectif", imperfective: "imperfectif",
    countable: "dénombrable", uncountable: "indénombrable",
  },
  en: {
    "third-person": "3rd person", "second-person": "2nd person", "first-person": "1st person",
  },
};

// French conjugation, laid out like a conjugation book: each simple tense is
// followed by its compound tense (présent / passé composé, imparfait /
// plus-que-parfait…), so the pairs line up in the columns. Tenses are keyed
// by the tags of their forms.
const FR_CONJUGATION = [
  ["Indicatif", [
    [["indicative present", "Présent"], ["indicative past multiword-construction", "Passé composé"]],
    [["indicative imperfect", "Imparfait"], ["indicative pluperfect", "Plus-que-parfait"]],
    [["indicative past", "Passé simple"], ["indicative past anterior", "Passé antérieur"]],
    [["indicative future", "Futur simple"], ["indicative future perfect", "Futur antérieur"]],
  ]],
  ["Subjonctif", [
    [["subjunctive present", "Présent"], ["subjunctive past", "Passé"]],
    [["subjunctive imperfect", "Imparfait"], ["subjunctive pluperfect", "Plus-que-parfait"]],
  ]],
  ["Conditionnel", [
    [["conditional present", "Présent"], ["conditional past", "Passé"]],
  ]],
  ["Impératif", [
    [["imperative present", "Présent"], ["imperative past", "Passé"]],
  ]],
  ["Participe", [
    [["participle present", "Présent"], ["participle past", "Passé"]],
  ]],
  ["Infinitif", [
    [["infinitive present", "Présent"], ["infinitive past", "Passé"]],
  ]],
  ["Gérondif", [
    [["gerund present", "Présent"], ["gerund past", "Passé"]],
  ]],
];

// Moods named with their tense in one phrase: "Participe passé".
const FR_IMPERSONAL = new Set(["Participe", "Infinitif", "Gérondif"]);

/** Every tense of a mood, simple and compound, as [tags, name]. */
const moodTenses = (rows) => rows.flat().filter(Boolean);

const EN_PRINCIPAL_PARTS = [
  ["present singular third-person", "3rd person singular"],
  ["participle present", "Present participle"],
  ["past", "Simple past"],
  ["participle past", "Past participle"],
];

// Tags are compared as sorted sets: "indicative present" == "present indicative".
const tagKey = (tags) => [...(tags ?? [])].sort().join(" ");
const key = (tags) => tagKey(tags.split(" "));

function tagLabel(lang, tag) {
  return TAGS[lang]?.[tag] ?? tag.replaceAll("-", " ");
}

function tagsLabel(lang, tags) {
  return tags.map((t) => tagLabel(lang, t)).join(" ");
}

function groupTags(group) {
  return [...(group.tags ?? []), ...(group.raw_tags ?? [])];
}

/** The tense name of a form's tags, e.g. "Indicatif · Imparfait", or
 * null when the tags are not a known tense. */
function knownTense(lang, tags) {
  if (lang === "fr") {
    const wanted = tagKey(tags);
    for (const [mood, rows] of FR_CONJUGATION) {
      for (const [k, name] of moodTenses(rows)) {
        if (key(k) === wanted) return FR_IMPERSONAL.has(mood) ? `${mood} ${name.toLowerCase()}` : `${mood} · ${name}`;
      }
    }
  }
  if (lang === "en") {
    const wanted = tagKey(tags);
    const part = EN_PRINCIPAL_PARTS.find(([k]) => key(k) === wanted);
    if (part) return part[1];
  }
  return null;
}

export function tenseLabel(lang, tags) {
  return knownTense(lang, tags) ?? tagsLabel(lang, tags);
}

// Same rules as qd-core's language::form_key, but keeping accents.
const FR_PRONOUNS = /^(?:que |qu'|je |j'|tu |il\/elle\/on |il |elle |on |nous |vous |ils\/elles |ils |elles |me |m'|te |t'|se |s')/;
const FR_SUFFIXES = /-(?:moi|toi|nous|vous|le|la|les|lui|leur|en|y)$/;

/** A form as typed: lowercase, without pronouns ("que nous allions" -> "allions"). */
function bareForm(form) {
  let bare = form.toLowerCase().replace(/[’‘ʼ]/g, "'").trim();
  for (let prev = null; prev !== bare; ) {
    prev = bare;
    bare = bare.replace(FR_PRONOUNS, "");
  }
  return bare.replace(FR_SUFFIXES, "");
}

/** Whether a form is the one the user looked up (in any tense). */
const isMatch = (form, matched) => matched != null && bareForm(form) === bareForm(matched);

function mark(form, matched) {
  return isMatch(form, matched) ? `<mark>${esc(form)}</mark>` : esc(form);
}

/** Splits forms into conjugation tenses (by tag key) and everything else. */
function splitForms(entry, keys) {
  const tenses = new Map();
  const other = [];
  for (const group of entry.forms) {
    const k = tagKey(group.tags);
    if (keys.has(k) && !group.raw_tags?.length && !tenses.has(k)) {
      tenses.set(k, group);
    } else {
      other.push(group);
    }
  }
  return { tenses, other };
}

function otherForms(lang, groups, matched, title) {
  if (!groups.length) return "";
  const rows = groups
    .map((g) => `<tr><th>${esc(tagsLabel(lang, groupTags(g)) || "—")}</th>
      <td>${g.forms.map((f) => mark(f, matched)).join(", ")}</td></tr>`)
    .join("");
  return `<h4>${esc(title)}</h4><table class="forms">${rows}</table>`;
}

/** A tense tile: its name and its forms, or an empty cell. */
function tenseTile(name, forms, isHit) {
  if (!forms) return `<div class="tense empty"></div>`;
  const hit = forms.some(isHit);
  const list = forms.map((f) => `<div class="form">${isHit(f) ? `<mark>${esc(f)}</mark>` : esc(f)}</div>`).join("");
  return `<div class="tense${hit ? " hit" : ""}"><div class="tense-name">${esc(name)}</div>${list}</div>`;
}

/** The moods of a conjugation, as [mood name, tiles html, tile count]. Moods
 * of one or two tenses take half the width, two side by side. */
function moodGrid(moods) {
  const html = moods
    .filter(([, tiles]) => tiles)
    .map(([mood, tiles, count]) => `<div class="mood${count <= 2 ? " half" : ""}">
      <div class="mood-name">${esc(mood)}</div><div class="tenses">${tiles}</div></div>`)
    .join("");
  return `<div class="moods">${html}</div>`;
}

function frenchConjugation(entry, matched) {
  const keys = new Set(FR_CONJUGATION.flatMap(([, rows]) => moodTenses(rows).map(([k]) => key(k))));
  const { tenses, other } = splitForms(entry, keys);
  if (tenses.size < 4) return null;

  const isHit = (f) => isMatch(f, matched);
  const tile = (tense) => tenseTile(tense?.[1], tense && tenses.get(key(tense[0]))?.forms, isHit);

  const aux = tenses.get(key("infinitive past"))?.forms[0]?.split(" ")[0];
  const auxLabel = aux ? ` <span class="faint">· ${esc(t("entry.auxiliary", { aux }))}</span>` : "";
  let html = `<h4>${esc(t("entry.conjugation"))}${auxLabel}</h4>`;
  html += moodGrid(FR_CONJUGATION.map(([mood, rows]) => {
    // A row shows when either of its tenses is known, so that simple and
    // compound tenses stay side by side.
    const shown = rows.filter((row) => row.some((tense) => tense && tenses.has(key(tense[0]))));
    return [mood, shown.map(([simple, compound]) => tile(simple) + tile(compound)).join(""), shown.length * 2];
  }));
  html += otherForms("fr", other, matched, t("entry.otherForms"));
  return html;
}

const EN_PERSONS = ["I", "you", "he/she/it", "we", "you", "they"];

// "Be" is the only verb whose present and past change with the person
// beyond the third person singular.
const EN_BE = {
  present: ["am", "are", "is", "are", "are", "are"],
  past: ["was", "were", "was", "were", "were", "were"],
};

/** The full English conjugation, built from the principal parts the way
 * online conjugators show it: each time (present, past, future, conditional)
 * in simple, continuous, perfect and perfect continuous forms. Null when a
 * principal part is missing (modal verbs such as "can"). */
function englishConjugation(entry, tenses) {
  const parts = Object.fromEntries(EN_PRINCIPAL_PARTS.map(([k]) => [k, tenses.get(key(k))?.forms]));
  const third = parts["present singular third-person"];
  const ing = parts["participle present"];
  const past = parts["past"];
  const pp = parts["participle past"];
  if (!third || !ing || !past || !pp) return null;

  // Spelling variants ("learned", "learnt") share one line.
  const [thirdV, ingV, pastV, ppV] = [third, ing, past, pp].map((forms) => forms.join(" / "));
  const base = entry.word;
  const be = base === "be";
  const persons = (verb) => EN_PERSONS.map((p, i) => `${p} ${typeof verb === "function" ? verb(i) : verb}`);

  const present = persons((i) => (be ? EN_BE.present[i] : i === 2 ? thirdV : base));
  const beNow = persons((i) => EN_BE.present[i]);
  const havePresent = persons((i) => (i === 2 ? "has" : "have"));
  const add = (forms, rest) => forms.map((f) => `${f} ${rest}`);

  const moods = [
    ["Indicative", [
      ["Present", present],
      ["Present continuous", add(beNow, ingV)],
      ["Present perfect", add(havePresent, ppV)],
      ["Present perfect continuous", add(havePresent, `been ${ingV}`)],
      ["Past", persons((i) => (be ? EN_BE.past[i] : pastV))],
      ["Past continuous", persons((i) => `${EN_BE.past[i]} ${ingV}`)],
      ["Past perfect", persons(`had ${ppV}`)],
      ["Past perfect continuous", persons(`had been ${ingV}`)],
      ["Future", persons(`will ${base}`)],
      ["Future continuous", persons(`will be ${ingV}`)],
      ["Future perfect", persons(`will have ${ppV}`)],
      ["Future perfect continuous", persons(`will have been ${ingV}`)],
    ]],
    ["Conditional", [
      ["Present", persons(`would ${base}`)],
      ["Continuous", persons(`would be ${ingV}`)],
      ["Perfect", persons(`would have ${ppV}`)],
      ["Perfect continuous", persons(`would have been ${ingV}`)],
    ]],
    ["Imperative", [
      ["Affirmative", [base, `let's ${base}`, base]],
      ["Negative", [`don't ${base}`, `let's not ${base}`, `don't ${base}`]],
    ]],
    ["Infinitive", [
      ["Present", [`to ${base}`]],
      ["Perfect", [`to have ${ppV}`]],
    ]],
    ["Participle", [
      ["Present", [ingV]],
      ["Past", [ppV]],
    ]],
  ];
  return moods;
}

/** Whether an English line ("he/she/it goes") is the form looked up. Only
 * simple tenses count: "gone" lights up the participle, not every perfect. */
function englishHit(line, matched) {
  if (matched == null) return false;
  const verb = line.replace(/^(?:I|you|he\/she\/it|we|they|to) /, "");
  return verb.split(" / ").some((v) => v.toLowerCase() === matched.toLowerCase());
}

function englishVerb(entry, matched) {
  const keys = new Set(EN_PRINCIPAL_PARTS.map(([k]) => key(k)));
  const { tenses, other } = splitForms(entry, keys);
  if (!tenses.size) return null;
  const moods = englishConjugation(entry, tenses);
  let html;
  if (moods) {
    // Principal parts in the heading, as in irregular verb lists: go – went – gone.
    const principal = [[entry.word], tenses.get(key("past")).forms, tenses.get(key("participle past")).forms]
      .map((forms) => forms.join("/"))
      .join(" – ");
    const isHit = (f) => englishHit(f, matched);
    html = `<h4>${esc(t("entry.conjugation"))} <span class="faint">· ${esc(principal)}</span></h4>`;
    html += moodGrid(moods.map(([mood, list]) => [
      mood,
      list.map(([name, forms]) => tenseTile(name, forms, isHit)).join(""),
      list.length,
    ]));
  } else {
    const rows = [["Infinitive", [entry.word]]]
      .concat(EN_PRINCIPAL_PARTS
        .filter(([k]) => tenses.has(key(k)))
        .map(([k, label]) => [label, tenses.get(key(k)).forms]))
      .map(([label, forms]) => `<tr><th>${esc(label)}</th><td>${forms.map((f) => mark(f, matched)).join(", ")}</td></tr>`)
      .join("");
    html = `<h4>${esc(t("entry.conjugation"))}</h4><table class="forms">${rows}</table>`;
  }
  if (other.length) {
    const summary = esc(t("entry.otherFormsCount", { n: other.length }));
    html += `<details><summary>${summary}</summary>${otherForms("en", other, matched, "")}</details>`;
  }
  return html;
}

function formsSection(result) {
  const { entry, lang } = result;
  if (!entry.forms.length) return "";
  const matched = result.matched_form?.form;
  if (entry.pos === "verb") {
    const html = lang === "fr" ? frenchConjugation(entry, matched) : lang === "en" ? englishVerb(entry, matched) : null;
    if (html) return `<section class="conjugation">${html}</section>`;
  }
  return `<section>${otherForms(lang, entry.forms, matched, t("entry.forms"))}</section>`;
}

const SENSES_SHOWN = 6;

function sensesSection(result) {
  const { entry, lang } = result;
  let previousContext = "";
  const items = entry.senses.map((sense, i) => {
    const gloss = sense.glosses[sense.glosses.length - 1];
    // Sub-senses share their parent's gloss: show it once per run.
    const fullContext = sense.glosses.slice(0, -1).map(esc).join(" › ");
    const context = fullContext === previousContext ? "" : fullContext;
    previousContext = fullContext;
    const tags = (sense.tags ?? [])
      .filter((t) => t !== "form-of" && t !== "alt-of")
      .map((t) => `<span class="chip">${esc(tagLabel(lang, t))}</span>`)
      .join("");
    const links = [...(sense.form_of ?? []), ...(sense.alt_of ?? [])]
      .map((w) => `<button class="link lookup-word" data-word="${esc(w)}">→ ${esc(w)}</button>`)
      .join(" ");
    const example = sense.examples?.length
      ? `<div class="example">${esc(sense.examples[0])}</div>`
      : "";
    return `<li class="${i >= SENSES_SHOWN ? "extra" : ""}">
      ${context ? `<div class="context">${context}</div>` : ""}
      ${tags}${esc(gloss)} ${links}${example}</li>`;
  });
  if (!items.length) return "";
  const more = entry.senses.length > SENSES_SHOWN
    ? `<button class="link toggle-senses">${esc(t("entry.showAllSenses", { n: entry.senses.length }))}</button>`
    : "";
  return `<ol class="senses">${items.join("")}</ol>${more}`;
}

function matchLine(result) {
  const { lang, entry } = result;
  if (result.matched_form) {
    // The same spelling can be several tenses ("allions": imparfait and
    // subjonctif présent): list them all.
    // Known tenses only, when there are some: English tables also list
    // archaic and dialectal rows.
    const matched = result.matched_form.form;
    const groups = entry.forms.filter((g) => g.forms.some((f) => isMatch(f, matched)));
    const known = groups.map((g) => knownTense(lang, groupTags(g))).filter(Boolean);
    const labels = [...new Set(known.length ? known : [tenseLabel(lang, result.matched_form.tags)].filter(Boolean))];
    return `<div class="match">↳ <strong>${esc(matched)}</strong>${labels.length ? ` — ${esc(labels.join(" ; "))}` : ""}</div>`;
  }
  if (result.note) return `<div class="match">↳ ${esc(result.note)}</div>`;
  return "";
}

function header(result) {
  const { entry, lang } = result;
  const pos = POS[lang]?.[entry.pos] ?? entry.pos;
  const tags = entry.tags.filter((t) => t !== "form-of" && t !== "alt-of").map((t) => tagLabel(lang, t));
  return `<div class="head">
    <button class="word copy" title="${esc(t("entry.copy"))}" data-word="${esc(entry.word)}">${esc(entry.word)}</button>
    <span class="pos">${esc([pos, ...tags].join(", "))}</span>
    ${entry.ipa ? `<span class="ipa">${esc(entry.ipa)}</span>` : ""}
    <span class="lang" title="${esc(result.lang_name)}">${esc(lang.toUpperCase())}</span>
  </div>`;
}

/** One result: compact (header, match and first definition) or expanded. */
export function renderResult(result, expanded) {
  if (!expanded) {
    const first = result.entry.senses[0];
    const gloss = first ? first.glosses[first.glosses.length - 1] : "";
    return `${header(result)}${matchLine(result)}<div class="summary">${esc(gloss)}</div>`;
  }
  return `${header(result)}${matchLine(result)}${sensesSection(result)}${formsSection(result)}`;
}
