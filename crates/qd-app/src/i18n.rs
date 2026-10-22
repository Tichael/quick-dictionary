//! Text shown by the app itself (tray menu, dialogs, error messages), in
//! each interface language. The windows' own text is in `ui/i18n.js`.

use std::fmt::Display;
use std::path::Path;

use crate::settings::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Fr,
}

impl Lang {
    /// The language to use for a setting: "same as the system" follows the
    /// system's display language, and falls back to English.
    pub fn resolve(setting: Language) -> Lang {
        match setting {
            Language::En => Lang::En,
            Language::Fr => Lang::Fr,
            Language::System => match sys_locale::get_locale() {
                Some(locale) if locale.to_lowercase().starts_with("fr") => Lang::Fr,
                _ => Lang::En,
            },
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Fr => "fr",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Text {
    TrayLookup,
    TraySettings,
    TrayQuit,
    SettingsTitle,
    AddPackTitle,
    PackFilter,
    RemoveTitle,
    RemoveButton,
    Cancel,
}

/// The match is exhaustive, so a missing translation does not compile.
pub fn text(lang: Lang, text: Text) -> &'static str {
    use Text::*;
    match (lang, text) {
        (Lang::En, TrayLookup) => "Look up a word",
        (Lang::En, TraySettings) => "Settings…",
        (Lang::En, TrayQuit) => "Quit",
        (Lang::En, SettingsTitle) => "Quick Dictionary — Settings",
        (Lang::En, AddPackTitle) => "Add a language pack",
        (Lang::En, PackFilter) => "Language packs",
        (Lang::En, RemoveTitle) => "Remove a dictionary",
        (Lang::En, RemoveButton) => "Remove",
        (Lang::En, Cancel) => "Cancel",
        (Lang::Fr, TrayLookup) => "Chercher un mot",
        (Lang::Fr, TraySettings) => "Paramètres…",
        (Lang::Fr, TrayQuit) => "Quitter",
        (Lang::Fr, SettingsTitle) => "Quick Dictionary — Paramètres",
        (Lang::Fr, AddPackTitle) => "Ajouter un fichier de langue",
        (Lang::Fr, PackFilter) => "Fichiers de langue",
        (Lang::Fr, RemoveTitle) => "Supprimer un dictionnaire",
        (Lang::Fr, RemoveButton) => "Supprimer",
        (Lang::Fr, Cancel) => "Annuler",
    }
}

pub fn remove_question(lang: Lang, file: &str) -> String {
    match lang {
        Lang::En => format!("Remove the dictionary {file}? The file will be deleted."),
        Lang::Fr => format!("Supprimer le dictionnaire {file} ? Le fichier sera effacé."),
    }
}

pub fn shortcut_error(lang: Lang, shortcut: &str, error: impl Display) -> String {
    match lang {
        Lang::En => format!("Could not use the shortcut “{shortcut}”: {error}"),
        Lang::Fr => format!("Impossible d’utiliser le raccourci « {shortcut} » : {error}"),
    }
}

pub fn autostart_error(lang: Lang, error: impl Display) -> String {
    match lang {
        Lang::En => format!("Could not change “start with the system”: {error}"),
        Lang::Fr => format!("Impossible de modifier « Lancer au démarrage » : {error}"),
    }
}

pub fn save_error(lang: Lang, error: impl Display) -> String {
    match lang {
        Lang::En => format!("Could not save the settings: {error}"),
        Lang::Fr => format!("Impossible d’enregistrer les paramètres : {error}"),
    }
}

pub fn not_a_pack(lang: Lang, path: &Path, error: impl Display) -> String {
    let path = path.display();
    match lang {
        Lang::En => format!("{path} is not a usable language pack: {error}"),
        Lang::Fr => format!("{path} n’est pas un fichier de langue utilisable : {error}"),
    }
}

pub fn install_error(lang: Lang, path: &Path, error: impl Display) -> String {
    let path = path.display();
    match lang {
        Lang::En => format!("Could not install {path}: {error}"),
        Lang::Fr => format!("Impossible d’installer {path} : {error}"),
    }
}

pub fn not_installed(lang: Lang, file: &str) -> String {
    match lang {
        Lang::En => format!("{file} is not an installed dictionary"),
        Lang::Fr => format!("{file} n’est pas un dictionnaire installé"),
    }
}

pub fn remove_error(lang: Lang, file: &str, error: impl Display) -> String {
    match lang {
        Lang::En => format!("Could not remove {file}: {error}"),
        Lang::Fr => format!("Impossible de supprimer {file} : {error}"),
    }
}

pub fn manifest_error(lang: Lang, error: impl Display) -> String {
    match lang {
        Lang::En => format!("Could not get the list of dictionaries to download: {error}"),
        Lang::Fr => {
            format!("Impossible d’obtenir la liste des dictionnaires à télécharger : {error}")
        }
    }
}

pub fn download_error(lang: Lang, name: &str, error: impl Display) -> String {
    match lang {
        Lang::En => format!("Could not download the {name} dictionary: {error}"),
        Lang::Fr => format!("Impossible de télécharger le dictionnaire {name} : {error}"),
    }
}

pub fn checksum_error(lang: Lang, name: &str) -> String {
    match lang {
        Lang::En => format!("The {name} dictionary was damaged during the download. Try again."),
        Lang::Fr => {
            format!("Le dictionnaire {name} a été abîmé pendant le téléchargement. Réessayez.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_settings_win_over_the_system() {
        assert_eq!(Lang::resolve(Language::En), Lang::En);
        assert_eq!(Lang::resolve(Language::Fr), Lang::Fr);
    }
}
