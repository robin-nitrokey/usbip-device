use usb_device::{bus::UsbBusAllocator, prelude::*};
use usbd_serial::{SerialPort, USB_CLASS_CDC};
use usbip_device::UsbIpBus;

fn main() -> Result<(), Box<dyn std::error::Error>> {
   pretty_env_logger::init();

   log::info!("initializing allocator");

   let bus_allocator = UsbBusAllocator::new(UsbIpBus::new()?);
   let mut usb_serial = SerialPort::new(&bus_allocator);
   let mut usb_bus = UsbDeviceBuilder::new(&bus_allocator, UsbVidPid(0x16c0, 0x27dd))
      .manufacturer("Fake company")
      .product("Serial port")
      .serial_number("TEST")
      .device_class(USB_CLASS_CDC)
      .build();

   loop {
      std::thread::sleep(std::time::Duration::from_millis(5));
      usb_bus.poll(&mut [&mut usb_serial]);

      let mut buf = [0; 64];
      if let Ok(count) = usb_serial.read(&mut buf) {
         let text = String::from_utf8_lossy(&buf);
         log::info!("read {} bytes: {}", count, text);

         // TODO: To uppercase

         // Send back, poll usb until we are ready
         loop {
            usb_bus.poll(&mut [&mut usb_serial]);
            if let Ok(count) = usb_serial.write(&buf) {
               log::info!("sent back {} bytes", count);
               break;
            }
         }
      }
   }
}