use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Read;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use rusb::{Context, Device, HotplugBuilder, Registration, UsbContext};
use system_status_bar_macos::sync_infinite_event_loop;

use crate::conf::YamlConfig;
use crate::hid_device::HidDevice;
use crate::menu_bar::{MenuBar, MenuCommand, set_menu_sender};
use crate::usb_hotplug::HotPlugHandler;
use crate::utils::{get_serial_number, get_yaml_file};
use crate::x1_process::X1mk1;
use crate::x1_process_hid::X1mk1Hid;

mod x1_process;
mod x1_process_hid;
mod usb_hotplug;
mod utils;
mod conf;
mod x1_board;
mod hid_device;
mod menu_bar;

const USB_ID_VENDOR: u16 = 0x17cc;
const USB_ID_PRODUCT: u16 = 0x2305;

fn main() {
    let (sender_menu_bar, receiver_menu_bar) = mpsc::channel::<HashMap<String, bool>>();
    let (cmd_tx, cmd_rx) = mpsc::channel::<MenuCommand>();
    set_menu_sender(cmd_tx);

    let run_flag = Arc::new(AtomicBool::new(true));
    let last_devices: Arc<Mutex<HashMap<String, bool>>> = Arc::new(Mutex::new(HashMap::new()));
    let last_devices_cmd = Arc::clone(&last_devices);
    let sender_menu_bar_cmd = sender_menu_bar.clone();
    let run_flag_cmd = Arc::clone(&run_flag);
    let run_flag_menu = Arc::clone(&run_flag);

    thread::spawn(move || {
        x1(sender_menu_bar, run_flag_cmd).unwrap();
    });

    let icon_path = resolve_menu_icon();
    let menu_bar = RefCell::new(MenuBar::new(icon_path));

    thread::spawn(move || {
        let run_flag = run_flag_menu;
        while let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                MenuCommand::Start => run_flag.store(true, Ordering::Relaxed),
                MenuCommand::Stop => run_flag.store(false, Ordering::Relaxed),
                MenuCommand::Quit => std::process::exit(0),
            }
            let snapshot = last_devices_cmd.lock().unwrap().clone();
            let _ = sender_menu_bar_cmd.send(snapshot);
        }
    });

    sync_infinite_event_loop(receiver_menu_bar, move |x1| {
        *last_devices.lock().unwrap() = x1.clone();
        let running = run_flag.load(Ordering::Relaxed);
        menu_bar.borrow_mut().update_menu(&x1, running);
    });
}

fn resolve_menu_icon() -> Option<std::path::PathBuf> {
    if let Ok(p) = std::env::var("X1_MENU_ICON") {
        let path = std::path::PathBuf::from(p);
        if path.exists() {
            return Some(path);
        }
    }
    let local = std::path::PathBuf::from("logo/18x18.png");
    if local.exists() {
        return Some(local);
    }
    let mut resources_dir = std::env::current_exe().ok()?;
    resources_dir.pop();
    resources_dir.pop();
    resources_dir.push("Resources");
    resources_dir.push("logo/18x18.png");
    if resources_dir.exists() {
        Some(resources_dir)
    } else {
        None
    }
}

fn x1(sender_menu_bar: Sender<HashMap<String, bool>>, run_flag: Arc<AtomicBool>) -> rusb::Result<()> {
    let mut file = get_yaml_file();
    let mut yaml_content = String::new();
    file.read_to_string(&mut yaml_content).expect("Failed to read YAML file");
    let yaml_config: YamlConfig = serde_yaml::from_str(&yaml_content).expect("Failed to parse YAML");
    let yaml_config = Arc::new(yaml_config);

    let force_libusb = std::env::var("FORCE_LIBUSB").ok().as_deref() == Some("1");
    // Try HID API first (works better on macOS 26.1+) unless forced to libusb
    if !force_libusb {
        println!("Attempting to use HID API...");
        if let Ok(hid_devices) = HidDevice::open() {
            println!("✓ Successfully opened {} device(s) via HID", hid_devices.len());
            for hid_dev in hid_devices {
                let serial = hid_dev.serial_number.clone();
                let devices_map = Arc::new(Mutex::new(HashMap::new()));
                devices_map.lock().unwrap().insert(serial.clone(), true);
                sender_menu_bar.send(devices_map.lock().unwrap().clone()).unwrap();
                
                let sender_mb = sender_menu_bar.clone();
                let devices_thread = Arc::clone(&devices_map);
                let serial_clone = serial.clone();
                let yaml_config_clone = Arc::clone(&yaml_config);
                let run_flag = Arc::clone(&run_flag);
                
                thread::spawn(move || {
                    let mut x1mk1 = X1mk1Hid::new(
                        hid_dev.handle,
                        serial_clone.clone(),
                        (*yaml_config_clone).clone(),
                        run_flag,
                    );
                    loop {
                        match x1mk1.read() {
                            Ok(_) => {}
                            Err(_e) => {
                                devices_thread.lock().unwrap().insert(serial_clone.clone(), false);
                                sender_mb.send(devices_thread.lock().unwrap().clone()).unwrap();
                                break;
                            }
                        }
                    }
                });
            }
            // Keep running the event loop
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
    } else {
        println!("FORCE_LIBUSB=1 set; skipping HID API.");
    }

    // Fall back to libusb if HID doesn't find devices
    println!("HID API did not find devices, falling back to libusb...");
    if rusb::has_hotplug() {
        println!("libusb hotplug supported");
        let context = Context::new()?;
        let (tx, rx) = mpsc::channel::<Device<Context>>();
        let _tx_enumerate = tx.clone();

        let mut reg: Option<Registration<Context>> = Some(
            HotplugBuilder::new()
                .enumerate(true)
                .vendor_id(USB_ID_VENDOR)
                .product_id(USB_ID_PRODUCT)
                .register(&context, Box::new(HotPlugHandler { sender: tx }))?,
        );

        // Manually enumerate existing devices in case hotplug callbacks don't fire for already-connected devices
        if let Ok(devices_list) = context.devices() {
            println!("Scanning existing devices...");
            for device in devices_list.iter() {
                if let Ok(descriptor) = device.device_descriptor() {
                    if descriptor.vendor_id() == USB_ID_VENDOR {
                        let pid = descriptor.product_id();
                        if pid != 0x1220 && pid != 0x2305 {
                            continue;
                        }
                        println!("Found device: vendor=0x{:04x} product=0x{:04x}", descriptor.vendor_id(), pid);
                        // attempt to open and spawn handler immediately if possible
                        let serial = get_serial_number(&device);
                        let serial_for_map = serial.clone();
                        let serial_for_x1 = serial.clone();
                        // avoid duplicates
                        let devices_map = Arc::new(Mutex::new(HashMap::new()));
                        if !devices_map.lock().unwrap().contains_key(&serial_for_map) {
                            match device.open() {
                                Ok(handle) => {
                                    println!("Opening device (pid=0x{:04x})", descriptor.product_id());
                                    devices_map.lock().unwrap().insert(serial_for_map.clone(), true);
                                    sender_menu_bar.send(devices_map.lock().unwrap().clone()).unwrap();
                                    let devices_thread = Arc::clone(&devices_map);
                                    let sender_mb = sender_menu_bar.clone();
                                    let device_clone = device.clone();
                                    let yaml_config_clone = Arc::clone(&yaml_config);
                                    let run_flag = Arc::clone(&run_flag);
                                    thread::spawn(move || {
                                        let mut x1mk1 = X1mk1::new(
                                            device_clone,
                                            handle,
                                            serial_for_x1,
                                            (*yaml_config_clone).clone(),
                                            run_flag,
                                        );
                                        loop {
                                            match x1mk1.read() {
                                                Ok(_) => {}
                                                Err(e) => {
                                                    eprintln!("Error reading from device: {:?}", e);
                                                    devices_thread.lock().unwrap().insert(serial_for_map.clone(), false);
                                                    sender_mb.send(devices_thread.lock().unwrap().clone()).unwrap();
                                                    break;
                                                }
                                            }
                                        }
                                    });
                                }
                                Err(e) => println!("Could not open device (maybe in use by system): {:?}", e),
                            }
                        }
                    }
                }
            }
        }

        let devices = Arc::new(Mutex::new(HashMap::new()));
        thread::spawn({
            let devices = Arc::clone(&devices);
            let sender_menu_bar = sender_menu_bar.clone();
            move || loop {
                let device = rx.recv().unwrap();
            println!("Device received on channel");
                let handle = device.open().unwrap();
                let serial_number = get_serial_number(&device);
                let serial_number_clone = serial_number.clone();
                let yaml_config: YamlConfig = serde_yaml::from_str(&yaml_content).expect("Failed to parse YAML");
                devices.lock().unwrap().insert(serial_number_clone.clone(), true);
                sender_menu_bar.send(devices.lock().unwrap().clone()).unwrap();
                let run_flag = Arc::clone(&run_flag);
                thread::spawn({
                    let devices = Arc::clone(&devices);
                    let sender_menu_bar = sender_menu_bar.clone();
                    move || {
                        let mut x1mk1 = X1mk1::new(
                            device,
                            handle,
                            serial_number,
                            yaml_config.clone(),
                            run_flag,
                        );
                        loop {
                            match x1mk1.read() {
                                Ok(x) => x,
                                Err(e) => {
                                    eprintln!("Error reading from device: {:?}", e);
                                    devices.lock().unwrap().insert(serial_number_clone, false);
                                    sender_menu_bar.send(devices.lock().unwrap().clone()).unwrap();
                                    break;
                                }
                            };
                        }
                    }
                });
            }
        });

        loop {
            match context.handle_events(None) {
                Ok(x) => x,
                Err(_) => {
                    if let Some(reg) = reg.take() {
                        context.unregister_callback(reg);
                    }
                }
            };
        }
    } else {
        eprintln!("libusb compiled without hotplug support, falling back to polling");
        let context = Context::new()?;
        let devices = Arc::new(Mutex::new(HashMap::new()));
        loop {
            match context.devices() {
                Ok(list) => {
                    for device in list.iter() {
                        if let Ok(desc) = device.device_descriptor() {
                            if desc.vendor_id() == USB_ID_VENDOR && desc.product_id() == USB_ID_PRODUCT {
                                let serial_number = get_serial_number(&device);
                                let serial_clone = serial_number.clone();
                                let already = devices.lock().unwrap().contains_key(&serial_clone);
                                if !already {
                                    match device.open() {
                                        Ok(handle) => {
                                                println!("Polling: spawning handler");
                                            let yaml_config: YamlConfig = serde_yaml::from_str(&yaml_content).expect("Failed to parse YAML");
                                            devices.lock().unwrap().insert(serial_clone.clone(), true);
                                            sender_menu_bar.send(devices.lock().unwrap().clone()).unwrap();
                                            let devices_thread = Arc::clone(&devices);
                                            let sender_mb = sender_menu_bar.clone();
                                            let device_clone = device.clone();
                                            let run_flag = Arc::clone(&run_flag);
                                            thread::spawn(move || {
                                                let mut x1mk1 = X1mk1::new(
                                                    device_clone,
                                                    handle,
                                                    serial_number,
                                                    yaml_config.clone(),
                                                    run_flag,
                                                );
                                                loop {
                                                    match x1mk1.read() {
                                                        Ok(_) => {}
                                                        Err(e) => {
                                                            eprintln!("Error reading from device: {:?}", e);
                                                            devices_thread.lock().unwrap().insert(serial_clone.clone(), false);
                                                            sender_mb.send(devices_thread.lock().unwrap().clone()).unwrap();
                                                            break;
                                                        }
                                                    }
                                                }
                                            });
                                        }
                                        Err(e) => eprintln!("Failed to open device: {:?}", e),
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => eprintln!("Error enumerating devices: {:?}", e),
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
    Ok(())
}
