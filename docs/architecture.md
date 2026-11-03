# Architektur

| Modul | Aufgabe |
| --- | --- |
| src/main.rs, src/ui.rs | Native iced 0.14 Shell, Zustandsübergänge, Views, Tasks |
| src/model.rs, src/config.rs | Typisierte Metadaten, XDG-JSON, Schema-Version |
| src/wow.rs, src/toc.rs | Begrenzte Installationserkennung, Validierung, TOC-Parser |
| src/providers/ | GitHub, MMOUI, offizielle CurseForge-API |
| src/archive.rs | ZIP-Eintragsprüfung, Größenlimits, Ordnerzuordnung |
| src/installer.rs | Transaktion, Backups, Commit, Rollback, kontrollierte Deinstallation |
| src/error.rs | Typisierte Domänenfehler |

Provider-Abfragen und Downloads laufen asynchron, ZIP-Operationen und Dateisystemscans in spawn_blocking. Kleine Konfigurationsänderungen schreiben atomare JSON-Dateien. Kein Daemon, kein Hintergrundkonto, keine Telemetrie.

Verwaltete Addons werden durch Installation, Provider und Projekt-ID identifiziert. Zugehörige Addon-Unterordner werden explizit hinterlegt; ein Paket mit Modulen wird als eine Einheit behandelt. Nicht zugeordnete manuelle Addons werden ausschließlich gelesen.

## Abgrenzungen

Es gibt noch kein dauerhaftes Transaktionsjournal zur Wiederherstellung nach Stromausfällen, keine Dateihash-basierte Fremdänderungserkennung, keine sichere Zuordnungsoberfläche für manuelle Addons und keine verlässliche Prozesszuordnung zu nur einer WoW-Installation.
