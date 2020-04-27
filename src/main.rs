use bendy::decoding::{Decoder, Object};
use std::{fs, str};

fn main() {
    match Decoder::new(&fs::read("big-buck-bunny.torrent").unwrap())
        .next_object()
        .unwrap()
    {
        None => (),
        Some(Object::List(d)) => println!("{:?}", d),
        Some(Object::Dict(mut d)) => {

            while let Ok(Some(pair)) = d.next_pair() {
                match pair {
                    (b"announce", Object::Bytes(a)) => println!("{:?}", str::from_utf8(a)),
                    _ => ()
                }
            }
        }
        Some(Object::Integer(d)) => println!("{:?}", d),
        Some(Object::Bytes(d)) => println!("{:?}", d),
    }
}
