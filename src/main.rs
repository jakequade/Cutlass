mod torrent;
mod udp;

use bendy::decoding::{FromBencode}; 
use std::{fs};

use crate::torrent::Torrent;

#[tokio::main]
async fn main() {
    match fs::read("big-buck-bunny.torrent") {
        Ok(file) => {
            match Torrent::from_bencode(&file) {
                Ok(torrent) => {
                    let _data = udp::attempt_download(&torrent).await;
                }
                _ => {
                }
            }
        }
        _ => ()
    };
}
