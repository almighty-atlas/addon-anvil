use crate::{App, Message, Page, design::Colors, key};
use addon_anvil::model::{GameInstallation, ProviderKind};
use addon_anvil::providers;
use iced::widget::{
    Space, button, column, container, pick_list, row, scrollable, text, text_input,
};
use iced::{Alignment, Border, Color, Element, Length, Theme};
use std::collections::HashSet;
use std::fmt;

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

// Dux uses expressive rounded controls. iced's stock buttons/fields have
// almost-square corners, so ALL action widgets use these shared styles.
#[derive(Clone, Copy)]
enum ButtonTone {
    Primary,
    Secondary,
    Quiet,
    Danger,
}

fn blend(base: Color, over: Color, amount: f32) -> Color {
    Color {
        r: base.r * (1.0 - amount) + over.r * amount,
        g: base.g * (1.0 - amount) + over.g * amount,
        b: base.b * (1.0 - amount) + over.b * amount,
        a: base.a,
    }
}

fn styled_button(theme: &Theme, status: button::Status, tone: ButtonTone) -> button::Style {
    let c = Colors::active();
    let (background, foreground) = match tone {
        ButtonTone::Primary => (c.primary, c.on_primary),
        ButtonTone::Secondary => (c.surface_container_high, c.on_surface),
        ButtonTone::Quiet => (c.surface_container, c.on_surface),
        ButtonTone::Danger => (c.error, c.on_primary),
    };
    let current = match status {
        button::Status::Hovered => blend(background, c.on_surface, 0.08),
        button::Status::Pressed => blend(background, c.on_surface, 0.16),
        _ => background,
    };
    let disabled = matches!(status, button::Status::Disabled);
    let mut style = button::secondary(theme, status);
    style.background = Some(with_alpha(current, if disabled { 0.45 } else { 1.0 }).into());
    style.text_color = with_alpha(foreground, if disabled { 0.55 } else { 1.0 });
    style.border = Border {
        radius: 14.0.into(),
        color: if matches!(tone, ButtonTone::Secondary) {
            with_alpha(c.outline, 0.3)
        } else {
            Color::TRANSPARENT
        },
        width: if matches!(tone, ButtonTone::Secondary) { 1.0 } else { 0.0 },
    };
    style
}

fn primary_button(theme: &Theme, status: button::Status) -> button::Style {
    styled_button(theme, status, ButtonTone::Primary)
}

fn secondary_button(theme: &Theme, status: button::Status) -> button::Style {
    styled_button(theme, status, ButtonTone::Secondary)
}

fn danger_button(theme: &Theme, status: button::Status) -> button::Style {
    styled_button(theme, status, ButtonTone::Danger)
}

fn nav_button_style(theme: &Theme, status: button::Status, active: bool) -> button::Style {
    styled_button(
        theme,
        status,
        if active { ButtonTone::Primary } else { ButtonTone::Quiet },
    )
}

fn action<'a>(label: impl Into<Element<'a, Message>>) -> button::Button<'a, Message> {
    button(label).padding([10, 16]).style(secondary_button)
}

fn field_style(_: &Theme, status: text_input::Status) -> text_input::Style {
    let c = Colors::active();
    let outline = if matches!(status, text_input::Status::Focused { .. }) {
        c.primary
    } else {
        with_alpha(c.outline, 0.5)
    };
    text_input::Style {
        background: c.surface_container_high.into(),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: outline,
        },
        icon: c.on_surface_variant,
        placeholder: c.on_surface_variant,
        value: c.on_surface,
        selection: c.primary,
    }
}

fn choice_style(_: &Theme, status: pick_list::Status) -> pick_list::Style {
    let c = Colors::active();
    let selected = matches!(status, pick_list::Status::Hovered | pick_list::Status::Opened { .. });
    pick_list::Style {
        background: c.surface_container_high.into(),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: if selected { c.primary } else { with_alpha(c.outline, 0.5) },
        },
        text_color: c.on_surface,
        placeholder_color: c.on_surface_variant,
        handle_color: c.on_surface_variant,
    }
}

// Show short, distinguishable labels; the original absolute path remains the
// selection ID and is never rewritten. Full paths remain visible in Settings.
#[derive(Debug, Clone, PartialEq, Eq)]
struct GameChoice {
    id: String,
    label: String,
}

impl fmt::Display for GameChoice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.label)
    }
}

fn game_choice(game: &GameInstallation) -> GameChoice {
    let parent = game.path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("WoW");
    let leaf = game.path.file_name().and_then(|n| n.to_str()).unwrap_or("Client");
    GameChoice {
        id: game.id.clone(),
        label: format!("{} · {parent}/{leaf}", game.label),
    }
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
    action(text(name).size(15))
        .width(Length::Fill)
        .padding([13, 16])
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
        column![
            text("Keine Installation gefunden").size(14),
            action("Clientordner auswählen").on_press(Message::AddGame)
        ]
        .spacing(10)
        .width(Length::Fill)
        .into()
    } else {
        let options: Vec<GameChoice> = app.config.installations.iter().map(game_choice).collect();
        let selected = options
            .iter()
            .find(|option| Some(option.id.as_str()) == app.config.selected_installation.as_deref())
            .cloned();
        column![
            row![
                text("Installation").size(14),
                Space::new().width(Length::Fill),
                action("Hinzufügen").on_press(Message::AddGame),
            ]
            .align_y(Alignment::Center),
            pick_list(options, selected, |choice| Message::SelectGame(choice.id))
                .width(Length::Fill)
                .padding([10, 14])
                .style(choice_style),
        ]
        .spacing(8)
        .width(Length::Fill)
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
                        action("Abbrechen").on_press(Message::CancelRemove),
                        action("Jetzt deinstallieren")
                            .style(danger_button)
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
        text_input("Addons filtern …", &app.query)
            .on_input(Message::SearchChanged)
            .padding([12, 14])
            .style(field_style)
            .width(Length::Fill),
        row![
            action("Aktualisieren").on_press(Message::Refresh),
            action("Updates prüfen").on_press(Message::CheckUpdates),
            action("Alle aktualisieren")
                .style(primary_button)
                .on_press(Message::UpdateAll)
        ]
        .spacing(10),
        row![
            action(if app.only_updates {
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
        let mut buttons = row![action("Deinstallieren").style(secondary_button).on_press(
            Message::ConfirmRemove(item.provider, item.project_id.clone())
        )]
        .spacing(8);
        if let Some(remote) = latest.filter(|_| update) {
            buttons = buttons.push(
                action("Aktualisieren")
                    .style(primary_button)
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
                    text(title.to_owned()).size(18).wrapping(text::Wrapping::WordOrGlyph).width(Length::Fill),
                    text(detail)
                        .size(13)
                        .color(Colors::active().on_surface_variant),
                    text(body_text.to_owned()).size(12).wrapping(text::Wrapping::WordOrGlyph).width(Length::Fill),
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
        column![
            text(name.to_owned())
                .size(18)
                .width(Length::Fill)
                .wrapping(text::Wrapping::WordOrGlyph),
            text(detail.to_owned())
                .size(13)
                .width(Length::Fill)
                .wrapping(text::Wrapping::WordOrGlyph)
                .color(Colors::active().on_surface_variant),
            actions,
        ]
        .spacing(10)
        .width(Length::Fill),
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
            action("GitHub")
                .style(if app.provider == ProviderKind::GitHub {
                    primary_button
                } else {
                    secondary_button
                })
                .on_press(Message::SourceSelected(ProviderKind::GitHub)),
            action("CurseForge")
                .style(if app.provider == ProviderKind::CurseForge {
                    primary_button
                } else {
                    secondary_button
                })
                .on_press(Message::SourceSelected(ProviderKind::CurseForge)),
            action("WoWInterface")
                .style(if app.provider == ProviderKind::WoWInterface {
                    primary_button
                } else {
                    secondary_button
                })
                .on_press(Message::SourceSelected(ProviderKind::WoWInterface))
        ]
        .spacing(10),
        row![
            text_input("Projekt suchen …", &app.query)
                .padding([12, 14])
                .style(field_style)
                .on_input(Message::SearchChanged)
                .on_submit(Message::SearchOnline)
                .width(Length::Fill),
            action("Suchen").on_press(Message::SearchOnline)
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
                .style(card_style),
        );
    }
    for remote in &app.discovered {
        let mut actions =
            row![action("Projektseite").on_press(Message::OpenUrl(remote.project_url.clone()))]
                .spacing(8);
        if remote.restriction.is_none() && app.config.selected().is_some() {
            actions = actions.push(
                action("Installieren")
                    .style(primary_button)
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
                .padding([12, 14])
                .style(field_style)
                .on_input(Message::LinkChanged)
                .width(Length::Fill),
            action("Aus URL installieren").on_press(Message::AddUrl)
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
                action("ZIP-Datei auswählen")
                    .on_press(Message::PickZip)
                    .style(primary_button)
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
        row![
            action("Installationen erneut erkennen").on_press(Message::DetectGames),
            action("Clientordner manuell hinzufügen").on_press(Message::AddGame),
        ]
        .spacing(10),
    ]
    .spacing(18);
    for game in &app.config.installations {
        content = content.push(
            container(
                column![
                    text(game.label.clone()).size(17),
                    text(game.path.display().to_string())
                        .size(12)
                        .width(Length::Fill)
                        .wrapping(text::Wrapping::WordOrGlyph)
                        .color(Colors::active().on_surface_variant),
                    text(format!(
                        "Interface: {}",
                        game.interface
                            .map(|v| v.to_string())
                            .unwrap_or("unbekannt".into())
                    ))
                    .size(12),
                    action("Nicht mehr verwalten")
                        .style(danger_button)
                        .on_press(Message::ForgetGame(game.id.clone()))
                ]
                .spacing(10)
                .width(Length::Fill),
            )
            .padding(MARGIN)
            .width(Length::Fill)
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
                action("GitHub-Projekt öffnen")
                    .on_press(Message::OpenUrl("https://github.com/almighty-atlas/addon-anvil".into()))
                    .style(primary_button),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn game_picker_label_is_compact_but_retains_identity() {
        let game = GameInstallation {
            id: "/home/atlas/Games/battlenet/drive_c/Program Files (x86)/World of Warcraft/_forever_".into(),
            label: "Forever (_forever_)".into(),
            path: PathBuf::from("/home/atlas/Games/battlenet/drive_c/Program Files (x86)/World of Warcraft/_forever_"),
            interface: Some(12902),
        };
        let choice = game_choice(&game);
        assert_eq!(choice.id, game.id);
        assert!(choice.to_string().contains("World of Warcraft/_forever_"));
        assert!(!choice.to_string().contains("/home/atlas/Games"));
    }

    #[test]
    fn all_button_variants_have_expressive_corner_radius() {
        let theme = Theme::Dark;
        for tone in [ButtonTone::Primary, ButtonTone::Secondary, ButtonTone::Quiet, ButtonTone::Danger] {
            let style = styled_button(&theme, button::Status::Active, tone);
            assert_eq!(style.border.radius.top_left, 14.0);
            assert!(style.background.is_some());
        }
    }
}
