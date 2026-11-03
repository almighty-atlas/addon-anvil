# Provider-Schnittstellen

## GitHub

Die öffentliche REST API liefert Repositories und Releases. AddonAnvil akzeptiert offizielle https://github.com/owner/repo-Links. Es werden ausschließlich hochgeladene ZIP-Assets betrachtet, niemals automatisch erzeugte Quellcodearchive. Genau ein Forever-spezifisches oder einziges generisches ZIP kann gewählt werden; der enthaltene Forever-TOC muss später trotzdem positiv bestätigt werden. Bei mehreren uneindeutigen ZIP-Assets wird nicht geraten.

## WoWInterface

Direkte /downloads/infoNNNN.html-Projektlinks werden angenommen. Die öffentliche MMOUI API ist unter api.mmoui.com zugänglich. Die Version und ein HTTPS-Download können über /v3/game/WOW/filedetails/NNNN.json gelesen werden, sofern diese Metadaten bereitgestellt werden. Die aktuelle API hat keine verlässlich dokumentierte Forever-spezifische Volltextsuche; diese Funktion ist bewusst deaktiviert. Der konkrete Antworttyp muss bei Netzwerkzugang überprüft werden.

## CurseForge

Nur die offizielle https://api.curseforge.com/v1 API mit dem benutzerseitig bereitgestellten CURSEFORGE_API_KEY wird verwendet. WoW besitzt gameId 1; die Forever-Version wird über /games/1/version-types ermittelt. Fehlende Forever-Zuordnung führt zu einer Fehlermeldung statt Retail/Classic als Ersatz zu wählen. Suchen und Dateilisten filtern nach gameVersionTypeId. allowModDistribution=false oder eine fehlende offizielle HTTPS-Download-URL verhindern den Download. Keine Web-Scraper oder CDN-URL-Manipulationen.

## Wago und TukUI

Nicht implementiert, da die Priorität auf nachvollziehbaren und genehmigten APIs liegt.

Referenzen:

- https://docs.curseforge.com/rest-api/
- https://warcraft.wiki.gg/wiki/TOC_format
- https://github.com/Topher121/AddonForge
- https://github.com/LightforgeProject/lightforge
