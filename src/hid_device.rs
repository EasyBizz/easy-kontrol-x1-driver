use hidapi::HidApi;

const VENDOR_ID: u16 = 0x17cc;
const PRODUCT_ID_MK1: u16 = 0x2305;
const PRODUCT_ID_MK2: u16 = 0x1220;

pub struct HidDevice {
    pub handle: hidapi::HidDevice,
    pub serial_number: String,
}

impl HidDevice {
    /// Try to find and open an NI X1 device using hidapi
    pub fn open() -> Result<Vec<HidDevice>, String> {
        let api = HidApi::new().map_err(|e| format!("Failed to create HID API: {}", e))?;
        let mut devices = Vec::new();

        // Scan for MK1 and MK2 devices
        for product_id in &[PRODUCT_ID_MK1, PRODUCT_ID_MK2] {
            match api.open(VENDOR_ID, *product_id) {
                Ok(device) => {
                    let serial = device
                        .get_serial_number_string()
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| "Unknown".to_string());
                    println!("Opened HID device: vendor=0x{:04x} product=0x{:04x}", 
                        VENDOR_ID, product_id);
                    devices.push(HidDevice {
                        handle: device,
                        serial_number: serial,
                    });
                }
                Err(_) => {
                    // Device not found, which is fine
                }
            }
        }

        if devices.is_empty() {
            return Err("No NI X1 devices found".to_string());
        }

        Ok(devices)
    }

    /// Write LED data to the device via HID
    pub fn write_leds(&self, data: &[u8; 32]) -> Result<(), String> {
        let mut buf = vec![0u8; 33];
        buf[0] = 0; // Report ID
        buf[1..].copy_from_slice(data);
        
        match self.handle.write(&buf) {
            Ok(n) => {
                if n != buf.len() {
                    eprintln!("Partial LED write: wrote {} bytes, expected {}", n, buf.len());
                }
                Ok(())
            }
            Err(e) => Err(format!("LED write error: {}", e)),
        }
    }

    /// Read button/knob data from the device via HID
    pub fn read_input(&self, buf: &mut [u8; 24], timeout_ms: i32) -> Result<usize, String> {
        match self.handle.read_timeout(buf, timeout_ms) {
            Ok(n) => {
                if n > 0 {
                    Ok(n)
                } else {
                    Err("No data".to_string())
                }
            }
            Err(e) => Err(format!("Read error: {}", e)),
        }
    }
}
