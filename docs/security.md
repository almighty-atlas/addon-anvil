# Sicherheitsmodell

1. Externe Archive dürfen nur via HTTPS heruntergeladen werden (auch nach Weiterleitung).
2. ZIP-Archive: maximal 128 MiB komprimiert, 384 MiB entpackt, 12.000 Einträge; Path Traversal und Symlinks werden zurückgewiesen.
3. Extraktion erfolgt nur in temporären Verzeichnissen auf dem Ziel-Dateisystem, nicht direkt über bestehenden Addons.
4. Kollisionen mit nicht verwalteten Ordnern führen zum Abbruch; manuelle TOC-Namen werden nicht automatisch einer Quelle zugeordnet.
5. Installation und Deinstallation nutzen gespeicherte SHA-256-Fingerprints für jedes verwaltete Verzeichnis und die Ordnernamen. Externe Änderungen oder fehlende Fingerprints blockieren destruktive Operationen. Sie legen Sicherungen an und rollen bei Fehlern vor dem Metadaten-Commit zurück.
6. Der WTF-Ordner und SavedVariables werden nicht gelesen, verändert oder gelöscht.
7. Konfigurationsdateien werden atomar ersetzt; beschädigte oder unbekannte Versionen blockieren das Schreiben.
8. API-Schlüssel werden nur aus Umgebungsvariablen gelesen und nicht protokolliert.

## Verbleibende Risiken

Verwaltete Verzeichnisse werden gegen SHA-256-Fingerprints geprüft. Eine Inhaltsänderung verhindert die weitere automatische Verwaltung, bis sie manuell geklärt wird. Der Fingerprint ist keine kryptografische Signatur eines Herausgebers und schützt nicht gegen gleichzeitige externe Dateizugriffe zwischen Prüfung und Umbenennung.

Ein Stromausfall zwischen Ordnerumbenennungen kann Backup-Verzeichnisse hinterlassen; diese sind manuell wiederherzustellen. Die Prozessterkennung über /proc ist konservativ, aber nicht lückenlos. Fremde Addon-Pakete sind nicht kryptografisch authentifiziert: nur vertrauenswürdige Quellen verwenden.
