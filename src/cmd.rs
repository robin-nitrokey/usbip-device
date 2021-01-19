use crate::UsbIpError;
use std::{
   convert::TryInto,
   io::{Error, ErrorKind, Read},
   net::TcpStream,
};

// TODO: Unlink commands

#[repr(C)]
#[derive(Debug, Clone)]
pub struct UsbIpHeader {
   pub command: u32,
   pub seqnum: u32,
   pub devid: u32,
}

impl UsbIpHeader {
   fn to_array(&self) -> [u8; 12] {
      let mut result = [0; 12];

      result[0..4].copy_from_slice(&self.command.to_be_bytes());
      result[4..8].copy_from_slice(&self.seqnum.to_be_bytes());
      result[8..12].copy_from_slice(&self.devid.to_be_bytes());

      result
   }

   fn from_slice(data: &[u8]) -> Self {
      Self {
         command: u32::from_be_bytes(data[0..4].try_into().unwrap()),
         seqnum: u32::from_be_bytes(data[4..8].try_into().unwrap()),
         devid: u32::from_be_bytes(data[8..12].try_into().unwrap()),
      }
   }
}

pub enum UsbIpRequest {
   Cmd(UsbIpHeader, UsbIpCmd, Vec<u8>),
}

impl UsbIpRequest {
   pub fn read(reader: &mut TcpStream) -> Result<Self, Error> {
      // Read an parse header
      reader.set_nonblocking(true)?;
      let mut header_buf = [0; 12];
      match reader.read(&mut header_buf) {
         Ok(bytes_read) if bytes_read == 12 => (),
         Ok(0) => {
            return Err(Error::new(
               ErrorKind::NotConnected,
               Box::new(UsbIpError::ConnectionClosed),
            ))
         }
         Ok(bytes_read) => {
            return Err(Error::new(
               ErrorKind::InvalidInput,
               Box::new(UsbIpError::PkgTooShort(bytes_read)),
            ))
         }
         Err(err) => return Err(err),
      }
      reader.set_nonblocking(false)?;

      let header = UsbIpHeader::from_slice(&header_buf);

      log::debug!(
         "received request with seqnum {} for devid {}",
         header.seqnum,
         header.devid
      );

      match header.command {
         0x00000001 => {
            let mut data_buf = [0; 36];
            reader.read_exact(&mut data_buf)?;
            let command = UsbIpCmd::from_slice(&data_buf);

            // Receive the URB if this is a OUT packet
            let urb_buf = if command.direction == 0 {
               let mut urb_buf = vec![0; command.transfer_buffer_length as usize];

               // NOTE: Reading 0 bytes would still block the reader
               if command.transfer_buffer_length != 0 {
                  reader.read_exact(&mut urb_buf)?;
               }

               urb_buf
            } else {
               vec![]
            };

            log::info!("parsed a command request");
            Ok(Self::Cmd(header, command, urb_buf))
         }
         _ => Err(Error::new(
            ErrorKind::InvalidInput,
            Box::new(UsbIpError::InvalidCommand(header.command as u16)),
         )),
      }
   }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct UsbIpCmd {
   pub direction: u32,
   pub ep: u32,
   pub transfer_flags: u32,
   pub transfer_buffer_length: u32,
   pub start_frame: u32,
   pub number_of_packets: u32,
   pub interval_or_err_count: u32,
   pub setup: [u8; 8],
}

impl UsbIpCmd {
   fn to_array(&self) -> [u8; 36] {
      let mut result = [0; 36];

      result[0..4].copy_from_slice(&self.direction.to_be_bytes());
      result[4..8].copy_from_slice(&self.ep.to_be_bytes());
      result[8..12].copy_from_slice(&self.transfer_flags.to_be_bytes());
      result[12..16].copy_from_slice(&self.transfer_buffer_length.to_be_bytes());
      result[16..20].copy_from_slice(&self.start_frame.to_be_bytes());
      result[20..24].copy_from_slice(&self.number_of_packets.to_be_bytes());
      result[24..28].copy_from_slice(&self.interval_or_err_count.to_be_bytes());
      result[28..36].copy_from_slice(&self.setup);

      result
   }

   fn from_slice(data: &[u8]) -> Self {
      Self {
         direction: u32::from_be_bytes(data[0..4].try_into().unwrap()),
         ep: u32::from_be_bytes(data[4..8].try_into().unwrap()),
         transfer_flags: u32::from_be_bytes(data[8..12].try_into().unwrap()),
         transfer_buffer_length: u32::from_be_bytes(data[12..16].try_into().unwrap()),
         start_frame: u32::from_be_bytes(data[16..20].try_into().unwrap()),
         number_of_packets: u32::from_be_bytes(data[20..24].try_into().unwrap()),
         interval_or_err_count: u32::from_be_bytes(data[24..28].try_into().unwrap()),
         setup: data[28..36].try_into().unwrap(),
      }
   }
}

// TODO: Implement Buffer flags
// TODO: Implement buffer flag integrity check

pub struct UsbIpResponse {
   pub header: UsbIpHeader,
   pub cmd: UsbIpResponseCmd,
   pub data: Vec<u8>,
}

pub enum UsbIpResponseCmd {
   Cmd(UsbIpCmd),
}

impl UsbIpResponse {
   pub fn to_vec(&self) -> Option<Vec<u8>> {
      let mut result = vec![];

      // Parse the header
      result.extend_from_slice(&self.header.to_array());

      // parse the command
      match self.cmd {
         UsbIpResponseCmd::Cmd(ref cmd) => {
            result.extend_from_slice(&cmd.to_array());
         }
      }

      // parse the data
      result.extend_from_slice(&self.data[..]);

      Some(result)
   }
}
