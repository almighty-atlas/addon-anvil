use crate::{App, Message, Page, design::Colors, key};
use addon_anvil::model::ProviderKind;
use addon_anvil::providers;
use iced::widget::{
    Space, button, column, container, pick_list, row, scrollable, text, text_input,
};
use iced::{Alignment, Border, Color, Element, Length, Theme};
use std::collections::HashSet;

const MARGIN: f32 = 18.0;

// All major visual decisions use semantic roles, not ad-hoc RGB values.
// A valid optional matugen file can override the built-in dark palette.
pub fn theme(_: &App) -> Theme {
    let c = Colors::active();
    Theme::custom(
        "AddonAnvil · Dux inspired",
        iced::theme::Palette {
            background: c.background,
            text: c.on_surface,
            primary: c.primary,
            success: Color::from_rgb8(88, 198, 150),
            danger: c.error,
            warning: Color::from_rgb8(244, 190, 105),
        },
    )
}

fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

fn surface_style(_: &Theme) -> container::Style {
    let c = Colors::active();
    container::Style {
        background: Some(c.surface_container.into()),
        text_color: Some(c.on_surface),
        ..Default::default()
    }
}

fn card_style(_: &Theme) -> container::Style {
    let c = Colors::active();
    container::Style {
        background: Some(c.surface_container_high.into()),
        text_color: Some(c.on_surface),
        border: Border {
            color: with_alpha(c.outline, 0.24),
            width: 1.0,
            radius: 18.0.into(),
        },
        ..Default::default()
    }
}

fn nav_button_style(theme: &Theme, state: button::Status, active: bool) -> button::Style {
    let c = Colors::active();
    let mut style = if active {
        button::primary(theme, state)
    } else {
        button::secondary(theme, state)
    };
    style.background = Some(
        if active {
            c.primary
        } else if matches!(state, button::Status::Hovered) {
            c.surface_container_high
        } else {
            c.surface_container
        }
        .into(),
    );
    style.text_color = if active { c.on_primary } else { c.on_surface };
    style.border = Border {
        radius: 14.0.into(),
        ..Default::default()
    };
    style
}

fn heading<'a>(title: &str, detail: &str) -> Element<'a, Message> {
    let c = Colors::active();
    column![
        text("ADDONANVIL  /  WORLD OF WARCRAFT")
            .size(12)
            .color(c.primary),
        text(title.to_owned()).size(30),
        text(detail.to_owned()).size(14).color(c.on_surface_variant),
    ]
    .spacing(7)
    .into()
}

fn side_button<'a>(name: &'static str, page: Page, active: Page) -> Element<'a, Message> {
    button(text(name).size(15))
        .width(Length::Fill)
        .padding(14)
        .style(move |theme, state| nav_button_style(theme, state, page == active))
        .on_press(Message::Navigate(page))
        .into()
}

pub fn view(app: &App) -> Element<'_, Message> {
    let side = container(
        column![
            row![
                text("◆").size(28).color(Colors::active().primary),
                text("AddonAnvil").size(22),
            ]
            .spacing(11)
            .align_y(Alignment::Center),
            text("FOREVER ADDON MANAGER")
                .size(11)
                .color(Colors::active().on_surface_variant),
            Space::new().height(22),
            side_button("Installierte Addons", Page::Installed, app.page),
            side_button("Addons entdecken", Page::Discover, app.page),
            side_button("ZIP installieren", Page::Local, app.page),
            side_button("Einstellungen", Page::Settings, app.page),
            side_button("Über AddonAnvil", Page::About, app.page),
        ]
        .spacing(8)
        .width(Length::Fill),
    )
    .padding(20)
    .width(252)
    .height(Length::Fill)
    .style(surface_style);

    let content = match app.page {
        Page::Installed => installed(app),
        Page::Discover => discover(app),
        Page::Local => local(app),
        Page::Settings => settings(app),
        Page::About => about(),
    };
    let game_picker: Element<'_, Message> = if app.config.installations.is_empty() {
        row![
            text("Keine Installation gefunden"),
            button("Clientordner auswählen").on_press(Message::AddGame)
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .into()
    } else {
        let options: Vec<String> = app
            .config
            .installations
            .iter()
            .map(|i| i.id.clone())
            .collect();
        row![
            text("Installation:").size(13),
            pick_list(
                options,
                app.config.selected_installation.clone(),
                Message::SelectGame
            )
            .width(Length::FillPortion(2)),
            button("Hinzufügen").on_press(Message::AddGame)
        ]
        .align_y(Alignment::Center)
        .spacing(12)
        .into()
    };
    let mut center = column![game_picker, content]
        .spacing(18)
        .width(Length::Fill);
    if let Some(error) = &app.error {
        center = center.push(
            container(row![
                text(format!("Fehler: {error}"))
                    .size(13)
                    .color(Colors::active().error),
            ])
            .padding(12)
            .style(card_style),
        );
    }
    if let Some((provider, id)) = &app.confirm_remove {
        center = center.push(
            container(
                column![
                    text(format!(
                        "{} ({id}) wirklich deinstallieren?",
                        provider.label()
                    ))
                    .size(16),
                    text("Nur zugeordnete Addon-Ordner werden entfernt. WTF bleibt erhalten.")
                        .size(13),
                    row![
                        button("Abbrechen").on_press(Message::CancelRemove),
                        button("Jetzt deinstallieren")
                            .style(button::danger)
                            .on_press(Message::RemoveConfirmed)
                    ]
                    .spacing(12)
                ]
                .spacing(12),
            )
            .padding(15)
            .style(card_style),
        );
    }
    let status = row![
        text(format!(
            "{} Addon-Ordner  •  {} Updates",
            app.installed.len(),
            available_updates(app)
        ))
        .size(12)
        .color(Colors::active().on_surface_variant),
        Space::new().width(Length::Fill),
        text(if app.busy { "Aktiv: " } else { "" }.to_string() + &app.notice)
            .size(12)
            .color(Colors::active().on_surface_variant),
    ]
    .spacing(16)
    .align_y(Alignment::Center);
    container(column![
        row![
            side,
            container(scrollable(center).height(Length::Fill))
                .padding(30)
                .width(Length::Fill)
                .height(Length::Fill)
        ]
        .height(Length::Fill),
        container(status)
            .padding(13)
            .width(Length::Fill)
            .style(surface_style)
    ])
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn available_updates(app: &App) -> usize {
    let Some(game) = app.config.selected() else {
        return 0;
    };
    app.config
        .managed
        .iter()
        .filter(|m| m.installation_id == game.id)
        .filter(|m| {
            app.updates
                .get(&key(m.provider, &m.project_id))
                .and_then(|r| r.release.as_ref())
                .is_some_and(|r| {
                    providers::release_is_newer(&m.release_id, r) && r.download_url.is_some()
                })
        })
        .count()
}

fn installed(app: &App) -> Element<'_, Message> {
    let mut body = column![
        heading(
            "Installierte Addons",
            "Installierte Pakete, Herkunft und Updates"
        ),
        row![
            text_input("Addons filtern …", &app.query)
                .on_input(Message::SearchChanged)
                .width(Length::Fill),
            button("Aktualisieren").on_press(Message::Refresh),
            button("Updates prüfen").on_press(Message::CheckUpdates),
            button("Alle aktualisieren").on_press(Message::UpdateAll)
        ]
        .spacing(10),
        row![
            button(if app.only_updates {
                "Alle anzeigen"
            } else {
                "Nur Updates"
            })
            .on_press(Message::ShowOnlyUpdates(!app.only_updates)),
            text(format!(
                "Letzte Prüfung: {}",
                app.config.last_check.as_deref().unwrap_or("noch keine")
            ))
            .size(12)
            .color(Colors::active().on_surface_variant)
        ]
        .spacing(18)
        .align_y(Alignment::Center),
    ]
    .spacing(19);
    let Some(game) = app.config.selected() else {
        return body
            .push(text("Füge eine Forever-Installation hinzu."))
            .into();
    };
    let mut owned = HashSet::new();
    let mut visible = 0_usize;
    for item in app
        .config
        .managed
        .iter()
        .filter(|m| m.installation_id == game.id)
    {
        for folder in &item.folders {
            owned.insert(folder.as_str());
        }
        let latest = app.updates.get(&key(item.provider, &item.project_id));
        let update = latest
            .and_then(|r| r.release.as_ref())
            .is_some_and(|release| {
                providers::release_is_newer(&item.release_id, release)
                    && release.download_url.is_some()
            });
        if app.only_updates && !update {
            continue;
        }
        if !item.name.to_lowercase().contains(&app.query.to_lowercase()) {
            continue;
        }
        let folder = item
            .folders
            .first()
            .and_then(|name| app.installed.iter().find(|a| a.folder == *name));
        let compatibility = folder.map_or("unbekannt", |a| compat_text(a.compatible));
        let detail = format!(
            "{} · {} · {} · {}",
            item.provider.label(),
            item.release_id,
            compatibility,
            if update {
                "Update verfügbar"
            } else {
                "installiert"
            }
        );
        let mut buttons = row![button("Deinstallieren").style(button::secondary).on_press(
            Message::ConfirmRemove(item.provider, item.project_id.clone())
        )]
        .spacing(8);
        if let Some(remote) = latest.filter(|_| update) {
            buttons = buttons.push(
                button("Aktualisieren")
                    .style(button::primary)
                    .on_press(Message::UpdateOne(remote.clone())),
            );
        }
        body = body.push(addon_card(&item.name, &detail, buttons.into()));
        visible += 1;
    }
    for item in &app.installed {
        if owned.contains(item.folder.as_str()) || app.only_updates {
            continue;
        }
        let title = item.toc.title.as_deref().unwrap_or(&item.folder);
        if !title.to_lowercase().contains(&app.query.to_lowercase())
            && !item
                .folder
                .to_lowercase()
                .contains(&app.query.to_lowercase())
        {
            continue;
        }
        let version = item.toc.version.as_deref().unwrap_or("Version unbekannt");
        let detail = format!(
            "Manuell installiert · {version} · {}",
            compat_text(item.compatible)
        );
        let body_text = item
            .toc
            .notes
            .as_deref()
            .unwrap_or("Keine offizielle Quelle zugeordnet");
        body = body.push(
            container(
                column![
                    text(title.to_owned()).size(18),
                    text(detail).size(13).color(Colors::active().on_surface_variant),
                    text(body_text.to_owned()).size(12),
                ]
                .spacing(6),
            )
            .padding(MARGIN)
            .width(Length::Fill)
            .style(card_style),
        );
        visible += 1;
    }
    if visible == 0 {
        body = body.push(text("Keine Addons für den aktuellen Filter gefunden."));
    }
    body.into()
}

fn compat_text(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "Forever-kompatibel",
        Some(false) => "Inkompatibel",
        None => "Kompatibilität unbekannt",
    }
}

fn addon_card<'a>(name: &str, detail: &str, actions: Element<'a, Message>) -> Element<'a, Message> {
    container(
        row![
            column![
                text(name.to_owned()).size(18),
                text(detail.to_owned())
                    .size(13)
                    .color(Colors::active().on_surface_variant),
            ]
            .spacing(7)
            .width(Length::Fill),
            actions
        ]
        .align_y(Alignment::Center)
        .spacing(12),
    )
    .padding(MARGIN)
    .width(Length::Fill)
    .style(card_style)
    .into()
}

fn discover(app: &App) -> Element<'_, Message> {
    let mut content = column![
        heading(
            "Addons entdecken",
            "Öffentliche Projektquellen und verifizierte Releases"
        ),
        row![
            button("GitHub")
                .style(if app.provider == ProviderKind::GitHub {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(Message::SourceSelected(ProviderKind::GitHub)),
            button("CurseForge")
                .style(if app.provider == ProviderKind::CurseForge {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(Message::SourceSelected(ProviderKind::CurseForge)),
            button("WoWInterface")
                .style(if app.provider == ProviderKind::WoWInterface {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(Message::SourceSelected(ProviderKind::WoWInterface))
        ]
        .spacing(10),
        row![
            text_input("Projekt suchen …", &app.query)
                .on_input(Message::SearchChanged)
                .on_submit(Message::SearchOnline)
                .width(Length::Fill),
            button("Suchen").on_press(Message::SearchOnline)
        ]
        .spacing(10),
    ]
    .spacing(16);
    if app.provider == ProviderKind::WoWInterface {
        content = content.push(
            container(text("WoWInterface: Keine verifizierte Forever-Katalogsuche. Projektlinks können unten hinzugefügt werden.").size(13))
                .padding(MARGIN)
                .style(card_style)
        );
    }
    if app.provider == ProviderKind::CurseForge && std::env::var("CURSEFORGE_API_KEY").is_err() {
        content = content.push(
            container(text("CurseForge benötigt CURSEFORGE_API_KEY aus der Umgebung.").size(13))
                .padding(MARGIN)
                .style(card_style)
        );
    }
    for remote in &app.discovered {
        let mut actions =
            row![button("Projektseite").on_press(Message::OpenUrl(remote.project_url.clone()))]
                .spacing(8);
        if remote.restriction.is_none() && app.config.selected().is_some() {
            actions = actions.push(
                button("Installieren")
                    .style(button::primary)
                    .on_press(Message::InstallRemote(remote.clone())),
            );
        }
        let detail = format!(
            "{} · {}",
            remote.provider.label(),
            remote
                .release
                .as_ref()
                .map(|r| r.version.as_str())
                .unwrap_or("Release wird bei Installation geprüft")
        );
        content = content.push(addon_card(&remote.name, &detail, actions.into()));
        if !remote.summary.is_empty() {
            content = content.push(text(remote.summary.clone()).size(13));
        }
        if let Some(reason) = &remote.restriction {
            content = content.push(text(reason.clone()).size(12));
        }
    }
    content = content.push(text("Direkte GitHub- oder WoWInterface-Projekt-URL").size(16));
    content = content.push(
        row![
            text_input("https://github.com/autor/addon", &app.link)
                .on_input(Message::LinkChanged)
                .width(Length::Fill),
            button("Aus URL installieren").on_press(Message::AddUrl)
        ]
        .spacing(10),
    );
    content.into()
}

fn local(app: &App) -> Element<'_, Message> {
    let c = Colors::active();
    column![
        heading("ZIP installieren", "Lokale Erweiterungen sicher importieren"),
        container(
            column![
                text("Addon aus einem Archiv hinzufügen").size(20),
                text("Wähle eine vorhandene ZIP-Datei. AddonAnvil prüft Pfade, Dateien und Konflikte vor dem Installieren.")
                    .size(14)
                    .color(c.on_surface_variant),
                Space::new().height(4),
                button("ZIP-Datei auswählen")
                    .on_press(Message::PickZip)
                    .style(button::primary)
                    .padding(15),
                text(if app.config.selected().is_some() {
                    "Ziel: Interface/AddOns der ausgewählten Installation"
                } else {
                    "Bitte zuerst einen Forever-Clientordner auswählen"
                })
                .size(13)
                .color(c.on_surface_variant),
            ]
            .spacing(16),
        )
        .padding(24)
        .width(Length::Fill)
        .style(card_style),
        container(
            column![
                text("Geschützte Installation").size(16),
                text("Keine Änderungen am WTF-Ordner. Keine Überschreibung fremder Addons. Fehler beim Installieren führen zu einem Rollback.")
                    .size(14)
                    .color(c.on_surface_variant),
            ]
            .spacing(9),
        )
        .padding(20)
        .width(Length::Fill)
        .style(card_style),
    ]
    .spacing(20)
    .into()
}

fn settings(app: &App) -> Element<'_, Message> {
    let mut content = column![
        heading("Einstellungen", "Installationspfade und lokale Daten"),
        button("Installationen erneut erkennen").on_press(Message::DetectGames),
        button("Clientordner manuell hinzufügen").on_press(Message::AddGame),
    ]
    .spacing(18);
    for game in &app.config.installations {
        content = content.push(
            container(
                row![
                    column![
                        text(game.label.clone()).size(17),
                        text(game.path.display().to_string()).size(12),
                        text(format!(
                            "Interface: {}",
                            game.interface
                                .map(|v| v.to_string())
                                .unwrap_or("unbekannt".into())
                        ))
                        .size(12),
                    ]
                    .width(Length::Fill)
                    .spacing(4),
                    button("Nicht mehr verwalten").on_press(Message::ForgetGame(game.id.clone()))
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            )
            .padding(MARGIN)
            .style(card_style),
        );
    }
    if let Some(paths) = &app.paths {
        content = content.push(
            container(
                column![
                    text("Lokale Dateien").size(17),
                    text(format!("Konfiguration: {}", paths.config_file().display()))
                        .size(12)
                        .color(Colors::active().on_surface_variant),
                    text(format!("Cache: {}", paths.cache.display()))
                        .size(12)
                        .color(Colors::active().on_surface_variant),
                ]
                .spacing(9),
            )
            .padding(20)
            .width(Length::Fill)
            .style(card_style),
        );
    }
    content = content.push(text(
        "CurseForge-Schlüssel: ausschließlich CURSEFORGE_API_KEY, nicht in config.json gespeichert"
    ).size(13));
    content.into()
}

fn about<'a>() -> Element<'a, Message> {
    let c = Colors::active();
    column![
        heading("Über AddonAnvil", "Native Addon-Verwaltung für World of Warcraft Forever"),
        container(
            column![
                row![
                    text("◆").size(40).color(c.primary),
                    column![
                        text("AddonAnvil").size(25),
                        text(concat!("Version ", env!("CARGO_PKG_VERSION"), "  ·  MIT-Lizenz"))
                            .size(13)
                            .color(c.on_surface_variant),
                    ]
                    .spacing(6),
                ]
                .spacing(16)
                .align_y(Alignment::Center),
                text("Entwickelt für Linux, Wayland und Niri. Rust und iced — ohne Electron und ohne integrierten Browser.")
                    .size(14),
                text("Kein Konto, keine Telemetrie, kein Hintergrunddienst. Dateien werden nur auf ausdrückliche Nutzeraktion verändert.")
                    .size(14)
                    .color(c.on_surface_variant),
                button("GitHub-Projekt öffnen")
                    .on_press(Message::OpenUrl("https://github.com/almighty-atlas/addon-anvil".into()))
                    .style(button::primary),
            ]
            .spacing(19),
        )
        .padding(24)
        .width(Length::Fill)
        .style(card_style),
        text("Dux-inspirierte Gestaltung. Unabhängiges Community-Projekt, keine offizielle Dux-Shell-Erweiterung.")
            .size(12)
            .color(c.on_surface_variant),
    ]
    .spacing(20)
    .into()
}
