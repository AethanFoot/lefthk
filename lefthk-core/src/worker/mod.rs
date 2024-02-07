pub mod context;

use std::path::Path;

use crate::child::Children;
use crate::config::{command, Keybind};
use crate::errors::{self, Error, LeftError};
use crate::evdev;
use crate::evdev::EvDev;
use crate::ipc::Pipe;
use xdg::BaseDirectories;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Status {
    Reload,
    Kill,
    Continue,
}

pub struct Worker {
    keybinds: Vec<Keybind>,
    base_directory: BaseDirectories,

    pub evdev: EvDev,
    pub children: Children,
    pub status: Status,

    /// "Chord Context": Holds the relevant data for chording
    pub chord_ctx: context::Chord,
}

impl Worker {
    pub fn new(keybinds: Vec<Keybind>, base_directory: BaseDirectories) -> Self {
        Self {
            status: Status::Continue,
            keybinds,
            base_directory,
            evdev: EvDev::new(),
            children: Children::default(),
            chord_ctx: context::Chord::new(),
        }
    }

    pub async fn event_loop(mut self) -> Status {
        let mut pipe = self.get_pipe().await;

        while self.status == Status::Continue {
            self.evaluate_chord();

            tokio::select! {
                () = self.children.wait_readable() => {
                    self.children.reap();
                }
                () = self.evdev.wait_readable() => {
                    for device in &self.evdev.devices {
                        while device.has_event_pending() {
                            match device.next_event(evdev_rs::ReadFlag::NORMAL) {
                                Ok((_, event)) if event.value == 1 => println!("{:?}", event),
                                Err(_) => println!("Boo"),
                                _ => {},
                            }
                        }
                    }
                }
                Some(command) = pipe.get_next_command() => {
                    errors::log_on_error!(command.execute(&mut self));
                }
            };
        }

        self.status
    }

    async fn get_pipe(&self) -> Pipe {
        let pipe_name = Pipe::pipe_name();
        let pipe_file = errors::exit_on_error!(self.base_directory.place_runtime_file(pipe_name));
        errors::exit_on_error!(Pipe::new(pipe_file).await)
    }

    // fn handle_event(&mut self, xlib_event: &xlib::XEvent) {
    //     let error = match xlib_event.get_type() {
    //         xlib::KeyPress => self.handle_key_press(&xlib::XKeyEvent::from(xlib_event)),
    //         xlib::MappingNotify => {
    //             self.handle_mapping_notify(&mut xlib::XMappingEvent::from(xlib_event))
    //         }
    //         _ => return,
    //     };
    //     errors::log_on_error!(error);
    // }
    //
    // fn handle_key_press(&mut self, event: &xlib::XKeyEvent) -> Error {
    //     let key = self.xwrap.keycode_to_keysym(event.keycode);
    //     let mask = xkeysym_lookup::clean_mask(event.state);
    //     if let Some(keybind) = self.get_keybind((mask, key)) {
    //         if let Ok(command) = command::denormalize(&keybind.command) {
    //             return command.execute(self);
    //         }
    //     } else {
    //         return Err(LeftError::CommandNotFound);
    //     }
    //     Ok(())
    // }
    //
    // fn get_keybind(&self, mask_key_pair: (u32, u32)) -> Option<Keybind> {
    //     let keybinds = if let Some(keybinds) = &self.chord_ctx.keybinds {
    //         keybinds
    //     } else {
    //         &self.keybinds
    //     };
    //     keybinds
    //         .iter()
    //         .find(|keybind| {
    //             if let Some(key) = xkeysym_lookup::into_keysym(&keybind.key) {
    //                 let mask = xkeysym_lookup::into_modmask(&keybind.modifier);
    //                 return mask_key_pair == (mask, key);
    //             }
    //             false
    //         })
    //         .cloned()
    // }
    //
    // fn handle_mapping_notify(&self, event: &mut xlib::XMappingEvent) -> Error {
    //     if event.request == xlib::MappingModifier || event.request == xlib::MappingKeyboard {
    //         return self.xwrap.refresh_keyboard(event);
    //     }
    //     Ok(())
    // }
}
