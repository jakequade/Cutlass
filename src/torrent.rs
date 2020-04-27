// Mostly inspired from https://github.com/P3KI/bendy/blob/master/examples/decode_torrent.rs#L40
// Torrent structure: https://en.wikipedia.org/wiki/Torrent_file#File_structure

use bendy::{
    decoding::{Error as DecodingError, FromBencode, Object, ResultExt},
    encoding::{AsString, Error as EncodingError, SingleItemEncoder, ToBencode},
};

use crypto::{digest::Digest, sha1::Sha1};

#[derive(Debug)]
pub struct Torrent {
    pub announce: String,
    pub info: Info,
    pub info_hash: String,
}

#[derive(Clone, Debug)]
pub struct Info {
    pub files: Vec<File>,
    pub piece_length: String,
    pub pieces: Vec<u8>,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct File {
    pub path: String,
    pub length: usize,
}

//#region Torrent FromBencode

impl FromBencode for Torrent {
    fn decode_bencode_object(object: Object) -> Result<Self, DecodingError>
    where
        Self: Sized,
    {
        let mut announce = None;
        let mut info = None;
        let mut info_hash = None;

        let mut dict_dec = object.try_into_dictionary()?;

        while let Some(pair) = dict_dec.next_pair()? {
            match pair {
                (b"announce", value) => {
                    announce = String::decode_bencode_object(value)
                        .context("announce")
                        .map(Some)?;
                }
                (b"info", value) => {
                    info = Info::decode_bencode_object(value)
                        .context("info")
                        .map(Some)?;
                    
                    // Encoding `info` back to bencode in order to get the SHA1 hash
                    let mut hash = info.clone().unwrap().clone().to_bencode();
                    let mut hasher = Sha1::new();
                    hasher.input_str(std::str::from_utf8(&hash.unwrap()).unwrap());
                    info_hash = Some(hasher.result_str().to_string());
                }
                _ => (),
            }
        }

        let announce = announce.ok_or_else(|| DecodingError::missing_field("announce"))?;
        let info = info.ok_or_else(|| DecodingError::missing_field("info"))?;
        let info_hash = info_hash.expect("Could not create info hash for torrent.");

        Ok(Torrent { announce, info, info_hash })
    }
}

impl Torrent {
    /// Removes the `udp://` prefix from announce URL, as that's the format needed
    /// when connecting socket.
    pub fn get_announce_url(&self) -> String {
        self.announce.replace("udp://", "").to_string()
    }

    pub fn get_torrent_total_size(&self) -> usize {
        self.info.files.iter().map(|file| file.length).sum()
    }
}

//#endregion

//#region Info FromBencode

impl FromBencode for Info {
    fn decode_bencode_object(object: Object) -> Result<Self, DecodingError>
    where
        Self: Sized,
    {
        let mut files: Vec<File> = vec![];
        let mut name = None;
        let mut piece_length = None;
        let mut pieces = None;

        let mut dict_dec = object.try_into_dictionary()?;
        while let Some(pair) = dict_dec.next_pair()? {
            match pair {
                (b"name", value) => {
                    name = String::decode_bencode_object(value)
                        .context("name")
                        .map(Some)?;
                }
                (b"piece length", value) => {
                    piece_length = value
                        .try_into_integer()
                        .context("length")
                        .map(ToString::to_string)
                        .map(Some)?;
                }
                (b"pieces", value) => {
                    pieces = AsString::decode_bencode_object(value)
                        .context("pieces")
                        .map(|bytes| Some(bytes.0))?;
                }
                (b"files", Object::List(mut value)) => {
                    while let Some(Object::Dict(mut file_dict)) = value.next_object()? {
                        let mut file_length: Option<usize> = None;
                        let mut file_path: Option<String> = None;

                        while let Some(pair) = file_dict.next_pair()? {
                            match pair {
                                (b"length", Object::Integer(b)) => {
                                    file_length = b.to_string().parse::<usize>().ok();
                                }
                                (b"path", Object::List(mut b)) => {
                                    let mut path_pieces: Vec<String> = vec![];
                                    while let Some(piece) = b.next_object()? {
                                        match piece {
                                            Object::Bytes(p) => {
                                                std::str::from_utf8(p).ok().map(|piece_str| {
                                                    path_pieces.push(piece_str.to_string())
                                                });
                                            }
                                            _ => (),
                                        }
                                    }

                                    file_path = Some(path_pieces.join("/"));
                                }
                                _ => (),
                            }
                        }

                        match (file_length, file_path) {
                            (Some(length), Some(path)) => {
                                files.push(File { length, path });
                            }
                            _ => (),
                        }
                    }
                }
                _ => (),
            }
        }

        let name = name.ok_or_else(|| DecodingError::missing_field("name"))?;
        let piece_length =
            piece_length.ok_or_else(|| DecodingError::missing_field("piece_length"))?;
        let pieces = pieces.ok_or_else(|| DecodingError::missing_field("pieces"))?;

        Ok(Info {
            files,
            name,
            piece_length,
            pieces,
        })
    }
}

impl ToBencode for Info {
    const MAX_DEPTH: usize = 3;

    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), EncodingError> {
        encoder.emit_dict(|mut e| {
            e.emit_pair(b"files", &self.files)?;
            e.emit_pair(b"name", &self.name)?;
            e.emit_pair(b"piece_length", &self.piece_length)?;
            e.emit_pair(b"pieces", &self.pieces)
        })?;

        Ok(())
    }
}

impl ToBencode for File {
    const MAX_DEPTH: usize = 1;

    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), EncodingError> {
        encoder.emit_dict(|mut e| {
            e.emit_pair(b"length", &self.length)?;
            e.emit_pair(b"path", &self.path)
        })?;

        Ok(())
    }
}

//#endregion
