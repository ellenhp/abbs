use std::{
    fs::{read_dir, File},
    io::Read,
    sync::Arc,
};

use bbs::BbsApp;
use config::Config;
use log::info;
use migrator::Migrator;
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use ssh_ui::{russh_keys::decode_secret_key, AppServer};
use tokio::spawn;
use ui::library::{push_library, Library};

pub(crate) mod bbs;
pub(crate) mod db;
pub(crate) mod migrator;
pub(crate) mod ui;
pub(crate) mod user;

#[macro_use]
extern crate tantivy;
#[macro_use]
extern crate lazy_static;

async fn setup_db(db_url: &str, reset: bool) -> Result<DatabaseConnection, anyhow::Error> {
    let db = Database::connect(db_url).await?;
    if reset {
        Migrator::fresh(&db).await?;
    } else {
        Migrator::up(&db, None).await?;
    }
    Ok(db)
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let settings = Config::builder()
        .add_source(config::File::with_name("Config").required(false))
        .add_source(config::Environment::with_prefix("ABBS"))
        .build()
        .unwrap();

    let db_url = settings
        .get_string("db_url")
        .expect("Need DB_URL config value.");
    let port = settings.get_int("listen_port").map_or_else(
        |_| {
            info!("No LISTEN_PORT set, using default value 22");
            22u16
        },
        |port| port as u16,
    );
    let reset = settings.get_bool("unsafe_db_reset").unwrap_or(false);
    info!("Loading database from URL '{}'", &db_url);
    let db = setup_db(&db_url, reset)
        .await
        .expect("Failed to load database.");

    let library_path = settings
        .get_string("library_path")
        .unwrap_or("./library.zim".into());
    spawn(async {
        let lib = Library::open::<String>("library", library_path, "_search_index".into())
            .await
            .expect("Failed to open library");
        println!("Opened library");
        push_library(lib);
    });

    info!("Using port {}", port);
    let mut server = AppServer::new_with_port(port);
    let bbs_app = BbsApp::new_with_db(db);
    let keys = read_dir(".")
        .unwrap()
        .filter_map(Result::ok)
        .filter(|key_path| {
            key_path
                .path()
                .file_name()
                .map(|name| name.to_string_lossy().starts_with("ssh_host_"))
                == Some(true)
        })
        .map(|key_path| File::open(key_path.path()))
        .filter_map(Result::ok)
        .map(|mut key_file| {
            let mut key_buf = String::new();
            key_file.read_to_string(&mut key_buf).unwrap();
            decode_secret_key(&key_buf, None)
        })
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    info!("Starting server...");
    server.run(&keys, Arc::new(bbs_app)).await.unwrap();
}
