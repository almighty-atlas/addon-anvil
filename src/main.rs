//! AddonAnvil's native iced shell. All filesystem and network work is delegated
//! to separate services and run via asynchronous iced Tasks.
mod ui;

use addon_anvil::config::Paths;
use addon_anvil::installer;
use addon_anvil::model::{
    AppConfig, GameInstallation, InstallRequest, ProviderKind, RemoteAddon, ScannedAddon,
};
use addon_anvil::providers::{self, Catalog};
use addon_anvil::{toc, wow};
use futures_util::future::join_all;
use iced::Task;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Page {
    Installed,
    Discover,
    Local,
    Settings,
    About,
}

#[derive(Debug, Clone)]
pub(crate) enum Message {
    Navigate(Page),
    SearchChanged(String),
    LinkChanged(String),
    SourceSelected(ProviderKind),
    GamesDetected(Vec<GameInstallation>),
    DetectGames,
    AddGame,
    GamePicked(Option<PathBuf>),
    SelectGame(String),
    ForgetGame(String),
    Refresh,
    Scanned(Result<Vec<ScannedAddon>, String>),
    SearchOnline,
    SearchDone(Result<Vec<RemoteAddon>, String>),
    AddUrl,
    RemoteResolved(Result<RemoteAddon, String>),
    InstallRemote(RemoteAddon),
    PickZip,
    ZipPicked(Option<PathBuf>),
    ConfirmRemove(ProviderKind, String),
    CancelRemove,
    RemoveConfirmed,
    OperationDone(Result<AppConfig, String>),
    CheckUpdates,
    UpdatesDone(Vec<(String, Result<RemoteAddon, String>)>),
    UpdateOne(RemoteAddon),
    UpdateAll,
    BulkDone((AppConfig, Vec<String>)),
    OpenUrl(String),
    UrlOpened(Result<(), String>),
    ShowOnlyUpdates(bool),
}

pub(crate) struct App {
    paths: Option<Paths>,
    config: AppConfig,
    read_only: bool,
    page: Page,
    provider: ProviderKind,
    query: String,
    link: String,
    installed: Vec<ScannedAddon>,
    discovered: Vec<RemoteAddon>,
    updates: HashMap<String, RemoteAddon>,
    busy: bool,
    notice: String,
    error: Option<String>,
    confirm_remove: Option<(ProviderKind, String)>,
    only_updates: bool,
}

fn key(provider: ProviderKind, id: &str) -> String {
    format!("{}:{id}", provider.label())
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        let (paths, config, read_only, error) = match Paths::discover() {
            Ok(paths) => match paths.load() {
                Ok(config) => (Some(paths), config, false, None),
                Err(e) => (Some(paths), AppConfig::default(), true, Some(e.to_string())),
            },
            Err(e) => (None, AppConfig::default(), true, Some(e.to_string())),
        };
        let mut state = Self {
            paths,
            config,
            read_only,
            page: Page::Installed,
            provider: ProviderKind::GitHub,
            query: String::new(),
            link: String::new(),
            installed: Vec::new(),
            discovered: Vec::new(),
            updates: HashMap::new(),
            busy: false,
            notice: "Bereit".into(),
            error,
            confirm_remove: None,
            only_updates: false,
        };
        let initial_scan = state.refresh();
        let detect = Task::perform(
            async {
                tokio::task::spawn_blocking(wow::scan_candidates)
                    .await
                    .unwrap_or_default()
            },
            Message::GamesDetected,
        );
        (state, Task::batch([initial_scan, detect]))
    }

    fn write_config(&mut self, next: AppConfig) -> bool {
        if self.read_only {
            self.error = Some("Konfiguration ist schreibgeschützt; bitte Fehler beheben".into());
            return false;
        }
        let Some(paths) = &self.paths else {
            return false;
        };
        if let Err(e) = paths.save(&next) {
            self.error = Some(e.to_string());
            return false;
        }
        self.config = next;
        true
    }

    fn refresh(&mut self) -> Task<Message> {
        let Some(game) = self.config.selected().cloned() else {
            self.installed.clear();
            return Task::none();
        };
        self.notice = "Lese Addons …".into();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    toc::scan_installed(&game.addons_dir(), game.interface)
                        .map_err(|e| e.to_string())
                })
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r)
            },
            Message::Scanned,
        )
    }

    fn start_install(&mut self, remote: RemoteAddon) -> Task<Message> {
        if self.busy || self.read_only {
            return Task::none();
        }
        let Some(game) = self.config.selected() else {
            self.error = Some("Bitte zuerst eine WoW-Installation auswählen".into());
            return Task::none();
        };
        if remote
            .release
            .as_ref()
            .and_then(|r| r.download_url.as_deref())
            .is_none()
        {
            self.error = Some(
                remote
                    .restriction
                    .unwrap_or("Kein zulässiger Download vorhanden".into()),
            );
            return Task::none();
        }
        let Some(paths) = self.paths.clone() else {
            return Task::none();
        };
        self.busy = true;
        self.error = None;
        self.notice = format!("Lade {} herunter und installiere …", remote.name);
        let cfg = self.config.clone();
        let game_id = game.id.clone();
        Task::perform(
            async move { install_remote(paths, cfg, game_id, remote).await },
            Message::OperationDone,
        )
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Navigate(page) => {
                self.page = page;
                Task::none()
            }
            Message::SearchChanged(text) => {
                self.query = text;
                Task::none()
            }
            Message::LinkChanged(text) => {
                self.link = text;
                Task::none()
            }
            Message::SourceSelected(source) => {
                self.provider = source;
                self.discovered.clear();
                Task::none()
            }
            Message::ShowOnlyUpdates(only) => {
                self.only_updates = only;
                Task::none()
            }
            Message::DetectGames => {
                if self.busy {
                    return Task::none();
                }
                self.notice = "Suche typische Wine-/Lutris-/Proton-Installationen …".into();
                Task::perform(
                    async {
                        tokio::task::spawn_blocking(wow::scan_candidates)
                            .await
                            .unwrap_or_default()
                    },
                    Message::GamesDetected,
                )
            }
            Message::GamesDetected(found) => {
                let mut next = self.config.clone();
                let count = found.len();
                for game in found {
                    if !next.installations.iter().any(|i| i.id == game.id) {
                        next.installations.push(game);
                    }
                }
                if next.selected_installation.is_none() {
                    next.selected_installation = next.installations.first().map(|i| i.id.clone());
                }
                if !self.read_only && self.write_config(next) {
                    self.notice = format!("{count} Installation(en) in typischen Pfaden gefunden");
                    return self.refresh();
                }
                Task::none()
            }
            Message::AddGame => {
                if self.busy {
                    return Task::none();
                }
                Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Forever-Clientordner auswählen")
                            .pick_folder()
                            .await
                            .map(|h| h.path().to_path_buf())
                    },
                    Message::GamePicked,
                )
            }
            Message::GamePicked(Some(path)) => {
                let path = if path.join("_forever_").is_dir() {
                    path.join("_forever_")
                } else {
                    path
                };
                match wow::validate(&path) {
                    Ok(game) => {
                        let mut next = self.config.clone();
                        if !next.installations.iter().any(|g| g.id == game.id) {
                            next.installations.push(game.clone());
                        }
                        next.selected_installation = Some(game.id);
                        if self.write_config(next) {
                            self.updates.clear();
                            return self.refresh();
                        }
                    }
                    Err(e) => self.error = Some(e.to_string()),
                }
                Task::none()
            }
            Message::GamePicked(None) => Task::none(),
            Message::SelectGame(id) => {
                if self.busy {
                    return Task::none();
                }
                let mut next = self.config.clone();
                next.selected_installation = Some(id);
                if self.write_config(next) {
                    self.updates.clear();
                    return self.refresh();
                }
                Task::none()
            }
            Message::ForgetGame(id) => {
                if self.busy {
                    return Task::none();
                }
                let mut next = self.config.clone();
                next.installations.retain(|g| g.id != id);
                next.managed.retain(|m| m.installation_id != id);
                if next.selected_installation.as_deref() == Some(id.as_str()) {
                    next.selected_installation = next.installations.first().map(|g| g.id.clone());
                }
                if self.write_config(next) {
                    self.updates.clear();
                    return self.refresh();
                }
                Task::none()
            }
            Message::Refresh => self.refresh(),
            Message::Scanned(result) => {
                match result {
                    Ok(addons) => {
                        self.installed = addons;
                        self.notice = format!("{} Addon-Ordner erkannt", self.installed.len());
                    }
                    Err(error) => self.error = Some(error),
                }
                Task::none()
            }
            Message::SearchOnline => {
                if self.busy {
                    return Task::none();
                }
                self.busy = true;
                self.notice = "Suche Addons …".into();
                self.error = None;
                let provider = self.provider;
                let query = self.query.clone();
                Task::perform(
                    async move {
                        let cat = Catalog::new().map_err(|e| e.to_string())?;
                        cat.search(provider, &query)
                            .await
                            .map_err(|e| e.to_string())
                    },
                    Message::SearchDone,
                )
            }
            Message::SearchDone(result) => {
                self.busy = false;
                match result {
                    Ok(items) => {
                        self.notice = format!("{} Suchergebnisse", items.len());
                        self.discovered = items;
                    }
                    Err(error) => {
                        self.discovered.clear();
                        self.error = Some(error);
                    }
                }
                Task::none()
            }
            Message::AddUrl => {
                if self.busy {
                    return Task::none();
                }
                let link = self.link.trim();
                let source = match providers::github::parse_project(link) {
                    Ok(id) => Ok((ProviderKind::GitHub, id)),
                    Err(_) => providers::wowinterface::parse_project(link)
                        .map(|id| (ProviderKind::WoWInterface, id)),
                };
                let (provider, id) = match source {
                    Ok(value) => value,
                    Err(e) => {
                        self.error = Some(format!(
                            "GitHub- oder WoWInterface-Projekt-URL benötigt: {e}"
                        ));
                        return Task::none();
                    }
                };
                self.busy = true;
                self.error = None;
                self.notice = "Prüfe Projekt und Release …".into();
                Task::perform(
                    async move {
                        let cat = Catalog::new().map_err(|e| e.to_string())?;
                        cat.latest(provider, &id).await.map_err(|e| e.to_string())
                    },
                    Message::RemoteResolved,
                )
            }
            Message::RemoteResolved(result) => {
                self.busy = false;
                match result {
                    Ok(remote) => self.start_install(remote),
                    Err(e) => {
                        self.error = Some(e);
                        Task::none()
                    }
                }
            }
            Message::InstallRemote(remote) => {
                if remote.release.is_none() {
                    if self.busy {
                        return Task::none();
                    }
                    self.busy = true;
                    let provider = remote.provider;
                    let id = remote.project_id;
                    self.notice = "Prüfe neuestes Release …".into();
                    Task::perform(
                        async move {
                            let cat = Catalog::new().map_err(|e| e.to_string())?;
                            cat.latest(provider, &id).await.map_err(|e| e.to_string())
                        },
                        Message::RemoteResolved,
                    )
                } else {
                    self.start_install(remote)
                }
            }
            Message::PickZip => {
                if self.busy {
                    return Task::none();
                }
                Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Addon-ZIP auswählen")
                            .add_filter("ZIP-Archiv", &["zip"])
                            .pick_file()
                            .await
                            .map(|h| h.path().to_path_buf())
                    },
                    Message::ZipPicked,
                )
            }
            Message::ZipPicked(Some(zip)) => {
                if self.busy || self.read_only {
                    return Task::none();
                }
                let Some(game) = self.config.selected() else {
                    self.error = Some("Bitte zuerst eine Installation auswählen".into());
                    return Task::none();
                };
                let Some(paths) = self.paths.clone() else {
                    return Task::none();
                };
                let cfg = self.config.clone();
                let id = game.id.clone();
                let filename = zip
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let req = InstallRequest {
                    provider: ProviderKind::Local,
                    project_id: zip.to_string_lossy().into_owned(),
                    name: filename,
                    release_id: "lokale ZIP".into(),
                    project_url: String::new(),
                };
                self.busy = true;
                self.error = None;
                self.notice = "Installiere lokale ZIP …".into();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            installer::install_zip(&paths, &cfg, &id, &zip, req)
                                .map_err(|e| e.to_string())
                        })
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                    },
                    Message::OperationDone,
                )
            }
            Message::ZipPicked(None) => Task::none(),
            Message::ConfirmRemove(provider, id) => {
                self.confirm_remove = Some((provider, id));
                Task::none()
            }
            Message::CancelRemove => {
                self.confirm_remove = None;
                Task::none()
            }
            Message::RemoveConfirmed => {
                if self.busy || self.read_only {
                    return Task::none();
                }
                let Some((provider, project_id)) = self.confirm_remove.take() else {
                    return Task::none();
                };
                let Some(game) = self.config.selected() else {
                    return Task::none();
                };
                let Some(paths) = self.paths.clone() else {
                    return Task::none();
                };
                let cfg = self.config.clone();
                let id = game.id.clone();
                self.busy = true;
                self.notice = "Entferne verwaltete Addon-Dateien …".into();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            installer::uninstall(&paths, &cfg, &id, provider, &project_id)
                                .map_err(|e| e.to_string())
                        })
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                    },
                    Message::OperationDone,
                )
            }
            Message::OperationDone(result) => {
                self.busy = false;
                match result {
                    Ok(config) => {
                        self.config = config;
                        self.updates.clear();
                        self.notice = "Dateioperation erfolgreich abgeschlossen".into();
                        self.error = None;
                        return self.refresh();
                    }
                    Err(e) => self.error = Some(e),
                }
                Task::none()
            }
            Message::CheckUpdates => {
                if self.busy {
                    return Task::none();
                }
                let Some(game) = self.config.selected() else {
                    return Task::none();
                };
                let items: Vec<_> = self
                    .config
                    .managed
                    .iter()
                    .filter(|m| m.installation_id == game.id && m.provider != ProviderKind::Local)
                    .cloned()
                    .collect();
                self.busy = true;
                self.notice = "Prüfe Provider-Versionen …".into();
                Task::perform(
                    async move {
                        let jobs = items.into_iter().map(|item| async move {
                            let result = async {
                                let cat = Catalog::new().map_err(|e| e.to_string())?;
                                cat.latest(item.provider, &item.project_id)
                                    .await
                                    .map_err(|e| e.to_string())
                            }
                            .await;
                            (key(item.provider, &item.project_id), result)
                        });
                        join_all(jobs).await
                    },
                    Message::UpdatesDone,
                )
            }
            Message::UpdatesDone(results) => {
                self.busy = false;
                self.updates.clear();
                let mut failures = Vec::new();
                for (key, result) in results {
                    match result {
                        Ok(remote) => {
                            self.updates.insert(key, remote);
                        }
                        Err(error) => failures.push(error),
                    }
                }
                if failures.is_empty() {
                    let mut config = self.config.clone();
                    config.last_check = Some(chrono::Utc::now().to_rfc3339());
                    let _ = self.write_config(config);
                    self.notice = "Versionsprüfung abgeschlossen".into();
                } else {
                    self.error = Some(format!(
                        "{} Provider fehlgeschlagen: {}",
                        failures.len(),
                        failures.join("; ")
                    ));
                }
                Task::none()
            }
            Message::UpdateOne(remote) => self.start_install(remote),
            Message::UpdateAll => {
                if self.busy || self.read_only {
                    return Task::none();
                }
                let Some(game) = self.config.selected() else {
                    return Task::none();
                };
                let Some(paths) = self.paths.clone() else {
                    return Task::none();
                };
                let game_id = game.id.clone();
                let projects: Vec<RemoteAddon> = self
                    .config
                    .managed
                    .iter()
                    .filter(|m| m.installation_id == game_id)
                    .filter_map(|m| {
                        self.updates
                            .get(&key(m.provider, &m.project_id))
                            .and_then(|r| {
                                r.release
                                    .as_ref()
                                    .filter(|release| {
                                        providers::release_is_newer(&m.release_id, release)
                                            && release.download_url.is_some()
                                    })
                                    .map(|_| r.clone())
                            })
                    })
                    .collect();
                if projects.is_empty() {
                    return Task::none();
                }
                let cfg = self.config.clone();
                self.busy = true;
                self.notice = format!("Installiere {} Updates …", projects.len());
                Task::perform(
                    async move {
                        let mut current = cfg;
                        let mut failures = Vec::new();
                        for project in projects {
                            match install_remote(
                                paths.clone(),
                                current.clone(),
                                game_id.clone(),
                                project.clone(),
                            )
                            .await
                            {
                                Ok(new) => current = new,
                                Err(error) => failures.push(format!("{}: {error}", project.name)),
                            }
                        }
                        (current, failures)
                    },
                    Message::BulkDone,
                )
            }
            Message::BulkDone((cfg, failures)) => {
                self.config = cfg;
                self.busy = false;
                self.updates.clear();
                if failures.is_empty() {
                    self.notice = "Alle verfügbaren Updates installiert".into();
                    self.error = None;
                } else {
                    self.error = Some(failures.join("; "));
                }
                self.refresh()
            }
            Message::OpenUrl(url) => {
                if !url.starts_with("https://") {
                    return Task::none();
                }
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            open::that(url).map_err(|e| e.to_string())
                        })
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                    },
                    Message::UrlOpened,
                )
            }
            Message::UrlOpened(result) => {
                if let Err(e) = result {
                    self.error = Some(e);
                }
                Task::none()
            }
        }
    }
}

async fn install_remote(
    paths: Paths,
    cfg: AppConfig,
    game_id: String,
    addon: RemoteAddon,
) -> Result<AppConfig, String> {
    let release = addon
        .release
        .ok_or_else(|| "Keine Release-Daten verfügbar".to_string())?;
    let url = release.download_url.ok_or_else(|| {
        addon
            .restriction
            .unwrap_or("Download nicht verfügbar".into())
    })?;
    let catalog = Catalog::new().map_err(|e| e.to_string())?;
    let file = catalog
        .download(&url, &paths.cache)
        .await
        .map_err(|e| e.to_string())?;
    let request = InstallRequest {
        provider: addon.provider,
        project_id: addon.project_id,
        name: addon.name,
        release_id: release.id,
        project_url: addon.project_url,
    };
    let file_for_worker = file.clone();
    let result = tokio::task::spawn_blocking(move || {
        installer::install_zip(&paths, &cfg, &game_id, &file_for_worker, request)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|r| r);
    let _ = std::fs::remove_file(file);
    result
}

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "addon_anvil=info".into()),
        )
        .init();
    iced::application(App::boot, App::update, ui::view)
        .title("AddonAnvil — WoW Addon Manager")
        .theme(ui::theme)
        .window_size((1190.0, 760.0))
        .centered()
        .run()
}
