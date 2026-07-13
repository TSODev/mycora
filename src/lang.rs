use crate::app::Mode;

/// Which language the TUI renders its interface in — labels, hints,
/// prompts, and status messages. Keybindings and command names (`:tags`,
/// `:export`, ...) are identical in every language, same as vim's `:w`
/// doesn't translate: they're interface *syntax*, not interface text, and
/// keeping them fixed means every keybinding reference, script, and
/// muscle memory works regardless of language.
///
/// Both languages are embedded in the binary rather than loaded from
/// external language files: every message here is a real `format!` call
/// checked at compile time, so a missing key or a typo'd placeholder is a
/// compile error instead of a runtime surprise — and the binary stays
/// self-contained (no files to install alongside it, nothing to fail to
/// parse at startup). The cost is that adding a language means
/// recompiling; an optional override file can be layered on later if
/// out-of-tree translations ever matter more than that guarantee.
///
/// Selected by `language = "fr"` in `config.toml` (see `Config`) —
/// English is the default. TUI-only for now: CLI output (`mycora vault
/// list`, reindex reports, load warnings) stays English, matching the
/// language of the on-disk formats and docs it quotes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    Fr,
}

impl Lang {
    /// Parses a `config.toml` `language` value. `None` for anything
    /// unrecognized — the caller decides whether that's an error
    /// (`Config::load` refuses, so a typo'd `language = "fe"` is caught
    /// loudly at startup rather than silently falling back to English).
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "en" => Some(Lang::En),
            "fr" => Some(Lang::Fr),
            _ => None,
        }
    }

    /// The config-file code for this language — `from_code`'s inverse,
    /// what `:lang` writes back into `config.toml`.
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Fr => "fr",
        }
    }

    /// `(syntax, description)` pairs for every command `execute_command`
    /// recognizes — rendered by `ui.rs`'s command-palette help popup.
    /// Command syntax is identical across languages (see the type-level
    /// doc comment); only the descriptions translate.
    pub fn command_reference(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Lang::En => &[
                (":reindex", "rebuild the search index"),
                (
                    ":tags <tag1,tag2,...>",
                    "list notes matching any of the given tags",
                ),
                (":tags list", "list every known tag, pick one to filter by"),
                (
                    ":tags limit <vault-name>",
                    "restrict :tags/:tags list to one mounted vault",
                ),
                (":tags unlimit", "lift a :tags limit, back to every mounted vault"),
                (":panes reset", "reset pane widths to the default 40/40/20"),
                (
                    ":export <path>",
                    "flatten the selected note's subtree to a Markdown file",
                ),
                (
                    ":config unmount <show|hide>",
                    "show/hide unmounted vault rows in the tree",
                ),
                (
                    ":config archive <show|hide>",
                    "show/hide archived vault rows in the tree",
                ),
                (":tag add <tag>", "add a tag to the selected note"),
                (":tag del <tag>", "remove a tag from the selected note"),
                (":lang <en|fr>", "switch the interface language (persists)"),
                (":q, :quit", "quit Mycora"),
            ],
            Lang::Fr => &[
                (":reindex", "reconstruit l'index de recherche"),
                (
                    ":tags <tag1,tag2,...>",
                    "liste les notes portant l'un des tags donnés",
                ),
                (":tags list", "liste tous les tags connus, en choisir un pour filtrer"),
                (
                    ":tags limit <vault-name>",
                    "restreint :tags/:tags list à un seul vault monté",
                ),
                (":tags unlimit", "lève la limite de :tags, retour à tous les vaults"),
                (":panes reset", "réinitialise les largeurs de panneaux à 40/40/20"),
                (
                    ":export <path>",
                    "aplatit le sous-arbre de la note sélectionnée en Markdown",
                ),
                (
                    ":config unmount <show|hide>",
                    "affiche/masque les vaults non montés dans l'arbre",
                ),
                (
                    ":config archive <show|hide>",
                    "affiche/masque les vaults archivés dans l'arbre",
                ),
                (":tag add <tag>", "ajoute un tag à la note sélectionnée"),
                (":tag del <tag>", "retire un tag de la note sélectionnée"),
                (":lang <en|fr>", "change la langue de l'interface (persistant)"),
                (":q, :quit", "quitte Mycora"),
            ],
        }
    }

    // ------------------------------------------------------------------
    // ui.rs — pane titles, badges, prompts, hint rows
    // ------------------------------------------------------------------

    /// The `(3 links)` badge on a collapsed branch.
    pub fn links_badge(self, count: i64) -> String {
        match self {
            Lang::En => {
                let plural = if count == 1 { "" } else { "s" };
                format!("({count} link{plural})")
            }
            Lang::Fr => {
                let plural = if count == 1 { "" } else { "s" };
                format!("({count} lien{plural})")
            }
        }
    }

    /// The `(3 notes)` count next to a tag in `:tags list`.
    pub fn notes_badge(self, count: i64) -> String {
        let plural = if count == 1 { "" } else { "s" };
        match self {
            Lang::En => format!("({count} note{plural})"),
            Lang::Fr => format!("({count} note{plural})"),
        }
    }

    /// Body-preview text for an unmounted vault's placeholder row. The
    /// `mycora vault mount` command line inside it never translates —
    /// it's meant to be copied and run verbatim.
    pub fn unmounted_vault_help(self, name: &str, path: &str) -> String {
        match self {
            Lang::En => format!(
                "Vault \"{name}\" is unmounted.\n\nPath: {path}\n\nTo activate it:\n  mycora vault mount {name}"
            ),
            Lang::Fr => format!(
                "Le vault \"{name}\" n'est pas monté.\n\nChemin : {path}\n\nPour l'activer :\n  mycora vault mount {name}"
            ),
        }
    }

    /// Body-preview text for an archived vault's placeholder row.
    pub fn archived_vault_help(self, name: &str, archive_path: &str) -> String {
        match self {
            Lang::En => format!(
                "Vault \"{name}\" is archived.\n\nArchive: {archive_path}\n\nTo restore it:\n  mycora vault unarchive {name}"
            ),
            Lang::Fr => format!(
                "Le vault \"{name}\" est archivé.\n\nArchive : {archive_path}\n\nPour le restaurer :\n  mycora vault unarchive {name}"
            ),
        }
    }

    pub fn backlinks_title(self) -> &'static str {
        match self {
            Lang::En => "Backlinks",
            Lang::Fr => "Rétroliens",
        }
    }

    pub fn search_title(self, scope: &str, query: &str) -> String {
        match self {
            Lang::En => format!("Search [{scope}]: {query}"),
            Lang::Fr => format!("Recherche [{scope}] : {query}"),
        }
    }

    pub fn tag_results_title(self, scope: &str) -> String {
        match self {
            Lang::En => format!("Tag results [{scope}]"),
            Lang::Fr => format!("Résultats tags [{scope}]"),
        }
    }

    pub fn tag_list_title(self, scope: &str) -> String {
        match self {
            Lang::En => format!("Tags [{scope}]"),
            Lang::Fr => format!("Tags [{scope}]"),
        }
    }

    /// What the tag overlays' titles show when no `:tags limit` is active.
    pub fn all_vaults_label(self) -> &'static str {
        match self {
            Lang::En => "all vaults",
            Lang::Fr => "tous les vaults",
        }
    }

    pub fn commands_title(self) -> &'static str {
        match self {
            Lang::En => "Commands",
            Lang::Fr => "Commandes",
        }
    }

    /// The `y/n` delete confirmation prompt. The `y/n` keys themselves
    /// don't translate (they're keybindings — see the type-level doc
    /// comment), so the prompt spells them out as-is in both languages.
    pub fn delete_prompt(self, title: &str, descendants: usize) -> String {
        match self {
            Lang::En => {
                if descendants > 0 {
                    format!("Delete '{title}' and its {descendants} descendant(s)? y/n")
                } else {
                    format!("Delete '{title}'? y/n")
                }
            }
            Lang::Fr => {
                if descendants > 0 {
                    format!("Supprimer '{title}' et ses {descendants} descendant(s) ? y/n")
                } else {
                    format!("Supprimer '{title}' ? y/n")
                }
            }
        }
    }

    /// Fallback for `delete_prompt`'s title when the pending note can't
    /// be resolved.
    pub fn this_note(self) -> &'static str {
        match self {
            Lang::En => "this note",
            Lang::Fr => "cette note",
        }
    }

    pub fn press_q_again(self) -> &'static str {
        match self {
            Lang::En => "Press q again to quit",
            Lang::Fr => "Appuyez encore sur q pour quitter",
        }
    }

    /// The bold prefix on `last_error` in the hint row.
    pub fn error_prefix(self) -> &'static str {
        match self {
            Lang::En => "ERROR",
            Lang::Fr => "ERREUR",
        }
    }

    /// `(mode label, hint string)` for the status bar's hint row. Hint
    /// strings keep `ui.rs`'s parser convention — `key: label` tokens,
    /// double-space separated — and the *key* half of each token is
    /// identical across languages (it's what `disabled_keys` matches on,
    /// and it names real keys); only the labels translate.
    pub fn mode_line(self, mode: Mode) -> (&'static str, &'static str) {
        match (self, mode) {
            (Lang::En, Mode::Normal) => (
                "NORMAL",
                "j/k: move  h/l/space: fold  a/o: new  y: copy  Tab/S-Tab: move  \
                 K/J: reorder  i: rename  e: edit  d: delete  u: undo  ^R: redo  \
                 /: search  b: backlinks  [/]: tree width  {/}: backlinks width  \
                 colon: command  q: quit",
            ),
            (Lang::Fr, Mode::Normal) => (
                "NORMAL",
                "j/k: bouger  h/l/space: plier  a/o: nouvelle  y: copier  Tab/S-Tab: déplacer  \
                 K/J: réordonner  i: renommer  e: éditer  d: supprimer  u: annuler  ^R: rétablir  \
                 /: rechercher  b: rétroliens  [/]: largeur arbre  {/}: largeur rétroliens  \
                 colon: commande  q: quitter",
            ),
            (Lang::En, Mode::Insert) => ("INSERT", "Enter: confirm  Esc: cancel"),
            (Lang::Fr, Mode::Insert) => ("INSERTION", "Enter: valider  Esc: annuler"),
            (Lang::En, Mode::Search) => (
                "SEARCH",
                "type: filter  Up/Down: move  Enter: open  Esc: cancel",
            ),
            (Lang::Fr, Mode::Search) => (
                "RECHERCHE",
                "taper: filtrer  Up/Down: bouger  Enter: ouvrir  Esc: annuler",
            ),
            (Lang::En, Mode::Backlinks) => (
                "BACKLINKS",
                "j/k: move  Enter: jump  Esc/b: back to tree",
            ),
            (Lang::Fr, Mode::Backlinks) => (
                "RÉTROLIENS",
                "j/k: bouger  Enter: sauter  Esc/b: retour à l'arbre",
            ),
            (Lang::En, Mode::EditBody) => ("EDIT BODY", "Esc: save & exit"),
            (Lang::Fr, Mode::EditBody) => ("ÉDITION", "Esc: sauver & quitter"),
            (Lang::En, Mode::TagResults) => {
                ("TAG RESULTS", "j/k: move  Enter: open  Esc: cancel")
            }
            (Lang::Fr, Mode::TagResults) => {
                ("RÉSULTATS TAGS", "j/k: bouger  Enter: ouvrir  Esc: annuler")
            }
            (Lang::En, Mode::TagList) => ("TAGS", "j/k: move  Enter: filter  Esc: cancel"),
            (Lang::Fr, Mode::TagList) => ("TAGS", "j/k: bouger  Enter: filtrer  Esc: annuler"),
            (_, Mode::ConfirmDelete | Mode::Command) => {
                unreachable!("ConfirmDelete/Command render their own prompt row, not hints")
            }
        }
    }

    /// The breadcrumb row's right-aligned status markers, and the fixed
    /// column width reserved for them (widest marker + a space of
    /// breathing room — per-language, since "LECTURE SEULE" is wider
    /// than "READ-ONLY").
    pub fn marker_read_only(self) -> &'static str {
        match self {
            Lang::En => "READ-ONLY",
            Lang::Fr => "LECTURE SEULE",
        }
    }

    pub fn marker_unmounted(self) -> &'static str {
        match self {
            Lang::En => "UNMOUNTED",
            Lang::Fr => "NON MONTÉ",
        }
    }

    pub fn marker_archived(self) -> &'static str {
        match self {
            Lang::En => "ARCHIVED",
            Lang::Fr => "ARCHIVÉ",
        }
    }

    pub fn marker_width(self) -> u16 {
        match self {
            Lang::En => 12,
            Lang::Fr => 14,
        }
    }

    // ------------------------------------------------------------------
    // app.rs — status messages, default note text
    // ------------------------------------------------------------------

    /// Title/body of the note auto-created in an otherwise empty vault.
    /// Persisted content (not just a label), so it's stamped in whichever
    /// language was configured at creation time and stays that way.
    pub fn welcome_title(self) -> &'static str {
        match self {
            Lang::En => "Welcome to Mycora",
            Lang::Fr => "Bienvenue dans Mycora",
        }
    }

    pub fn welcome_body(self) -> &'static str {
        match self {
            Lang::En => "a: child  o: sibling  i: rename  y: copy  d: delete  u: undo  q: quit",
            Lang::Fr => {
                "a: enfant  o: voisine  i: renommer  y: copier  d: supprimer  u: annuler  q: quitter"
            }
        }
    }

    /// Placeholder title of a freshly created note, before `begin_naming`
    /// replaces it.
    pub fn new_note_title(self) -> &'static str {
        match self {
            Lang::En => "New note",
            Lang::Fr => "Nouvelle note",
        }
    }

    pub fn read_only_vault(self) -> &'static str {
        match self {
            Lang::En => "this vault is read-only",
            Lang::Fr => "ce vault est en lecture seule",
        }
    }

    pub fn trash_failed(self, err: &impl std::fmt::Display) -> String {
        match self {
            Lang::En => format!("trash failed: {err}"),
            Lang::Fr => format!("échec de la mise à la corbeille : {err}"),
        }
    }

    pub fn save_failed(self, err: &impl std::fmt::Display) -> String {
        match self {
            Lang::En => format!("save failed: {err}"),
            Lang::Fr => format!("échec de la sauvegarde : {err}"),
        }
    }

    pub fn reindex_failed(self, err: &impl std::fmt::Display) -> String {
        match self {
            Lang::En => format!("reindex failed: {err}"),
            Lang::Fr => format!("échec de la réindexation : {err}"),
        }
    }

    pub fn search_failed(self, err: &impl std::fmt::Display) -> String {
        match self {
            Lang::En => format!("search failed: {err}"),
            Lang::Fr => format!("échec de la recherche : {err}"),
        }
    }

    pub fn tag_list_failed(self, err: &impl std::fmt::Display) -> String {
        match self {
            Lang::En => format!("tag list failed: {err}"),
            Lang::Fr => format!("échec de la liste des tags : {err}"),
        }
    }

    pub fn tag_filter_failed(self, err: &impl std::fmt::Display) -> String {
        match self {
            Lang::En => format!("tag filter failed: {err}"),
            Lang::Fr => format!("échec du filtre par tags : {err}"),
        }
    }

    pub fn export_failed(self, err: &impl std::fmt::Display) -> String {
        match self {
            Lang::En => format!("export failed: {err}"),
            Lang::Fr => format!("échec de l'export : {err}"),
        }
    }

    pub fn unknown_command(self, name: &str) -> String {
        match self {
            Lang::En => format!("unknown command: {name}"),
            Lang::Fr => format!("commande inconnue : {name}"),
        }
    }

    pub fn reindexed_notes(self, count: usize) -> String {
        match self {
            Lang::En => format!("reindexed {count} note(s)"),
            Lang::Fr => format!("{count} note(s) réindexée(s)"),
        }
    }

    /// Usage strings quote the command syntax verbatim (untranslated —
    /// it's what the user must literally type), so only the `usage:`
    /// prefix differs.
    pub fn tags_usage(self) -> &'static str {
        match self {
            Lang::En => {
                "usage: :tags <tag1,tag2,...> or :tags list or :tags limit <vault-name> or \
                 :tags unlimit"
            }
            Lang::Fr => {
                "usage : :tags <tag1,tag2,...> ou :tags list ou :tags limit <vault-name> ou \
                 :tags unlimit"
            }
        }
    }

    pub fn tags_limit_usage(self) -> &'static str {
        match self {
            Lang::En => "usage: :tags limit <vault-name>",
            Lang::Fr => "usage : :tags limit <vault-name>",
        }
    }

    pub fn no_mounted_vault_named(self, name: &str) -> String {
        match self {
            Lang::En => format!("no mounted vault named \"{name}\""),
            Lang::Fr => format!("aucun vault monté nommé \"{name}\""),
        }
    }

    pub fn tags_limited_to(self, name: &str) -> String {
        match self {
            Lang::En => format!("tags limited to \"{name}\""),
            Lang::Fr => format!("tags limités à \"{name}\""),
        }
    }

    pub fn tags_were_not_limited(self) -> &'static str {
        match self {
            Lang::En => "tags were not limited",
            Lang::Fr => "les tags n'étaient pas limités",
        }
    }

    pub fn tags_no_longer_limited(self) -> &'static str {
        match self {
            Lang::En => "tags no longer limited",
            Lang::Fr => "tags non limités désormais",
        }
    }

    pub fn no_tags_in(self, name: &str) -> String {
        match self {
            Lang::En => format!("no tags in \"{name}\""),
            Lang::Fr => format!("aucun tag dans \"{name}\""),
        }
    }

    pub fn no_tags_anywhere(self) -> &'static str {
        match self {
            Lang::En => "no tags in any mounted vault",
            Lang::Fr => "aucun tag dans aucun vault monté",
        }
    }

    pub fn no_notes_tagged_in(self, tags: &str, name: &str) -> String {
        match self {
            Lang::En => format!("no notes tagged {tags} in \"{name}\""),
            Lang::Fr => format!("aucune note taguée {tags} dans \"{name}\""),
        }
    }

    pub fn no_notes_tagged_anywhere(self, tags: &str) -> String {
        match self {
            Lang::En => format!("no notes tagged {tags} in any mounted vault"),
            Lang::Fr => format!("aucune note taguée {tags} dans aucun vault monté"),
        }
    }

    pub fn panes_usage(self) -> &'static str {
        match self {
            Lang::En => "usage: :panes reset",
            Lang::Fr => "usage : :panes reset",
        }
    }

    pub fn panes_reset_done(self) -> &'static str {
        match self {
            Lang::En => "pane widths reset to default",
            Lang::Fr => "largeurs de panneaux réinitialisées",
        }
    }

    pub fn config_usage(self) -> &'static str {
        match self {
            Lang::En => "usage: :config <unmount|archive> <show|hide>",
            Lang::Fr => "usage : :config <unmount|archive> <show|hide>",
        }
    }

    /// `:config unmount/archive show/hide`'s confirmation message.
    pub fn config_vaults_visibility(self, unmounted: bool, shown: bool) -> String {
        match self {
            Lang::En => {
                let noun = if unmounted { "unmounted" } else { "archived" };
                let verb = if shown { "shown" } else { "hidden" };
                format!("{noun} vaults now {verb} in the tree")
            }
            Lang::Fr => {
                let noun = if unmounted { "non montés" } else { "archivés" };
                let verb = if shown { "affichés" } else { "masqués" };
                format!("vaults {noun} désormais {verb} dans l'arbre")
            }
        }
    }

    pub fn tag_usage(self) -> &'static str {
        match self {
            Lang::En => "usage: :tag <add|del> <tag>",
            Lang::Fr => "usage : :tag <add|del> <tag>",
        }
    }

    pub fn nothing_selected_to_tag(self) -> &'static str {
        match self {
            Lang::En => "nothing selected to tag",
            Lang::Fr => "aucune note sélectionnée à taguer",
        }
    }

    pub fn already_tagged(self, tag: &str) -> String {
        match self {
            Lang::En => format!("already tagged \"{tag}\""),
            Lang::Fr => format!("déjà taguée \"{tag}\""),
        }
    }

    pub fn not_tagged(self, tag: &str) -> String {
        match self {
            Lang::En => format!("not tagged \"{tag}\""),
            Lang::Fr => format!("pas taguée \"{tag}\""),
        }
    }

    pub fn tag_added(self, tag: &str) -> String {
        match self {
            Lang::En => format!("tag \"{tag}\" added"),
            Lang::Fr => format!("tag \"{tag}\" ajouté"),
        }
    }

    pub fn tag_removed(self, tag: &str) -> String {
        match self {
            Lang::En => format!("tag \"{tag}\" removed"),
            Lang::Fr => format!("tag \"{tag}\" retiré"),
        }
    }

    pub fn lang_usage(self) -> &'static str {
        match self {
            Lang::En => "usage: :lang <en|fr>",
            Lang::Fr => "usage : :lang <en|fr>",
        }
    }

    /// Names the current language, in that language — both the bare
    /// `:lang` status reply and the confirmation right after a switch
    /// (rendered in the *new* language, which is itself the proof the
    /// switch took effect).
    pub fn language_now(self) -> &'static str {
        match self {
            Lang::En => "language: English (en)",
            Lang::Fr => "langue : français (fr)",
        }
    }

    /// The switch already happened in memory, but writing it to
    /// `config.toml` failed — honest about the half-applied state: this
    /// session is switched, the next launch won't be.
    pub fn language_save_failed(self, err: &impl std::fmt::Display) -> String {
        match self {
            Lang::En => {
                format!("language switched for this session, but saving to config.toml failed: {err}")
            }
            Lang::Fr => {
                format!("langue changée pour cette session, mais l'écriture de config.toml a échoué : {err}")
            }
        }
    }

    pub fn export_usage(self) -> &'static str {
        match self {
            Lang::En => "usage: :export <path>",
            Lang::Fr => "usage : :export <path>",
        }
    }

    pub fn nothing_selected_to_export(self) -> &'static str {
        match self {
            Lang::En => "nothing selected to export",
            Lang::Fr => "aucune note sélectionnée à exporter",
        }
    }

    pub fn already_exists(self, path: &str) -> String {
        match self {
            Lang::En => format!("{path} already exists"),
            Lang::Fr => format!("{path} existe déjà"),
        }
    }

    pub fn exported_to(self, path: &str) -> String {
        match self {
            Lang::En => format!("exported to {path}"),
            Lang::Fr => format!("exporté vers {path}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_code_parses_known_languages_and_rejects_unknown() {
        assert_eq!(Lang::from_code("en"), Some(Lang::En));
        assert_eq!(Lang::from_code("fr"), Some(Lang::Fr));
        assert_eq!(Lang::from_code("de"), None);
        assert_eq!(Lang::from_code(""), None);
        // Deliberately strict: no case folding or region tags — the
        // config documents exactly "en"/"fr", and a fuzzy match here
        // would just delay the clear startup error to a subtler place.
        assert_eq!(Lang::from_code("EN"), None);
        assert_eq!(Lang::from_code("fr_FR"), None);
    }

    #[test]
    fn parameterized_messages_embed_their_arguments_in_both_languages() {
        for lang in [Lang::En, Lang::Fr] {
            assert!(lang.unknown_command("frobnicate").contains("frobnicate"));
            assert!(lang.delete_prompt("My Note", 3).contains("My Note"));
            assert!(lang.delete_prompt("My Note", 3).contains('3'));
            assert!(lang.tags_limited_to("work").contains("work"));
            assert!(lang.exported_to("/tmp/out.md").contains("/tmp/out.md"));
        }
    }

    #[test]
    fn command_reference_has_the_same_commands_in_every_language() {
        // Command *syntax* must never diverge between languages — only
        // descriptions translate (see `Lang`'s doc comment).
        let en: Vec<&str> = Lang::En.command_reference().iter().map(|(c, _)| *c).collect();
        let fr: Vec<&str> = Lang::Fr.command_reference().iter().map(|(c, _)| *c).collect();
        assert_eq!(en, fr);
    }

    #[test]
    fn normal_mode_hint_keys_are_identical_across_languages() {
        // `disabled_keys` matching in `ui.rs` depends on the key half of
        // each `key: label` token being byte-identical across languages —
        // but only Normal mode's hints ever get keys disabled, so only
        // Normal mode carries this constraint. Other modes may translate
        // a pseudo-key too (Search's "type:"/"taper:" is a description
        // of typing, not a key name).
        let keys = |lang: Lang| -> Vec<String> {
            lang.mode_line(Mode::Normal)
                .1
                .split("  ")
                .filter(|t| !t.is_empty())
                .filter_map(|t| t.split_once(": ").map(|(k, _)| k.to_string()))
                .collect()
        };
        assert_eq!(keys(Lang::En), keys(Lang::Fr));
    }

    #[test]
    fn markers_fit_their_reserved_breadcrumb_column() {
        for lang in [Lang::En, Lang::Fr] {
            let width = lang.marker_width() as usize;
            for marker in [
                lang.marker_read_only(),
                lang.marker_unmounted(),
                lang.marker_archived(),
            ] {
                assert!(
                    marker.chars().count() <= width,
                    "{marker:?} overflows the {width}-cell marker column"
                );
            }
        }
    }
}
