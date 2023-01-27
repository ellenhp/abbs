use std::{
    fs::{read_dir, File},
    io::Read,
    sync::Arc,
};

use bbs::BbsApp;
use ssh_ui::{russh_keys::decode_secret_key, AppServer};
use tokio::spawn;
use ui::library::{push_library, Library};

mod bbs;

pub(crate) mod ui;

#[macro_use]
extern crate tantivy;
#[macro_use]
extern crate lazy_static;

#[tokio::main]
async fn main() {
    spawn(async {
        let lib = Library::open::<String>("library", "library.zim".into(), "_search_index".into())
            .await
            .expect("Failed to open library");
        println!("Opened library");
        push_library(lib);
    });

    let mut server = AppServer::new_with_port(2222);
    let bbs_app = BbsApp {};
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
    server.run(&keys, Arc::new(bbs_app)).await.unwrap();
}
