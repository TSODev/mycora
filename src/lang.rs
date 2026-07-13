use crate::app::Mode;

/// Which language the TUI renders its interface in — labels, hints,
/// prompts, and status messages. Keybindings and command names (`:tags`,
/// `:export`, ...) are identical in every language, same as vim's `:w`
/// doesn't translate: they're interface *syntax*, not interface text, and
/// keeping them fixed means every keybinding reference, script, and
/// muscle memory works regardless of language.
///
/// Every language is embedded in the binary rather than loaded from
/// external language files: every message here is a real `format!` call
/// checked at compile time, so a missing key or a typo'd placeholder is a
/// compile error instead of a runtime surprise — and the binary stays
/// self-contained (no files to install alongside it, nothing to fail to
/// parse at startup). The cost is that adding a language means
/// recompiling; an optional override file can be layered on later if
/// out-of-tree translations ever matter more than that guarantee. Adding
/// one is mechanical, not risky: the compiler's exhaustiveness check
/// refuses to build until every `match self { ... }` here has a new arm,
/// so nothing can be silently left in English.
///
/// Selected by `language = "fr"` in `config.toml` (see `Config`) —
/// English is the default. Spanish (`"es"`) and German (`"de"`) machine-
/// translated by the assistant that added them (2026-07-13) and flagged
/// for a native-speaker review — not yet reviewed, unlike English/French.
/// TUI-only for now: CLI output (`mycora vault list`, reindex reports,
/// load warnings) stays English, matching the language of the on-disk
/// formats and docs it quotes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    Fr,
    Es,
    De,
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
            "es" => Some(Lang::Es),
            "de" => Some(Lang::De),
            _ => None,
        }
    }

    /// The config-file code for this language — `from_code`'s inverse,
    /// what `:lang` writes back into `config.toml`.
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Fr => "fr",
            Lang::Es => "es",
            Lang::De => "de",
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
                (":lang <en|fr|es|de>", "switch the interface language (persists)"),
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
                (":lang <en|fr|es|de>", "change la langue de l'interface (persistant)"),
                (":q, :quit", "quitte Mycora"),
            ],
            Lang::Es => &[
                (":reindex", "reconstruye el índice de búsqueda"),
                (
                    ":tags <tag1,tag2,...>",
                    "lista notas que coincidan con alguna de las etiquetas dadas",
                ),
                (":tags list", "lista todas las etiquetas conocidas, elige una para filtrar"),
                (
                    ":tags limit <vault-name>",
                    "restringe :tags/:tags list a un solo vault montado",
                ),
                (":tags unlimit", "quita el límite de :tags, vuelve a todos los vaults"),
                (":panes reset", "restablece los anchos de panel a 40/40/20"),
                (
                    ":export <path>",
                    "aplana el subárbol de la nota seleccionada a un archivo Markdown",
                ),
                (
                    ":config unmount <show|hide>",
                    "muestra/oculta las filas de vaults no montados en el árbol",
                ),
                (
                    ":config archive <show|hide>",
                    "muestra/oculta las filas de vaults archivados en el árbol",
                ),
                (":tag add <tag>", "añade una etiqueta a la nota seleccionada"),
                (":tag del <tag>", "elimina una etiqueta de la nota seleccionada"),
                (":lang <en|fr|es|de>", "cambia el idioma de la interfaz (persistente)"),
                (":q, :quit", "sale de Mycora"),
            ],
            Lang::De => &[
                (":reindex", "baut den Suchindex neu auf"),
                (
                    ":tags <tag1,tag2,...>",
                    "listet Notizen mit einem der angegebenen Tags",
                ),
                (":tags list", "listet alle bekannten Tags, eines zum Filtern wählen"),
                (
                    ":tags limit <vault-name>",
                    "beschränkt :tags/:tags list auf einen gemounteten Vault",
                ),
                (":tags unlimit", "hebt die :tags-Beschränkung auf, zurück zu allen Vaults"),
                (":panes reset", "setzt die Feldbreiten auf 40/40/20 zurück"),
                (
                    ":export <path>",
                    "flacht den Teilbaum der ausgewählten Notiz zu Markdown ab",
                ),
                (
                    ":config unmount <show|hide>",
                    "zeigt/versteckt Zeilen für nicht gemountete Vaults im Baum",
                ),
                (
                    ":config archive <show|hide>",
                    "zeigt/versteckt Zeilen für archivierte Vaults im Baum",
                ),
                (":tag add <tag>", "fügt der ausgewählten Notiz einen Tag hinzu"),
                (":tag del <tag>", "entfernt einen Tag von der ausgewählten Notiz"),
                (":lang <en|fr|es|de>", "wechselt die Oberflächensprache (dauerhaft)"),
                (":q, :quit", "beendet Mycora"),
            ],
        }
    }

    /// `(key, description)` pairs for `?`'s full-pane keybinding
    /// reference (`Mode::Help`, see `ui.rs`'s `draw_help`) — every Normal
    /// mode key, not just the short curated subset `mode_line`'s own
    /// `Normal` hint string shows. Key *syntax* is identical across
    /// languages, same reasoning as `command_reference`; kept in sync
    /// with `event.rs`'s `handle_normal` by hand, same as
    /// `command_reference` is with `execute_command`.
    pub fn help_reference(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Lang::En => &[
                ("j/k, ↑/↓", "move selection"),
                ("l, →, Enter", "expand / open"),
                ("h, ←", "collapse"),
                ("space", "toggle expand"),
                ("a", "new child note"),
                ("o", "new sibling note"),
                ("y", "duplicate subtree"),
                ("i", "rename"),
                ("e", "edit body"),
                ("d", "delete (asks to confirm)"),
                ("Tab / Shift+Tab", "indent / outdent"),
                ("K / J", "reorder up / down among siblings"),
                ("u", "undo"),
                ("Ctrl+R", "redo"),
                ("/", "search"),
                ("b", "backlinks (notes linking here)"),
                ("f", "follow outgoing links"),
                ("[ / ]", "shrink / grow tree pane"),
                ("{ / }", "shrink / grow backlinks pane"),
                ("Ctrl+D / Ctrl+U", "scroll body preview down / up"),
                (":", "command palette"),
                ("?", "this help"),
                ("q q", "quit (press twice)"),
                ("Ctrl+C", "quit immediately"),
            ],
            Lang::Fr => &[
                ("j/k, ↑/↓", "déplacer la sélection"),
                ("l, →, Enter", "déplier / ouvrir"),
                ("h, ←", "plier"),
                ("space", "basculer le pli"),
                ("a", "nouvelle note enfant"),
                ("o", "nouvelle note voisine"),
                ("y", "dupliquer le sous-arbre"),
                ("i", "renommer"),
                ("e", "éditer le corps"),
                ("d", "supprimer (demande confirmation)"),
                ("Tab / Shift+Tab", "indenter / désindenter"),
                ("K / J", "réordonner parmi les voisines"),
                ("u", "annuler"),
                ("Ctrl+R", "rétablir"),
                ("/", "rechercher"),
                ("b", "rétroliens (notes qui pointent ici)"),
                ("f", "suivre les liens sortants"),
                ("[ / ]", "réduire / agrandir le panneau arbre"),
                ("{ / }", "réduire / agrandir le panneau rétroliens"),
                ("Ctrl+D / Ctrl+U", "faire défiler l'aperçu du corps"),
                (":", "palette de commandes"),
                ("?", "cette aide"),
                ("q q", "quitter (appuyer deux fois)"),
                ("Ctrl+C", "quitter immédiatement"),
            ],
            Lang::Es => &[
                ("j/k, ↑/↓", "mover la selección"),
                ("l, →, Enter", "desplegar / abrir"),
                ("h, ←", "plegar"),
                ("space", "alternar plegado"),
                ("a", "nueva nota hija"),
                ("o", "nueva nota hermana"),
                ("y", "duplicar subárbol"),
                ("i", "renombrar"),
                ("e", "editar cuerpo"),
                ("d", "eliminar (pide confirmación)"),
                ("Tab / Shift+Tab", "sangrar / desangrar"),
                ("K / J", "reordenar entre hermanas"),
                ("u", "deshacer"),
                ("Ctrl+R", "rehacer"),
                ("/", "buscar"),
                ("b", "retroenlaces (notas que enlazan aquí)"),
                ("f", "seguir enlaces salientes"),
                ("[ / ]", "encoger / agrandar el panel del árbol"),
                ("{ / }", "encoger / agrandar el panel de retroenlaces"),
                ("Ctrl+D / Ctrl+U", "desplazar la vista previa arriba/abajo"),
                (":", "paleta de comandos"),
                ("?", "esta ayuda"),
                ("q q", "salir (pulsar dos veces)"),
                ("Ctrl+C", "salir de inmediato"),
            ],
            Lang::De => &[
                ("j/k, ↑/↓", "Auswahl bewegen"),
                ("l, →, Enter", "aufklappen / öffnen"),
                ("h, ←", "einklappen"),
                ("space", "Klappzustand umschalten"),
                ("a", "neue Kind-Notiz"),
                ("o", "neue Geschwister-Notiz"),
                ("y", "Teilbaum duplizieren"),
                ("i", "umbenennen"),
                ("e", "Inhalt bearbeiten"),
                ("d", "löschen (fragt nach Bestätigung)"),
                ("Tab / Shift+Tab", "einrücken / ausrücken"),
                ("K / J", "unter Geschwistern umordnen"),
                ("u", "rückgängig"),
                ("Ctrl+R", "wiederholen"),
                ("/", "suchen"),
                ("b", "Rückverweise (Notizen, die hierher verlinken)"),
                ("f", "ausgehenden Links folgen"),
                ("[ / ]", "Baumbereich verkleinern / vergrößern"),
                ("{ / }", "Rückverweisbereich verkleinern / vergrößern"),
                ("Ctrl+D / Ctrl+U", "Vorschau runter-/hochscrollen"),
                (":", "Befehlspalette"),
                ("?", "diese Hilfe"),
                ("q q", "beenden (zweimal drücken)"),
                ("Ctrl+C", "sofort beenden"),
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
            Lang::Es => {
                let plural = if count == 1 { "" } else { "s" };
                format!("({count} enlace{plural})")
            }
            Lang::De => {
                let plural = if count == 1 { "" } else { "s" };
                format!("({count} Link{plural})")
            }
        }
    }

    /// The `(3 notes)` count next to a tag in `:tags list`. German's
    /// plural ("Notizen") isn't a simple suffix on the singular
    /// ("Notiz"), unlike the other three languages, hence the noun
    /// picked whole per-arm rather than shared `plural`-suffix logic.
    pub fn notes_badge(self, count: i64) -> String {
        match self {
            Lang::En => {
                let plural = if count == 1 { "" } else { "s" };
                format!("({count} note{plural})")
            }
            Lang::Fr => {
                let plural = if count == 1 { "" } else { "s" };
                format!("({count} note{plural})")
            }
            Lang::Es => {
                let plural = if count == 1 { "" } else { "s" };
                format!("({count} nota{plural})")
            }
            Lang::De => {
                let noun = if count == 1 { "Notiz" } else { "Notizen" };
                format!("({count} {noun})")
            }
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
            Lang::Es => format!(
                "El vault \"{name}\" no está montado.\n\nRuta: {path}\n\nPara activarlo:\n  mycora vault mount {name}"
            ),
            Lang::De => format!(
                "Vault \"{name}\" ist nicht gemountet.\n\nPfad: {path}\n\nZum Aktivieren:\n  mycora vault mount {name}"
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
            Lang::Es => format!(
                "El vault \"{name}\" está archivado.\n\nArchivo: {archive_path}\n\nPara restaurarlo:\n  mycora vault unarchive {name}"
            ),
            Lang::De => format!(
                "Vault \"{name}\" ist archiviert.\n\nArchiv: {archive_path}\n\nZum Wiederherstellen:\n  mycora vault unarchive {name}"
            ),
        }
    }

    pub fn backlinks_title(self) -> &'static str {
        match self {
            Lang::En => "Backlinks",
            Lang::Fr => "Rétroliens",
            Lang::Es => "Retroenlaces",
            Lang::De => "Rückverweise",
        }
    }

    pub fn search_title(self, scope: &str, query: &str) -> String {
        match self {
            Lang::En => format!("Search [{scope}]: {query}"),
            Lang::Fr => format!("Recherche [{scope}] : {query}"),
            Lang::Es => format!("Búsqueda [{scope}]: {query}"),
            Lang::De => format!("Suche [{scope}]: {query}"),
        }
    }

    pub fn tag_results_title(self, scope: &str) -> String {
        match self {
            Lang::En => format!("Tag results [{scope}]"),
            Lang::Fr => format!("Résultats tags [{scope}]"),
            Lang::Es => format!("Resultados de etiquetas [{scope}]"),
            Lang::De => format!("Tag-Ergebnisse [{scope}]"),
        }
    }

    /// Title of the `f` (outgoing links) full-pane overlay — `scope` is
    /// the *source* note's vault, matching `search_title`'s convention.
    pub fn links_title(self, scope: &str) -> String {
        match self {
            Lang::En => format!("Links [{scope}]"),
            Lang::Fr => format!("Liens [{scope}]"),
            Lang::Es => format!("Enlaces [{scope}]"),
            Lang::De => format!("Links [{scope}]"),
        }
    }

    pub fn tag_list_title(self, scope: &str) -> String {
        match self {
            Lang::En => format!("Tags [{scope}]"),
            Lang::Fr => format!("Tags [{scope}]"),
            Lang::Es => format!("Etiquetas [{scope}]"),
            Lang::De => format!("Tags [{scope}]"),
        }
    }

    /// What the tag overlays' titles show when no `:tags limit` is active.
    pub fn all_vaults_label(self) -> &'static str {
        match self {
            Lang::En => "all vaults",
            Lang::Fr => "tous les vaults",
            Lang::Es => "todos los vaults",
            Lang::De => "alle Vaults",
        }
    }

    pub fn commands_title(self) -> &'static str {
        match self {
            Lang::En => "Commands",
            Lang::Fr => "Commandes",
            Lang::Es => "Comandos",
            Lang::De => "Befehle",
        }
    }

    /// Title of `?`'s full-pane keybinding reference (`Mode::Help`).
    pub fn help_title(self) -> &'static str {
        match self {
            Lang::En => "Keybindings",
            Lang::Fr => "Raccourcis",
            Lang::Es => "Atajos de teclado",
            Lang::De => "Tastenkürzel",
        }
    }

    /// Title of the `[[wikilink]]` autocomplete popup in the body editor.
    pub fn link_popup_title(self) -> &'static str {
        match self {
            Lang::En => "Link",
            Lang::Fr => "Lien",
            Lang::Es => "Enlace",
            Lang::De => "Link",
        }
    }

    /// The `y/n` delete confirmation prompt. The `y/n` keys themselves
    /// don't translate (they're keybindings — see the type-level doc
    /// comment), so the prompt spells them out as-is in every language.
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
            Lang::Es => {
                if descendants > 0 {
                    format!("¿Eliminar '{title}' y sus {descendants} descendiente(s)? y/n")
                } else {
                    format!("¿Eliminar '{title}'? y/n")
                }
            }
            Lang::De => {
                if descendants > 0 {
                    format!("'{title}' und seine {descendants} Nachfahren löschen? y/n")
                } else {
                    format!("'{title}' löschen? y/n")
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
            Lang::Es => "esta nota",
            Lang::De => "diese Notiz",
        }
    }

    pub fn press_q_again(self) -> &'static str {
        match self {
            Lang::En => "Press q again to quit",
            Lang::Fr => "Appuyez encore sur q pour quitter",
            Lang::Es => "Pulsa q de nuevo para salir",
            Lang::De => "Erneut q drücken zum Beenden",
        }
    }

    /// The bold prefix on `last_error` in the hint row.
    pub fn error_prefix(self) -> &'static str {
        match self {
            Lang::En => "ERROR",
            Lang::Fr => "ERREUR",
            Lang::Es => "ERROR",
            Lang::De => "FEHLER",
        }
    }

    /// `(mode label, hint string)` for the status bar's hint row. Hint
    /// strings keep `ui.rs`'s parser convention — `key: label` tokens,
    /// double-space separated — and the *key* half of each token is
    /// identical across languages (it's what `disabled_keys` matches on,
    /// and it names real keys); only the labels translate.
    pub fn mode_line(self, mode: Mode) -> (&'static str, &'static str) {
        match (self, mode) {
            // Deliberately short — the full set (this used to list every
            // Normal-mode key here) ran to 233 characters, wider than any
            // realistic terminal. `?` opens the complete reference
            // (`Lang::help_reference`, `Mode::Help`) instead; this row
            // only keeps the handful reached for constantly, so it stays
            // short even as more keys are added later.
            (Lang::En, Mode::Normal) => (
                "NORMAL",
                "j/k: move  a/o: new  e: edit  d: delete  u: undo  \
                 /: search  ?: help  q: quit",
            ),
            (Lang::Fr, Mode::Normal) => (
                "NORMAL",
                "j/k: bouger  a/o: nouvelle  e: éditer  d: supprimer  u: annuler  \
                 /: rechercher  ?: aide  q: quitter",
            ),
            (Lang::Es, Mode::Normal) => (
                "NORMAL",
                "j/k: mover  a/o: nueva  e: editar  d: eliminar  u: deshacer  \
                 /: buscar  ?: ayuda  q: salir",
            ),
            (Lang::De, Mode::Normal) => (
                "NORMAL",
                "j/k: bewegen  a/o: neu  e: bearbeiten  d: löschen  u: rückgängig  \
                 /: suchen  ?: Hilfe  q: beenden",
            ),
            (Lang::En, Mode::Insert) => ("INSERT", "Enter: confirm  Esc: cancel"),
            (Lang::Fr, Mode::Insert) => ("INSERTION", "Enter: valider  Esc: annuler"),
            (Lang::Es, Mode::Insert) => ("INSERTAR", "Enter: confirmar  Esc: cancelar"),
            (Lang::De, Mode::Insert) => ("EINFÜGEN", "Enter: bestätigen  Esc: abbrechen"),
            (Lang::En, Mode::Search) => (
                "SEARCH",
                "type: filter  Up/Down: move  Enter: open  Esc: cancel",
            ),
            (Lang::Fr, Mode::Search) => (
                "RECHERCHE",
                "taper: filtrer  Up/Down: bouger  Enter: ouvrir  Esc: annuler",
            ),
            (Lang::Es, Mode::Search) => (
                "BÚSQUEDA",
                "escribir: filtrar  Up/Down: mover  Enter: abrir  Esc: cancelar",
            ),
            (Lang::De, Mode::Search) => (
                "SUCHE",
                "tippen: filtern  Up/Down: bewegen  Enter: öffnen  Esc: abbrechen",
            ),
            (Lang::En, Mode::Backlinks) => (
                "BACKLINKS",
                "j/k: move  Enter: jump  Esc/b: back to tree",
            ),
            (Lang::Fr, Mode::Backlinks) => (
                "RÉTROLIENS",
                "j/k: bouger  Enter: sauter  Esc/b: retour à l'arbre",
            ),
            (Lang::Es, Mode::Backlinks) => (
                "ENLACES",
                "j/k: mover  Enter: saltar  Esc/b: volver al árbol",
            ),
            (Lang::De, Mode::Backlinks) => (
                "RÜCKVERWEISE",
                "j/k: bewegen  Enter: springen  Esc/b: zurück zum Baum",
            ),
            (Lang::En, Mode::EditBody) => ("EDIT BODY", "Esc: save & exit"),
            (Lang::Fr, Mode::EditBody) => ("ÉDITION", "Esc: sauver & quitter"),
            (Lang::Es, Mode::EditBody) => ("EDITAR", "Esc: guardar y salir"),
            (Lang::De, Mode::EditBody) => ("BEARBEITEN", "Esc: speichern & verlassen"),
            (Lang::En, Mode::TagResults) => {
                ("TAG RESULTS", "j/k: move  Enter: open  Esc: cancel")
            }
            (Lang::Fr, Mode::TagResults) => {
                ("RÉSULTATS TAGS", "j/k: bouger  Enter: ouvrir  Esc: annuler")
            }
            (Lang::Es, Mode::TagResults) => {
                ("RESULTADOS", "j/k: mover  Enter: abrir  Esc: cancelar")
            }
            (Lang::De, Mode::TagResults) => {
                ("TAG-ERGEBNISSE", "j/k: bewegen  Enter: öffnen  Esc: abbrechen")
            }
            (Lang::En, Mode::TagList) => ("TAGS", "j/k: move  Enter: filter  Esc: cancel"),
            (Lang::Fr, Mode::TagList) => ("TAGS", "j/k: bouger  Enter: filtrer  Esc: annuler"),
            (Lang::Es, Mode::TagList) => ("TAGS", "j/k: mover  Enter: filtrar  Esc: cancelar"),
            (Lang::De, Mode::TagList) => ("TAGS", "j/k: bewegen  Enter: filtern  Esc: abbrechen"),
            (Lang::En, Mode::Links) => ("LINKS", "j/k: move  Enter: jump  Esc: cancel"),
            (Lang::Fr, Mode::Links) => ("LIENS", "j/k: bouger  Enter: sauter  Esc: annuler"),
            (Lang::Es, Mode::Links) => ("ENLACES", "j/k: mover  Enter: saltar  Esc: cancelar"),
            (Lang::De, Mode::Links) => ("LINKS", "j/k: bewegen  Enter: springen  Esc: abbrechen"),
            (Lang::En, Mode::Help) => ("HELP", "any key: close"),
            (Lang::Fr, Mode::Help) => ("AIDE", "n'importe quelle touche : fermer"),
            (Lang::Es, Mode::Help) => ("AYUDA", "cualquier tecla: cerrar"),
            (Lang::De, Mode::Help) => ("HILFE", "beliebige Taste: schließen"),
            (_, Mode::ConfirmDelete | Mode::Command) => {
                unreachable!("ConfirmDelete/Command render their own prompt row, not hints")
            }
        }
    }

    /// The breadcrumb row's right-aligned status markers, and the fixed
    /// column width reserved for them (widest marker + a space of
    /// breathing room — per-language, since e.g. "LECTURE SEULE" is
    /// wider than "READ-ONLY").
    pub fn marker_read_only(self) -> &'static str {
        match self {
            Lang::En => "READ-ONLY",
            Lang::Fr => "LECTURE SEULE",
            Lang::Es => "SOLO LECTURA",
            Lang::De => "NUR LESEN",
        }
    }

    pub fn marker_unmounted(self) -> &'static str {
        match self {
            Lang::En => "UNMOUNTED",
            Lang::Fr => "NON MONTÉ",
            Lang::Es => "DESMONTADO",
            Lang::De => "AUSGEHÄNGT",
        }
    }

    pub fn marker_archived(self) -> &'static str {
        match self {
            Lang::En => "ARCHIVED",
            Lang::Fr => "ARCHIVÉ",
            Lang::Es => "ARCHIVADO",
            Lang::De => "ARCHIVIERT",
        }
    }

    pub fn marker_width(self) -> u16 {
        match self {
            Lang::En => 12,
            Lang::Fr => 14,
            Lang::Es => 14,
            Lang::De => 12,
        }
    }

    /// The breadcrumb row's centered "last modified" label — `formatted`
    /// is already a plain `"YYYY-MM-DD HH:MM"` string (see `ui.rs`'s
    /// `format_last_modified`); this just prepends the translated word.
    pub fn last_modified_label(self, formatted: &str) -> String {
        match self {
            Lang::En => format!("modified: {formatted}"),
            Lang::Fr => format!("modifié : {formatted}"),
            Lang::Es => format!("modificado: {formatted}"),
            Lang::De => format!("geändert: {formatted}"),
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
            Lang::Es => "Bienvenido a Mycora",
            Lang::De => "Willkommen bei Mycora",
        }
    }

    pub fn welcome_body(self) -> &'static str {
        match self {
            Lang::En => "a: child  o: sibling  i: rename  y: copy  d: delete  u: undo  q: quit",
            Lang::Fr => {
                "a: enfant  o: voisine  i: renommer  y: copier  d: supprimer  u: annuler  q: quitter"
            }
            Lang::Es => {
                "a: hijo  o: hermano  i: renombrar  y: copiar  d: eliminar  u: deshacer  q: salir"
            }
            Lang::De => {
                "a: Kind  o: Geschwister  i: umbenennen  y: kopieren  d: löschen  u: rückgängig  q: beenden"
            }
        }
    }

    /// Placeholder title of a freshly created note, before `begin_naming`
    /// replaces it.
    pub fn new_note_title(self) -> &'static str {
        match self {
            Lang::En => "New note",
            Lang::Fr => "Nouvelle note",
            Lang::Es => "Nueva nota",
            Lang::De => "Neue Notiz",
        }
    }

    pub fn read_only_vault(self) -> &'static str {
        match self {
            Lang::En => "this vault is read-only",
            Lang::Fr => "ce vault est en lecture seule",
            Lang::Es => "este vault es de solo lectura",
            Lang::De => "dieser Vault ist schreibgeschützt",
        }
    }

    pub fn trash_failed(self, err: &impl std::fmt::Display) -> String {
        match self {
            Lang::En => format!("trash failed: {err}"),
            Lang::Fr => format!("échec de la mise à la corbeille : {err}"),
            Lang::Es => format!("error al mover a la papelera: {err}"),
            Lang::De => format!("Papierkorb fehlgeschlagen: {err}"),
        }
    }

    pub fn save_failed(self, err: &impl std::fmt::Display) -> String {
        match self {
            Lang::En => format!("save failed: {err}"),
            Lang::Fr => format!("échec de la sauvegarde : {err}"),
            Lang::Es => format!("error al guardar: {err}"),
            Lang::De => format!("Speichern fehlgeschlagen: {err}"),
        }
    }

    pub fn reindex_failed(self, err: &impl std::fmt::Display) -> String {
        match self {
            Lang::En => format!("reindex failed: {err}"),
            Lang::Fr => format!("échec de la réindexation : {err}"),
            Lang::Es => format!("error al reindexar: {err}"),
            Lang::De => format!("Reindexierung fehlgeschlagen: {err}"),
        }
    }

    pub fn search_failed(self, err: &impl std::fmt::Display) -> String {
        match self {
            Lang::En => format!("search failed: {err}"),
            Lang::Fr => format!("échec de la recherche : {err}"),
            Lang::Es => format!("error al buscar: {err}"),
            Lang::De => format!("Suche fehlgeschlagen: {err}"),
        }
    }

    pub fn tag_list_failed(self, err: &impl std::fmt::Display) -> String {
        match self {
            Lang::En => format!("tag list failed: {err}"),
            Lang::Fr => format!("échec de la liste des tags : {err}"),
            Lang::Es => format!("error al listar etiquetas: {err}"),
            Lang::De => format!("Tag-Liste fehlgeschlagen: {err}"),
        }
    }

    pub fn tag_filter_failed(self, err: &impl std::fmt::Display) -> String {
        match self {
            Lang::En => format!("tag filter failed: {err}"),
            Lang::Fr => format!("échec du filtre par tags : {err}"),
            Lang::Es => format!("error al filtrar por etiquetas: {err}"),
            Lang::De => format!("Tag-Filter fehlgeschlagen: {err}"),
        }
    }

    /// `f`'s empty-result message — this note's body has no `[[wikilink]]`
    /// that resolves to another note.
    pub fn no_outgoing_links(self) -> &'static str {
        match self {
            Lang::En => "this note has no outgoing links",
            Lang::Fr => "cette note n'a aucun lien sortant",
            Lang::Es => "esta nota no tiene enlaces salientes",
            Lang::De => "diese Notiz hat keine ausgehenden Links",
        }
    }

    pub fn links_failed(self, err: &impl std::fmt::Display) -> String {
        match self {
            Lang::En => format!("links failed: {err}"),
            Lang::Fr => format!("échec de la liste des liens : {err}"),
            Lang::Es => format!("error al listar enlaces: {err}"),
            Lang::De => format!("Link-Liste fehlgeschlagen: {err}"),
        }
    }

    pub fn export_failed(self, err: &impl std::fmt::Display) -> String {
        match self {
            Lang::En => format!("export failed: {err}"),
            Lang::Fr => format!("échec de l'export : {err}"),
            Lang::Es => format!("error al exportar: {err}"),
            Lang::De => format!("Export fehlgeschlagen: {err}"),
        }
    }

    pub fn unknown_command(self, name: &str) -> String {
        match self {
            Lang::En => format!("unknown command: {name}"),
            Lang::Fr => format!("commande inconnue : {name}"),
            Lang::Es => format!("comando desconocido: {name}"),
            Lang::De => format!("unbekannter Befehl: {name}"),
        }
    }

    pub fn reindexed_notes(self, count: usize) -> String {
        match self {
            Lang::En => format!("reindexed {count} note(s)"),
            Lang::Fr => format!("{count} note(s) réindexée(s)"),
            Lang::Es => format!("{count} nota(s) reindexada(s)"),
            Lang::De => format!("{count} Notiz(en) reindexiert"),
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
            Lang::Es => {
                "uso: :tags <tag1,tag2,...> o :tags list o :tags limit <vault-name> o \
                 :tags unlimit"
            }
            Lang::De => {
                "Verwendung: :tags <tag1,tag2,...> oder :tags list oder :tags limit <vault-name> \
                 oder :tags unlimit"
            }
        }
    }

    pub fn tags_limit_usage(self) -> &'static str {
        match self {
            Lang::En => "usage: :tags limit <vault-name>",
            Lang::Fr => "usage : :tags limit <vault-name>",
            Lang::Es => "uso: :tags limit <vault-name>",
            Lang::De => "Verwendung: :tags limit <vault-name>",
        }
    }

    pub fn no_mounted_vault_named(self, name: &str) -> String {
        match self {
            Lang::En => format!("no mounted vault named \"{name}\""),
            Lang::Fr => format!("aucun vault monté nommé \"{name}\""),
            Lang::Es => format!("no hay ningún vault montado llamado \"{name}\""),
            Lang::De => format!("kein gemounteter Vault namens \"{name}\""),
        }
    }

    pub fn tags_limited_to(self, name: &str) -> String {
        match self {
            Lang::En => format!("tags limited to \"{name}\""),
            Lang::Fr => format!("tags limités à \"{name}\""),
            Lang::Es => format!("etiquetas limitadas a \"{name}\""),
            Lang::De => format!("Tags beschränkt auf \"{name}\""),
        }
    }

    pub fn tags_were_not_limited(self) -> &'static str {
        match self {
            Lang::En => "tags were not limited",
            Lang::Fr => "les tags n'étaient pas limités",
            Lang::Es => "las etiquetas no estaban limitadas",
            Lang::De => "Tags waren nicht beschränkt",
        }
    }

    pub fn tags_no_longer_limited(self) -> &'static str {
        match self {
            Lang::En => "tags no longer limited",
            Lang::Fr => "tags non limités désormais",
            Lang::Es => "etiquetas ya no limitadas",
            Lang::De => "Tags nicht mehr beschränkt",
        }
    }

    pub fn no_tags_in(self, name: &str) -> String {
        match self {
            Lang::En => format!("no tags in \"{name}\""),
            Lang::Fr => format!("aucun tag dans \"{name}\""),
            Lang::Es => format!("no hay etiquetas en \"{name}\""),
            Lang::De => format!("keine Tags in \"{name}\""),
        }
    }

    pub fn no_tags_anywhere(self) -> &'static str {
        match self {
            Lang::En => "no tags in any mounted vault",
            Lang::Fr => "aucun tag dans aucun vault monté",
            Lang::Es => "no hay etiquetas en ningún vault montado",
            Lang::De => "keine Tags in keinem gemounteten Vault",
        }
    }

    pub fn no_notes_tagged_in(self, tags: &str, name: &str) -> String {
        match self {
            Lang::En => format!("no notes tagged {tags} in \"{name}\""),
            Lang::Fr => format!("aucune note taguée {tags} dans \"{name}\""),
            Lang::Es => format!("no hay notas etiquetadas {tags} en \"{name}\""),
            Lang::De => format!("keine Notizen mit {tags} in \"{name}\""),
        }
    }

    pub fn no_notes_tagged_anywhere(self, tags: &str) -> String {
        match self {
            Lang::En => format!("no notes tagged {tags} in any mounted vault"),
            Lang::Fr => format!("aucune note taguée {tags} dans aucun vault monté"),
            Lang::Es => format!("no hay notas etiquetadas {tags} en ningún vault montado"),
            Lang::De => format!("keine Notizen mit {tags} in keinem gemounteten Vault"),
        }
    }

    pub fn panes_usage(self) -> &'static str {
        match self {
            Lang::En => "usage: :panes reset",
            Lang::Fr => "usage : :panes reset",
            Lang::Es => "uso: :panes reset",
            Lang::De => "Verwendung: :panes reset",
        }
    }

    pub fn panes_reset_done(self) -> &'static str {
        match self {
            Lang::En => "pane widths reset to default",
            Lang::Fr => "largeurs de panneaux réinitialisées",
            Lang::Es => "anchos de panel restablecidos",
            Lang::De => "Feldbreiten zurückgesetzt",
        }
    }

    pub fn config_usage(self) -> &'static str {
        match self {
            Lang::En => "usage: :config <unmount|archive> <show|hide>",
            Lang::Fr => "usage : :config <unmount|archive> <show|hide>",
            Lang::Es => "uso: :config <unmount|archive> <show|hide>",
            Lang::De => "Verwendung: :config <unmount|archive> <show|hide>",
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
            Lang::Es => {
                let noun = if unmounted { "no montados" } else { "archivados" };
                let verb = if shown { "mostrados" } else { "ocultos" };
                format!("vaults {noun} ahora {verb} en el árbol")
            }
            Lang::De => {
                let noun = if unmounted { "Nicht gemountete" } else { "Archivierte" };
                let verb = if shown { "angezeigt" } else { "ausgeblendet" };
                format!("{noun} Vaults jetzt im Baum {verb}")
            }
        }
    }

    pub fn tag_usage(self) -> &'static str {
        match self {
            Lang::En => "usage: :tag <add|del> <tag>",
            Lang::Fr => "usage : :tag <add|del> <tag>",
            Lang::Es => "uso: :tag <add|del> <tag>",
            Lang::De => "Verwendung: :tag <add|del> <tag>",
        }
    }

    pub fn nothing_selected_to_tag(self) -> &'static str {
        match self {
            Lang::En => "nothing selected to tag",
            Lang::Fr => "aucune note sélectionnée à taguer",
            Lang::Es => "nada seleccionado para etiquetar",
            Lang::De => "nichts zum Taggen ausgewählt",
        }
    }

    pub fn already_tagged(self, tag: &str) -> String {
        match self {
            Lang::En => format!("already tagged \"{tag}\""),
            Lang::Fr => format!("déjà taguée \"{tag}\""),
            Lang::Es => format!("ya etiquetada \"{tag}\""),
            Lang::De => format!("bereits getaggt mit \"{tag}\""),
        }
    }

    pub fn not_tagged(self, tag: &str) -> String {
        match self {
            Lang::En => format!("not tagged \"{tag}\""),
            Lang::Fr => format!("pas taguée \"{tag}\""),
            Lang::Es => format!("no etiquetada \"{tag}\""),
            Lang::De => format!("nicht getaggt mit \"{tag}\""),
        }
    }

    pub fn tag_added(self, tag: &str) -> String {
        match self {
            Lang::En => format!("tag \"{tag}\" added"),
            Lang::Fr => format!("tag \"{tag}\" ajouté"),
            Lang::Es => format!("etiqueta \"{tag}\" añadida"),
            Lang::De => format!("Tag \"{tag}\" hinzugefügt"),
        }
    }

    pub fn tag_removed(self, tag: &str) -> String {
        match self {
            Lang::En => format!("tag \"{tag}\" removed"),
            Lang::Fr => format!("tag \"{tag}\" retiré"),
            Lang::Es => format!("etiqueta \"{tag}\" eliminada"),
            Lang::De => format!("Tag \"{tag}\" entfernt"),
        }
    }

    pub fn lang_usage(self) -> &'static str {
        match self {
            Lang::En => "usage: :lang <en|fr|es|de>",
            Lang::Fr => "usage : :lang <en|fr|es|de>",
            Lang::Es => "uso: :lang <en|fr|es|de>",
            Lang::De => "Verwendung: :lang <en|fr|es|de>",
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
            Lang::Es => "idioma: español (es)",
            Lang::De => "Sprache: Deutsch (de)",
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
            Lang::Es => {
                format!("idioma cambiado para esta sesión, pero no se pudo guardar en config.toml: {err}")
            }
            Lang::De => {
                format!("Sprache für diese Sitzung gewechselt, aber Speichern in config.toml fehlgeschlagen: {err}")
            }
        }
    }

    pub fn export_usage(self) -> &'static str {
        match self {
            Lang::En => "usage: :export <path>",
            Lang::Fr => "usage : :export <path>",
            Lang::Es => "uso: :export <path>",
            Lang::De => "Verwendung: :export <path>",
        }
    }

    pub fn nothing_selected_to_export(self) -> &'static str {
        match self {
            Lang::En => "nothing selected to export",
            Lang::Fr => "aucune note sélectionnée à exporter",
            Lang::Es => "nada seleccionado para exportar",
            Lang::De => "nichts zum Exportieren ausgewählt",
        }
    }

    pub fn already_exists(self, path: &str) -> String {
        match self {
            Lang::En => format!("{path} already exists"),
            Lang::Fr => format!("{path} existe déjà"),
            Lang::Es => format!("{path} ya existe"),
            Lang::De => format!("{path} existiert bereits"),
        }
    }

    pub fn exported_to(self, path: &str) -> String {
        match self {
            Lang::En => format!("exported to {path}"),
            Lang::Fr => format!("exporté vers {path}"),
            Lang::Es => format!("exportado a {path}"),
            Lang::De => format!("exportiert nach {path}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Lang; 4] = [Lang::En, Lang::Fr, Lang::Es, Lang::De];

    #[test]
    fn from_code_parses_known_languages_and_rejects_unknown() {
        assert_eq!(Lang::from_code("en"), Some(Lang::En));
        assert_eq!(Lang::from_code("fr"), Some(Lang::Fr));
        assert_eq!(Lang::from_code("es"), Some(Lang::Es));
        assert_eq!(Lang::from_code("de"), Some(Lang::De));
        assert_eq!(Lang::from_code("it"), None);
        assert_eq!(Lang::from_code(""), None);
        // Deliberately strict: no case folding or region tags — the
        // config documents exactly these codes, and a fuzzy match here
        // would just delay the clear startup error to a subtler place.
        assert_eq!(Lang::from_code("EN"), None);
        assert_eq!(Lang::from_code("fr_FR"), None);
    }

    #[test]
    fn code_round_trips_through_from_code_for_every_language() {
        for lang in ALL {
            assert_eq!(Lang::from_code(lang.code()), Some(lang));
        }
    }

    #[test]
    fn parameterized_messages_embed_their_arguments_in_every_language() {
        for lang in ALL {
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
        for lang in ALL {
            let commands: Vec<&str> = lang.command_reference().iter().map(|(c, _)| *c).collect();
            assert_eq!(en, commands, "command syntax diverged in {lang:?}");
        }
    }

    #[test]
    fn help_reference_has_the_same_keys_in_every_language() {
        // Same invariant as `command_reference` above, for `?`'s full
        // keybinding reference — the key column is real keys, not prose.
        let en: Vec<&str> = Lang::En.help_reference().iter().map(|(k, _)| *k).collect();
        for lang in ALL {
            let keys: Vec<&str> = lang.help_reference().iter().map(|(k, _)| *k).collect();
            assert_eq!(en, keys, "help key syntax diverged in {lang:?}");
        }
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
        let en = keys(Lang::En);
        for lang in ALL {
            assert_eq!(en, keys(lang), "keys diverged in {lang:?}");
        }
    }

    #[test]
    fn markers_fit_their_reserved_breadcrumb_column() {
        for lang in ALL {
            let width = lang.marker_width() as usize;
            for marker in [
                lang.marker_read_only(),
                lang.marker_unmounted(),
                lang.marker_archived(),
            ] {
                assert!(
                    marker.chars().count() <= width,
                    "{marker:?} overflows the {width}-cell marker column in {lang:?}"
                );
            }
        }
    }

    #[test]
    fn mode_line_covers_every_mode_for_every_language() {
        // A regression guard for the match arms above: if a `Mode`
        // variant is ever added without extending every language's
        // `mode_line`, this panics via the `unreachable!` rather than
        // one language silently missing a mode's hints.
        for lang in ALL {
            for mode in [
                Mode::Normal,
                Mode::Insert,
                Mode::Search,
                Mode::Backlinks,
                Mode::EditBody,
                Mode::TagResults,
                Mode::TagList,
                Mode::Links,
                Mode::Help,
            ] {
                let (label, hints) = lang.mode_line(mode);
                assert!(!label.is_empty());
                assert!(!hints.is_empty());
            }
        }
    }
}
