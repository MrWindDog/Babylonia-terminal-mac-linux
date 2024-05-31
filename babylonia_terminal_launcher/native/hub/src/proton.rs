use std::thread;

use babylonia_terminal_sdk::{
    components::{
        component_downloader::ComponentDownloader,
        proton_component::{ProtonComponent, PROTON_DEV, PROTON_REPO},
    },
    game_manager::GameManager,
    game_state::GameState,
    utils::github_requester::{GithubRelease, GithubRequester},
};
use rinf::debug_print;
use tokio_with_wasm::tokio;

use crate::{
    github::GithubInfo,
    messages::{
        error::ReportError,
        steps::proton::{
            NotifiyProtonSuccessfullyInstalled, NotifyProtonStartDecompressing,
            ProtonDownloadProgress, StartProtonInstallation,
        },
    },
};

pub async fn listen_proton_installation() {
    let mut receiver = StartProtonInstallation::get_dart_signal_receiver();
    while let Some(info) = receiver.recv().await {
        let releases: Result<Vec<GithubRelease>, _> =
            GithubInfo::get_github_releases(PROTON_DEV, PROTON_REPO).await;
        if releases.is_err() {
            ReportError {
                error_message: format!("When fetching proton versions : {}", releases.unwrap_err()),
            }
            .send_signal_to_dart();
            continue;
        }

        let checked_releases = releases.unwrap();
        let release_index = checked_releases
            .iter()
            .position(|v| v.tag_name == info.message.proton_version);

        if release_index.is_none() {
            ReportError {
                error_message: "Failed to find Proton version".to_string(),
            }
            .send_signal_to_dart();
            continue;
        }

        thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    match GameManager::install_wine(
                        GameState::get_config().await.config_dir,
                        release_index.unwrap(),
                        Some(DownloadReporter::create()),
                    )
                    .await
                    {
                        Err(e) => ReportError {
                            error_message: format!("Failed to install Proton : {}", e),
                        }
                        .send_signal_to_dart(),
                        Ok(_) => NotifiyProtonSuccessfullyInstalled {}.send_signal_to_dart(),
                    }
                });
        });
    }
}

struct DownloadReporterPrivate {
    max_progress: Option<u64>,
}

struct DownloadReporter {
    private: std::sync::Mutex<Option<DownloadReporterPrivate>>,
}

impl DownloadReporter {
    pub fn create() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            private: std::sync::Mutex::new(None),
        })
    }
}

impl downloader::progress::Reporter for DownloadReporter {
    fn setup(&self, max_progress: Option<u64>, _: &str) {
        let private = DownloadReporterPrivate { max_progress };

        let mut guard = self.private.lock().unwrap();
        *guard = Some(private);
    }

    fn progress(&self, current: u64) {
        if let Some(p) = self.private.lock().unwrap().as_mut() {
            ProtonDownloadProgress {
                current,
                max: p.max_progress.unwrap(),
            }
            .send_signal_to_dart();
        }
    }

    fn set_message(&self, _: &str) {}

    fn done(&self) {
        NotifyProtonStartDecompressing {}.send_signal_to_dart();
        let mut guard = self.private.lock().unwrap();
        *guard = None;
    }
}
