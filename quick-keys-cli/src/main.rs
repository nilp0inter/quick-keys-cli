extern crate hidapi;

use std::{thread,time};
use hidapi::HidApi;

fn pad_zeroes<const A:usize, const B:usize>(arr:[u8;A])->[u8;B]{
    assert!(B >= A);
    let mut b = [0;B];
    b[..A].copy_from_slice(&arr);
    b
}


fn mkcmd_subscribe_to_key_events() -> [u8; 32] {
    pad_zeroes([0x02, 0xb0, 0x04])
}


fn mkcmd_subscribe_to_battery() -> [u8; 32] {
    pad_zeroes([0x02, 0xb4, 0x10])
}

enum ScreenOrientation {
    Rotate0 = 1,
    Rotate90 = 2,
    Rotate180 = 3,
    Rotate270 = 4,
}

fn mkcmd_rotate_screen(rot: ScreenOrientation) -> [u8; 32] {
    pad_zeroes([0x02, 0xb1, rot as u8])
}

enum ScreenBrightness {
    Off = 0,
    Low = 1,
    Medium = 2,
    Full = 3,
}

fn mkcmd_set_screen_brightness(level: ScreenBrightness) -> [u8; 32] {
    pad_zeroes([0x02, 0xb1, 0x0a, 0x01, level as u8])
}

enum WheelSpeed {
    Slowest = 5,
    Slower = 4,
    Normal = 3,
    Faster = 2,
    Fastest = 1,
}

fn mkcmd_set_wheel_speed(speed: WheelSpeed) -> [u8; 32] {
    pad_zeroes([0x02, 0xb4, 0x04, 0x01, 0x01, speed as u8])
}

// TODO: check
fn mkcmd_set_sleep_timeout(minutes: u8) -> [u8; 32] {
    pad_zeroes([0x02, 0xb4, 0x08, 0x01, minutes])
}

fn mkcmd_set_wheel_color(r: u8, g: u8, b: u8) -> [u8; 32] {
    pad_zeroes([0x02, 0xb4, 0x01, 0x01, 0x00, 0x00, r, g, b])
}

fn to_array(a: &[u8]) -> [u8; 32] {
    a[..32].try_into().unwrap()
}

fn mkcmd_set_key_text(key: u8, text: &str) -> [u8; 32] {
    let mut body = [0u8; 32];
    body[..6].clone_from_slice(&[0x02, 0xb1, 0x00, key + 1, 0x00, (if text.len() <= 8 {text.len()*2} else {16}) as u8]);

    let mut payload = text.encode_utf16().flat_map(|c| c.to_le_bytes()).collect::<Vec<u8>>();
    payload.resize(16, 0);
    body[16..].clone_from_slice(&payload);
    body
}

fn mk_overlay_chunk(is_cont: bool, duration: u8, text: &str, has_more: bool) -> [u8; 32] {
    let mut body = [0u8; 32];
    body[..7].clone_from_slice(&[0x02, 0xb1, if is_cont { 0x06 } else { 0x05 }, duration, 0x00, (if text.len() <= 8 {text.len()*2} else {8}) as u8, has_more as u8]);

    let mut payload = text.encode_utf16().flat_map(|c| c.to_le_bytes()).collect::<Vec<u8>>();
    payload.resize(16, 0);
    body[16..].clone_from_slice(&payload);
    body
}

fn mkcmd_show_overlay_text(duration: u8, text: &str) -> Vec<[u8; 32]> {
    assert!(text.len() <= 32);
    let mut res = Vec::new();
    for (i, chunk, is_last_element) in text.chars().collect::<Vec<char>>().chunks(8).map(|c| c.iter().collect::<String>()).collect::<Vec<String>>().iter().enumerate().map(|(i, w)| (i, w, i == (text.len()/8)-1)) {
        res.push(mk_overlay_chunk(i != 0, duration, &chunk, is_last_element))
    }
    res
}

#[derive(Debug)]
enum WheelDirection {
    Right,
    Left,
}

#[derive(Debug)]
struct ButtonState {
    Button0: bool,
    Button1: bool,
    Button2: bool,
    Button3: bool,
    Button4: bool,
    Button5: bool,
    Button6: bool,
    Button7: bool,
    ButtonExtra: bool,
    ButtonWheel: bool,
}

#[derive(Debug)]
enum Event {
    Button { state: ButtonState },
    Wheel { direction: WheelDirection },
    Battery { percent: u8 },
    Unknown { },
}

fn process_input(data: &[u8; 16]) -> Event {
    if data[0] == 0x02 {
        if data[1] == 0xf0 {
            let wheelByte = data[7];
            if wheelByte & 0x01 > 0 {
                Event::Wheel { direction: WheelDirection::Right }
            } else if wheelByte & 0x02 > 0 {
                Event::Wheel { direction: WheelDirection::Left }
            } else {
                let keys1 = data[2];
                let keys2 = data[3];
                Event:: Button { state: ButtonState {
                    Button0: keys1 & (1 << 0) > 0,
                    Button1: keys1 & (1 << 1) > 0,
                    Button2: keys1 & (1 << 2) > 0,
                    Button3: keys1 & (1 << 3) > 0,
                    Button4: keys1 & (1 << 4) > 0,
                    Button5: keys1 & (1 << 5) > 0,
                    Button6: keys1 & (1 << 6) > 0,
                    Button7: keys1 & (1 << 7) > 0,
                    ButtonExtra: keys2 & (1 << 0) > 0,
                    ButtonWheel: keys2 & (1 << 1) > 0,
                }}
            }
        } else if data[0] == 0xf2 && data[2] == 0x01 {
            Event:: Battery { percent: data[3] }
        } else {
            Event:: Unknown { }
        }
    } else {
        Event:: Unknown { }
    }
}

fn main() {
    println!("Printing all available hid devices:");
    let maybeApi = hidapi::HidApi::new();

    match maybeApi {
        Ok(api) => {
            for device in api.device_list() {
                let vid = device.vendor_id();
                let pid = device.product_id();
                let usage = device.usage();
                let usage_page = device.usage_page();
                if vid == 0x28BD && pid == 0x5202 && usage == 1 && usage_page == 0xff0a {
                    println!("Connect!");
                } else {
                    continue;
                }
                let dev = device.open_device(&api).unwrap();

                dev.write(&mkcmd_subscribe_to_key_events()).unwrap();
                dev.write(&mkcmd_subscribe_to_battery()).unwrap();
                dev.write(&mkcmd_rotate_screen(ScreenOrientation::Rotate270)).unwrap();
                dev.write(&mkcmd_set_screen_brightness(ScreenBrightness::Low)).unwrap();
                dev.write(&mkcmd_set_wheel_speed(WheelSpeed::Normal)).unwrap();
                dev.write(&mkcmd_set_sleep_timeout(1)).unwrap();
                dev.write(&mkcmd_set_wheel_color(255, 0, 0)).unwrap();
                dev.write(&mkcmd_set_key_text(0, "red")).unwrap();
                dev.write(&mkcmd_set_key_text(1, "green")).unwrap();
                dev.write(&mkcmd_set_key_text(2, "blue")).unwrap();
                dev.write(&mkcmd_set_key_text(3, "yellow")).unwrap();
                dev.write(&mkcmd_set_key_text(4, "purple")).unwrap();
                dev.write(&mkcmd_set_key_text(5, "turquoise")).unwrap();
                dev.write(&mkcmd_set_key_text(6, "white")).unwrap();
                dev.write(&mkcmd_set_key_text(7, "off")).unwrap();

                thread::sleep(time::Duration::from_millis(1000));
                for chunk in &mkcmd_show_overlay_text(2, "This is an overlay!") {
                    dev.write(chunk).unwrap();
                }

                // Read data from device
                loop {
                    let mut buf = [0u8; 16];
                    dev.read(&mut buf[..]).unwrap();
                    let ev = process_input(&buf);
                    println!("Read: {:?}", &ev);
                    match ev {
                        Event::Wheel { direction: WheelDirection::Left } => dev.write(&mkcmd_set_wheel_color(255, 0, 0)).unwrap(),
                        Event::Wheel { direction: WheelDirection::Right } => dev.write(&mkcmd_set_wheel_color(0, 255, 0)).unwrap(),
                        Event::Button { state: ButtonState { ButtonWheel: true, .. } } => dev.write(&mkcmd_set_wheel_color(0, 0, 255)).unwrap(),
                        Event::Button { state: ButtonState { Button0: true, .. } } => dev.write(&mkcmd_set_wheel_color(255, 0, 0)).unwrap(),
                        Event::Button { state: ButtonState { Button1: true, .. } } => dev.write(&mkcmd_set_wheel_color(0, 255, 0)).unwrap(),
                        Event::Button { state: ButtonState { Button2: true, .. } } => dev.write(&mkcmd_set_wheel_color(0, 0, 255)).unwrap(),
                        Event::Button { state: ButtonState { Button3: true, .. } } => dev.write(&mkcmd_set_wheel_color(255, 255, 0)).unwrap(),
                        Event::Button { state: ButtonState { Button4: true, .. } } => dev.write(&mkcmd_set_wheel_color(255, 0, 255)).unwrap(),
                        Event::Button { state: ButtonState { Button5: true, .. } } => dev.write(&mkcmd_set_wheel_color(0, 255, 255)).unwrap(),
                        Event::Button { state: ButtonState { Button6: true, .. } } => dev.write(&mkcmd_set_wheel_color(255, 255, 255)).unwrap(),
                        Event::Button { state: ButtonState { Button7: true, .. } } => dev.write(&mkcmd_set_wheel_color(0, 0, 0)).unwrap(),
                        _ => 0,
                    };
                }
            }
        },
        Err(e) => {
            eprintln!("Error: {}", e);
        },
    }
}
