extern crate sdl2;

use std::collections::HashMap;
use std::env;
use std::path::Path;
use sdl2::event::Event;
use sdl2::image::{LoadSurface, InitFlag};
use sdl2::keyboard::Keycode;
use sdl2::mouse::Cursor;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::surface::Surface;
struct KeyRepeatEvent {
    key_code: sdl2::keyboard::Keycode,
    
}
pub fn run(png: &Path) -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let _image_context = sdl2::image::init(InitFlag::PNG | InitFlag::JPG)?;
    let window = video_subsystem.window("rust-sdl2 demo: Cursor", 800, 600)
      .position_centered()
      .build()
      .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().software().build().map_err(|e| e.to_string())?;
    let mut keys_down = HashMap::<sdl2::keyboard::Keycode, ()>::new();
    sdl_context.event()?.register_custom_event::<KeyRepeatEvent>()?;
    let surface = Surface::from_file(png)
        .map_err(|err| format!("failed to load cursor image: {}", err))?;
    let cursor = Cursor::from_surface(surface, 0, 0)
        .map_err(|err| format!("failed to load cursor: {}", err))?;
    cursor.set();
    
    canvas.clear();
    canvas.present();

    canvas.set_draw_color(Color::RGBA(255, 255, 255, 255));

    let mut events = sdl_context.event_pump()?;

    'mainloop: loop {
        for event in events.wait_iter() {
            match event {
                Event::Quit{..} => break 'mainloop,
                Event::KeyDown {keycode: Option::Some(key_code), ..} =>{
                    if let None = keys_down.insert(key_code, ()) {
                        for (key,_)in keys_down.iter() {
                            eprintln!("Key is down {}\n", *key)
                        }
                        let timer = sdl_context.timer()?.add_timer(
                            16/*ms*/,
                            Box::new(|| {
                                let mut opt_evt_mgr = sdl_context.event();
                                if let Ok(evt_mgr) = opt_evt_mgr {
                                    evt_mgr.push_custom_event(KeyRepeatEvent{key_code:key_code});
                                    0
                                } else {
                                    1
                                }
                            }));
                    }
                },
                Event::KeyUp {keycode: Option::Some(key_code), ..} =>
                {
                    keys_down.remove(&key_code);
                },
                 Event::MouseButtonDown {x, y, ..} => {
                    canvas.fill_rect(Rect::new(x, y, 1, 1))?;
                    canvas.present();
                }
                _ => {}
            }
        }
    }

    Ok(())
}


fn main() -> Result<(), String> {
    let args: Vec<_> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: cargo run /path/to/image.(png|jpg)")
    } else {
        run(Path::new(&args[1]))?;
    }

    Ok(())
}
