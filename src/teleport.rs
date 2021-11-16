use crate::*;
use byteorder::{LittleEndian, ReadBytesExt};

fn generate_checksum(input: &[u8]) -> u8 {
    input.iter().map(|x| *x as u64).sum::<u64>() as u8
}

fn validate_checksum(input: &[u8]) -> Result<(), Error> {
    if input.len() < 2 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "Vector is too short to validate checksum",
        ));
    }

    let checksum: u8 = input[..input.len() - 1]
        .iter()
        .map(|x| *x as u64)
        .sum::<u64>() as u8;
    if checksum != *input.last().unwrap() {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "Teleport checksum is invalid",
        ));
    }

    Ok(())
}

fn vec_to_string(input: &[u8], size: u16) -> Vec<char> {
    let mut s = Vec::<char>::new();
    for i in input.iter() {
        let c: char = match (*i).try_into() {
            Ok(c) => c,
            Err(_) => break,
        };
        if c.is_ascii_graphic() || c == ' ' {
            s.push(c);
        } else {
            break;
        }
        if s.len() == size.into() {
            break;
        }
    }

    s
}

fn read_version(mut input: &[u8]) -> [u16; 3] {
    let mut out: [u16; 3] = [0; 3];
    for i in &mut out {
        *i = input.read_u16::<LittleEndian>().unwrap();
    }
    out
}

fn write_version(input: [u16; 3]) -> Vec<u8> {
    let mut out = Vec::<u8>::new();
    for i in &input {
        out.append(&mut i.to_le_bytes().to_vec());
    }
    out
}

#[derive(Debug, PartialEq)]
pub struct TeleportHeader {
    protocol: u64,
    //data_len: u32,
    action: TeleportAction,
    iv: Option<[u8; 12]>,
    data: Vec<u8>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TeleportAction {
    Init = 0x01,
    InitAck = 0x02,
    TxFile = 0x10,
    RxFile = 0x20,
    Data = 0x40,
    Encrypted = 0x80,
}

impl TeleportHeader {
    pub fn new() -> TeleportHeader {
        TeleportHeader {
            protocol: PROTOCOL_NEXT,
            action: TeleportAction::Init,
            iv: None,
            data: Vec::<u8>::new(),
        }
    }

    pub fn serialize(&mut self) -> Vec<u8> {
        let mut out = Vec::<u8>::new();

        // Add Protocol identifier
        out.append(&mut self.protocol.to_le_bytes().to_vec());

        // Add data length
        let data_len = self.data.len() as u32;
        out.append(&mut data_len.to_le_bytes().to_vec());

        // Add action code
        let mut action = self.action as u8;
        if self.iv.is_some() {
            action |= TeleportAction::Encrypted as u8;
        }
        out.push(action);

        // If Encrypted, add IV
        if let Some(iv) = self.iv {
            out.append(&mut iv[..].to_vec());
        };

        // Add data
        out.append(&mut self.data.clone());

        out
    }

    pub fn deserialize(&mut self, input: Vec<u8>) -> Result<(), Error> {
        let mut buf: &[u8] = &input;

        // Extract Protocol
        self.protocol = buf.read_u64::<LittleEndian>().unwrap();
        if self.protocol != PROTOCOL_NEXT {
            return Err(Error::new(ErrorKind::InvalidData, "Error reading protocol"));
        }

        // Extract data length
        let data_len = buf.read_u32::<LittleEndian>().unwrap() as usize;

        // Extract action code
        let action = buf.read_u8().unwrap();

        // If Encrypted, extract IV
        if (action & TeleportAction::Encrypted as u8) == TeleportAction::Encrypted as u8 {
            if input.len() < 25 {
                return Err(Error::new(ErrorKind::InvalidData, "Not enough data for IV"));
            }
            let iv: [u8; 12] = input[13..25].try_into().expect("Error reading IV");
            self.iv = Some(iv);
        }

        // Extract data
        self.data = input[25..].to_vec();
        if self.data.len() != data_len {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Data is not the expected length",
            ));
        }

        Ok(())
    }
}
/*
impl TryFrom<u8> for TeleportStatus {
    type Error = &'static str;

    fn try_from(v: u8) -> std::result::Result<Self, Self::Error> {
        match v {
            x if x == TeleportInitStatus::Proceed as u8 => Ok(TeleportInitStatus::Proceed),
            x if x == TeleportInitStatus::Overwrite as u8 => Ok(TeleportInitStatus::Overwrite),
            x if x == TeleportInitStatus::NoOverwrite as u8 => Ok(TeleportInitStatus::NoOverwrite),
            x if x == TeleportInitStatus::NoSpace as u8 => Ok(TeleportInitStatus::NoSpace),
            x if x == TeleportInitStatus::NoPermission as u8 => {
                Ok(TeleportInitStatus::NoPermission)
            }
            x if x == TeleportInitStatus::WrongVersion as u8 => {
                Ok(TeleportInitStatus::WrongVersion)
            }
            _ => Err("TeleportInitStatus is invalid"),
        }
    }
}
*/
#[cfg(test)]
mod tests {
    use super::*;

    const TESTINIT: &[u8] = &[
        84, 69, 76, 69, 80, 79, 82, 84, 17, 0, 0, 0, 129, 5, 48, 46, 50, 46, 51, 0, 246, 9, 10, 11,
        12, 4, 0, 0, 0, 184, 34, 0, 0, 0, 0, 0, 0, 10, 10, 32, 3, 21,
    ];
    const TESTINITIV: &[u8; 12] = &[5, 48, 46, 50, 46, 51, 0, 246, 9, 10, 11, 12];
    const TESTDATA: &[u8] = &[4, 0, 0, 0, 184, 34, 0, 0, 0, 0, 0, 0, 10, 10, 32, 3, 21];

    #[test]
    fn test_teleportheader_serialize() {
        let mut t = TeleportHeader::new();
        t.data.append(&mut TESTDATA.to_vec());
        t.iv = Some(*TESTINITIV);
        let s = t.serialize();
        assert_eq!(s, TESTINIT);
    }

    #[test]
    fn test_teleportheader_deserialize() {
        let mut test = TeleportHeader::new();
        test.data.append(&mut TESTDATA.to_vec());
        test.iv = Some(*TESTINITIV);
        let mut t = TeleportHeader::new();
        t.deserialize(TESTINIT.to_vec());
        assert_eq!(t, test);
    }
}