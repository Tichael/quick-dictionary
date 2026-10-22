"""Per-language rules for turning inflected forms into lookup keys.

Forms are displayed as Wiktionary gives them ("je lance", "abats-toi"), but
users type the bare word ("lance"), so each language knows how to strip
subject pronouns and similar decorations from an already-normalized form.
"""

import re

# Form tags that are not real inflections: headword lines, table metadata.
SKIP_FORM_TAGS = {
    "canonical",
    "table-tags",
    "inflection-template",
    "class",
    "romanization",
}


def _french_key(form: str) -> str:
    key = form
    prefix = re.compile(
        r"^(?:que |qu'|"
        r"je |j'|tu |il/elle/on |il |elle |on |nous |vous |ils/elles |ils |elles |"
        r"me |m'|te |t'|se |s')"
    )
    while True:
        stripped = prefix.sub("", key, count=1)
        if stripped == key:
            break
        key = stripped
    return re.sub(r"-(?:moi|toi|nous|vous|le|la|les|lui|leur|en|y)$", "", key)


def _identity_key(form: str) -> str:
    return form


LANGUAGES = {
    "fr": {"name": "Français", "form_key": _french_key},
    "en": {"name": "English", "form_key": _identity_key},
}
