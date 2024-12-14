use arguments::Args;
use babylonia_terminal_sdk::game_config::GameConfig;
use clap::Parser;
use log::debug;

pub mod arguments;
pub mod game;
pub mod reporter;
pub mod utils;

pub fn run() {
    let args = Args::parse();

    debug!("Launch option -> {:?}", args.options);

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            if let Some(command) = args.set_options {
                GameConfig::set_launch_options(Some(command))
                    .await
                    .expect("Failed to save launch options into the config file");
            }

            game::run(args.options, args.logs).await;
        });
}
