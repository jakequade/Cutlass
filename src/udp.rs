/// TODO: get rid of println! usage. It should tidy up the conditionals quite well.
extern crate rand;
extern crate tokio;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::{
    convert::TryInto,
    error::Error,
    io::Cursor,
    net::{Ipv4Addr, SocketAddrV4},
    str,
};

use crate::torrent::Torrent;

use rand::Rng;

use tokio::net::UdpSocket;

#[derive(Debug)]
struct TrackerConnectResponseSuccessful {
    action: u32,
    connection: u64,
    transaction: u32,
}

pub async fn attempt_download(torrent: &Torrent) -> Result<(), Box<dyn Error>> {
    let mut socket =
        tokio::net::UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 7777)).await?;

    match make_connect_request(&mut socket, torrent).await {
        Ok(connection_data) => {
            println!("{:?}", connection_data);
            let peers = make_announce_request(&connection_data, &mut socket, &torrent);
            Ok(())
        }
        Err(e) => {
            println!("Couldn't connect: {:?}", e);
            Err(e)
        }
    }
}

/// Creates the initial connection request to the tracker.
/// As per [BEP](http://www.bittorrent.org/beps/bep_0015.html) there are three message requirements:
/// 1. At offset 0, a 64-bit integer `connection_id`, with the value of `0x41727101980`
/// 2. At offset 8, a 32-bit integer `action`, which is `0`, as we're initiating a connection.
/// 3. At offset 12, a 32-bit integer `transaction_id`, which is randomised.
///
/// Successful responses return a 32-bit response:
/// 1. At offset 0, a 32-bit integer `action`, which is `0`
/// 2. At offset 4, a 32-bit integer `transaction_id`
/// 3. At offset 8, a 64-bit integer `connection_id`
async fn make_connect_request(
    socket: &mut UdpSocket,
    torrent: &Torrent
) -> Result<TrackerConnectResponseSuccessful, Box<dyn Error>> {
    let mut rnd = rand::thread_rng();
    let mut message = vec![];
    
    socket.connect(&torrent.get_announce_url()).await?;

    // Send param #1
    message.write_u64::<BigEndian>(0x41727101980).unwrap();
    // Send param #2
    message.write_u32::<BigEndian>(0).unwrap();
    // Send param #3
    message
        .write_u32::<BigEndian>(rnd.gen_range(0, 50))
        .unwrap();

    match socket.send_to(&message, &torrent.get_announce_url()).await {
        Ok(_) => {
            let mut buf = [0; 16];

            match socket.recv_from(&mut buf).await {
                // Successful connection
                Ok(_) => {
                    let mut reader = Cursor::new(buf);

                    // Needs to be in this order, as it's how the data is returned.
                    let action_attempt = reader.read_u32::<BigEndian>();
                    let transaction_attempt = reader.read_u32::<BigEndian>();
                    let connection_attempt = reader.read_u64::<BigEndian>();

                    match (action_attempt, connection_attempt, transaction_attempt) {
                        (Ok(action), Ok(connection), Ok(transaction)) => {
                            println!("Connection successful.");

                            Ok(TrackerConnectResponseSuccessful {
                                action,
                                connection,
                                transaction,
                            })
                        }
                        _ => {
                            println!("Couldn't parse connection data");
                            Err(Box::from("Could not parse connection data."))
                        }
                    }
                }
                Err(e) => {
                    println!("Some other error");
                    Err(Box::from(format!(
                        "Error receiving connect response {:?}",
                        e
                    )))
                }
            }
        }
        Err(e) => {
            println!("Tracker connection error: {:?}", e);
            Err(Box::from(format!("Error connecting to tracker: {:?}", e)))
        }
    }
}

async fn make_announce_request(
    connection_data: &TrackerConnectResponseSuccessful,
    socket: &mut UdpSocket,
    torrent: &Torrent
) -> Result<(), Box<dyn Error>> {
    let mut buffer: Vec<u8> = vec![];
    let mut rnd = rand::thread_rng();

    // Connection ID
    buffer
        .write_u64::<BigEndian>(connection_data.connection)
        .unwrap();
    // Action
    buffer.write_u32::<BigEndian>(1);
    // Transaction ID
    buffer.write_u32::<BigEndian>(rnd.gen_range(0, 100));
    // Info hash
    // buffer.write_int::<BigEndian>(torrent.info_hash, 20);
    // Peer ID TODO: make this alphanumeric
    buffer.write_int::<BigEndian>(rnd.gen_range(0, 100), 20);
    // Downloaded (64)
    buffer.write_u64::<BigEndian>(0);
    // Left (64)
    buffer.write_u64::<BigEndian>(torrent.get_torrent_total_size() as u64);
    // Uploaded (64)
    buffer.write_u64::<BigEndian>(0);
    // Event (32) - 0: none, 1: completed; 2: started; 3: stopped;
    buffer.write_u32::<BigEndian>(0);
    // IP Address (32) - 0
    buffer.write_u32::<BigEndian>(0);
    // Key (32) - random
    buffer.write_u32::<BigEndian>(rnd.gen_range(0, 1000));
    // Num want (32)
    buffer.write_i32::<BigEndian>(-1);
    // Port (16) - between 6881 and 6889
    buffer.write_u16::<BigEndian>(6881);


    Ok(())
}

// fn create_peer_id() {}
