use serde::{Deserialize, Serialize};

#[repr(C)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OpHeader {
   pub version: u16,
   pub command: u16,
   pub status: u32,
}

pub enum OpRequest {
   ListDevices(OpHeader),
   ConnectDevice(OpHeader, String),
}

impl OpRequest {
   pub fn from_slice(data: &[u8]) -> Option<Self> {
      // Parse header
      if data.len() < 8 {
         log::warn!("received too short packet of length {}", data.len());
         return None;
      }
      let header: OpHeader = match ssmarshal::deserialize(&data[..8]) {
         Ok(header) => header.0,
         Err(_) => {
            log::warn!("failed to deserialize header");
            return None;
         }
      };

      // Check status
      if header.status != 0 {
         log::warn!("received request with error status code {}", header.status);
         return None;
      }

      log::debug!("request version is {}", header.version);

      // Dispatch on command
      match header.command {
         0x8005 => {
            log::info!("received request to list devices");
            Some(Self::ListDevices(header))
         }
         0x8003 => {
            if !data[8..].len() == 32 {
               log::warn!("packet has length of {}, expected 32", data[8..].len());
            }

            let bus_id = match std::str::from_utf8(&data) {
               Ok(data) => data.trim_matches(char::from(0)),
               _ => return None,
            };

            log::info!("connect request for bus id {}", bus_id);

            Some(Self::ConnectDevice(header, bus_id.to_string()))
         }
         _ => {
            log::warn!("received request with unknown command {}", header.command);
            None
         }
      }
   }
}

#[derive(Debug, Clone)]
pub struct OpResponse {
   pub version: u16,
   pub path: String,
   pub bus_id: String,
   pub descriptor: OpDeviceDescriptor,
   pub cmd: OpResponseCommand,
}

#[derive(Debug, Clone)]
pub enum OpResponseCommand {
   ListDevices(OpInterfaceDescriptor),
   ConnectDevice,
}

impl OpResponse {
   pub fn to_vec(self) -> Option<Vec<u8>> {
      let mut result = vec![];

      // Build and serialize the header
      let reply: u16 = match self.cmd {
         OpResponseCommand::ListDevices(_) => 0x0005,
         OpResponseCommand::ConnectDevice => 0x0003,
      };

      let header = OpHeader {
         version: self.version,
         command: reply,
         status: 0,
      };

      let mut header_buf = [0; 8];
      ssmarshal::serialize(&mut header_buf, &header).unwrap();
      result.extend_from_slice(&header_buf);

      // Serialize path
      if self.path.as_bytes().len() > 256 {
         log::warn!("path is longer than 256 bytes");
         return None;
      }

      let mut path_buf = [0; 256];
      path_buf.copy_from_slice(self.path.as_bytes());
      result.extend_from_slice(&path_buf);

      // Serialize bus_id
      if self.bus_id.as_bytes().len() > 32 {
         log::warn!("bus_id is longr than 32 bytes");
         return None;
      }

      let mut bus_id_buf = [0; 32];
      bus_id_buf.copy_from_slice(self.bus_id.as_bytes());
      result.extend_from_slice(&bus_id_buf);

      // Serialize the Op Desciptor
      let mut descriptor_buf = [0; 24];
      ssmarshal::serialize(&mut descriptor_buf, &self.descriptor).unwrap();
      result.extend_from_slice(&descriptor_buf);

      // If exists, serialize the interface descriptor
      if let OpResponseCommand::ListDevices(interface) = self.cmd {
         let mut interface_buf = [0; 4];
         ssmarshal::serialize(&mut interface_buf, &interface).unwrap();
         result.extend_from_slice(&interface_buf);
      }

      Some(result)
   }
}

#[repr(C)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OpDeviceDescriptor {
   pub busnum: u32,
   pub devnum: u32,
   pub speed: u32,
   pub vendor: u16,
   pub product: u16,
   pub bcd_device: u16,
   pub device_class: u8,
   pub device_subclass: u8,
   pub device_protocol: u8,
   pub configuration_value: u8,
   pub num_configurations: u8,
   pub num_interfaces: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OpInterfaceDescriptor {
   pub interface_class: u8,
   pub interface_subclass: u8,
   pub interface_protocol: u8,
   pub padding: u8,
}