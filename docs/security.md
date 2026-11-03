# Sicherheitsmodell

1. Externe Archive dürfen nur via HTTPS heruntergeladen werden (auch nach Weiterleitung).
2. ZIP-Archive: maximal 128 MiB komprimiert, 384 MiB entpackt, 12.000 Einträge; Path Traversal und Symlinks werden zurückgewiesen.
3. Extraktion erfolgt nur in temporären Verzeichnissen auf dem Ziel-Dateisystem, nicht direkt über bestehenden Addons.
4. Kollisionen mit nicht verwalteten Ordnern führen zum Abbruch; manuelle TOC-Namen werden nicht automatisch einer Quelle zugeordnet.
5. Installation und Deinstallation nutzen bekannte Ordnernamen, legen Sicherungen an und rollen bei Fehlern vor dem Metadaten-Commit zurück.
6. Der WTF-Ordner und SavedVariables werden nicht gelesen, verändert oder gelöscht.
7. Konfigurationsdateien werden atomar ersetzt; beschädigte oder unbekannte Versionen blockieren das Schreiben.
8. API-Schlüssel werden nur aus Umgebungsvariablen gelesen und nicht protokolliert.

## Verbleibende Risiken

Die ownership-Datenbank erfasst zunächst Ordnernamen, keine Datei-Hashes. Wird ein zuvor verwalteter Ordner nachträglich außerhalb von AddonAnvil ersetzt, kann dessen Eigentümerschaft nicht sicher nachgewiesen werden. Bearbeite verwaltete Ordner daher bis zur Einführung eines Dateimanifests nicht extern.

Ein Stromausfall zwischen Ordnerumbenennungen kann Backup-Verzeichnisse hinterlassen; diese sind manuell wiederherzustellen. Die Prozessterkennung über /proc ist konservativ, aber nicht lückenlos. Fremde Addon-Pakete sind nicht kryptografisch authentifiziert: nur vertrauenswürdige Quellen verwenden.
