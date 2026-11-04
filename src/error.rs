use thiserror::Error;

#[derive(Debug, Error)]
pub enum AnvilError {
    #[error("Dateisystemfehler: {0}")]
    Io(#[from] std::io::Error),
    #[error("Konfiguration beschädigt; Datei wurde nicht überschrieben: {0}")]
    Config(#[from] serde_json::Error),
    #[error("Netzwerkfehler: {0}")]
    Network(#[from] reqwest::Error),
    #[error("ZIP-Datei fehlerhaft: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Ungültiger Pfad oder unsicheres Archiv: {0}")]
    UnsafePath(String),
    #[error("Die WoW-Installation konnte nicht validiert werden: {0}")]
    InvalidGame(String),
    #[error("Dateikonflikt mit einem nicht verwalteten Addon: {0}")]
    Conflict(String),
    #[error("Addon-Quelle nicht verfügbar: {0}")]
    Provider(String),
    #[error("Download oder Archiv zu groß")]
    TooLarge,
    #[error("Keine unterstützte Forever-Version in diesem Paket gefunden")]
    Incompatible,
    #[error("Bitte World of Warcraft vor Dateioperationen vollständig schließen")]
    GameRunning,
    #[error("Operation nicht möglich: {0}")]
    InvalidOperation(String),
}

pub type Result<T> = std::result::Result<T, AnvilError>;
