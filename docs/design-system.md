# AddonAnvil Design System — Dux Shell inspired

Stand: 2026-11-04. **Eigenständige Rust/iced-Anwendung**, keine QML-Erweiterung, kein offizielles Dux-Produkt. Die Begriffe „Dux-inspiriert“ bezeichnen Gestaltungsprinzipien und keine Marken- oder Kompatibilitätszusage.

## Primärquellen

- [Dux: Theme Property Reference](https://github.com/duxlabs-dev/dux-shell/blob/master/.agents/skills/dms-plugin-dev/references/theme-reference.md) — semantische Farben, Typografie, Abstände, Radien, Widget-Muster
- [Dux: Plugin Development](https://duxlabs.dev/docs/dankmaterialshell/plugin-development/) — Navigation, Flächen, Eingaben, visuelle Zustände
- [Dux: Application Theming](https://duxlabs.dev/docs/dankmaterialshell/application-themes/) — eigene matugen-Templates im Benutzerverzeichnis
- [Dux: Custom Themes](https://duxlabs.dev/docs/dankmaterialshell/custom-themes) — Light/Dark, Kontrast und Reaktivität

Die Quellwerte sind **QML-Theme-Eigenschaften**, keine in iced importierbaren Variablen. Wir übernehmen die Rollen, nicht QML-Code. Falls die Originalquellen geändert werden, müssen die Adaptionen überprüft werden.

## Gestaltungsgrundlagen

| Rolle | iced-Umsetzung | Zweck |
| --- | --- | --- |
| `surface` / `background` | Fenstergrund | Basisfläche |
| `surfaceContainer` | Sidebar und Fußleiste | Sekundäre UI |
| `surfaceContainerHigh` | Karten / Panels | zusammengehörige Inhalte |
| `onSurface` | Haupttext | hoher Kontrast |
| `onSurfaceVariant` | Beschreibung / Metadaten | sekundäre Information |
| `primary`, `onPrimary` | Primäraktion, aktive Navigation | fokussierte Interaktion |
| `outline` | subtile Kartenrahmen | Trennung ohne dominante Linien |
| `error`, `success`, `warning` | Status | **nicht allein durch Farbe** kommunizieren |

**Typografie:** Dux dokumentiert 12 / 14 / 16 / 20 px für Standardrollen. AddonAnvil nutzt 12 px Status, 14 px Beschreibung, 16 px Interaktion, 20 px Überschriften und eine 28–30-px-Seitenüberschrift für klare Hierarchie.

**Layout:** 8-px-Grundraster; kompakte Abstände 4/8 px, Standard 12/16 px, Gruppenabstand 24 px, Außenrand 28 px. Die Werte sind **AddonAnvil-Adaptionen**, nicht als exakte Dux-Konstanten ausgegeben.

**Form:** Kacheln mit 16–20 px Rundung, aktive Navigationsflächen und Aktionsbuttons als Pills mit 12–16 px Rundung; subtile Umrandungen. Iced-Fokus-/Hover-Zustände müssen sichtbar bleiben.

**Dichte und Zugriff:** Im Fenster 1190×760 soll die Sidebar (~235 px) erhalten bleiben, Inhaltsbereiche scrollen. Keine abgeschnittenen Buttons oder nur durch Hover erreichbaren Kernaktionen. Farbkontrast für primäre Texte mindestens WCAG-AA-Ziel (4,5:1); Status zusätzlich beschriften. Tastaturnavigation beim manuellen Wayland-Test prüfen.

## Fünf Screens und gemeinsame Komponenten

1. **Installiert:** Header, Such-/Update-Aktionsbereich, Karten pro Addon; Quelle, Version und Kompatibilität sind Texte und kein bloßes Icon. Löschen bleibt bestätigungspflichtig.
2. **Entdecken:** Einzige Provider-Auswahl für GitHub/CurseForge/WoWInterface, Suchfeld, Ergebnis-Karten, manuelle Projekt-URL; Provider-Einschränkungen sichtbar.
3. **Lokale ZIP:** Primäraktion zum Dateidialog, Hinweis zum Zielordner, Sicherheitsinfo zur ZIP-Prüfung.
4. **Einstellungen:** gruppierte Installationskarten, Erkennen/Hinzufügen, technische lokale Pfade, Secret-Hinweis; keine Persistierung von CurseForge-Schlüsseln.
5. **Über:** Produkt- und Versionsidentität, Plattform/Datenschutz, Repo-Link.

Gemeinsam: Seitennavigation, Header, Karten, Hilfstexte, Buttons, Fehlerbox, Fußleiste und Installationsauswahl. Gleichartige Komponenten sollen gleiche Abstände, Typografie und Zustände verwenden.

## Dynamische Dux-Farben

Die optionale matugen-Anbindung ist **explizit opt-in**:

- Vorlage: `packaging/matugen/addon-anvil.json`
- Ausgabedatei: `${XDG_CONFIG_HOME:-~/.config}/addon-anvil/colors.json`
- Farbschlüssel: `background`, `surface_container`, `surface_container_high`, `on_surface`, `on_surface_variant`, `primary`, `on_primary`, `outline`, `error`.
- Lesen nur vom lokalen Benutzerkonfigurationspfad. Kein Shell-Aufruf, kein Netzwerk, kein Zugriff auf Dux-Prozesse.
- Bei fehlender/ungültiger Datei: eingebautes Dark-Theme. Ohne Dux funktioniert die App eigenständig.
- Die Datei enthält **ausschließlich Farbdaten**; keine Konfiguration für Addons oder Accounts.

### Einrichtung

1. `mkdir -p ~/.config/matugen/templates`
2. `cp packaging/matugen/addon-anvil.json ~/.config/matugen/templates/addon-anvil.json`
3. In `~/.config/matugen/config.toml` unter `[config]` folgenden Abschnitt ergänzen (absolute Pfade anpassen):

   ```toml
   [templates.addon_anvil]
   input_path = '/home/USER/.config/matugen/templates/addon-anvil.json'
   output_path = '/home/USER/.config/addon-anvil/colors.json'
   ```

4. Dux-Shell-Farbschema/Wallpaper wechseln, damit matugen die Datei erzeugt.

**Hinweis:** AddonAnvil liest JSON und nutzt iced für die Darstellung. Ein Qt-/GTK-Theme allein färbt iced nicht. Die Dokumentation nennt den Kompatibilitätsstand und muss bei Änderungen der matugen-/Dux-API gepflegt werden.

## Verifikation / Release-Gate

- `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release --locked`
- Live-Screenshots aus dem jeweils **aktuellen** Binary für alle fünf Seiten, nicht aus einem veralteten Release-Artefakt.
- Test ohne matugen-Datei und mit gültiger/ungültiger Palette; kein Absturz.
- Manuelle Prüfung unter **Niri/Wayland**, einschließlich Fokus, Schrift-Skalierung, langen Addon-Namen und Dateidialog.
- Screenshots aus Xvfb sind ein visuelles Regressionstool, **kein Wayland-Test**.

## Nicht-Ziele

Kein Qt/QML-Rewrite, keine Dux-Laufzeit-Abhängigkeit, keine Kopie geschützter Icons/Schriftdateien und keine Änderung an Installations-/Update-Sicherheitslogik durch das UI-Redesign.
