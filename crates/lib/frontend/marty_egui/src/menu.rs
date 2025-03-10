/*
    MartyPC
    https://github.com/dbalsom/martypc

    Copyright 2022-2025 Daniel Balsom

    Permission is hereby granted, free of charge, to any person obtaining a
    copy of this software and associated documentation files (the “Software”),
    to deal in the Software without restriction, including without limitation
    the rights to use, copy, modify, merge, publish, distribute, sublicense,
    and/or sell copies of the Software, and to permit persons to whom the
    Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in
    all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
    FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.

    --------------------------------------------------------------------------

    egui::menu.rs

    Implement the main emulator menu bar.

*/
use crate::{state::GuiState, GuiBoolean, GuiEnum, GuiEvent, GuiFloat, GuiVariable, GuiVariableContext, GuiWindow};
use std::path::{Path, PathBuf};

use marty_frontend_common::display_manager::DtHandle;

//use egui_file_dialog::FileDialog;
use marty_core::{device_traits::videocard::VideoType, machine::MachineState};

#[cfg(feature = "scaler_ui")]
use marty_frontend_common::display_manager::DisplayTargetType;
#[cfg(feature = "scaler_ui")]
use strum::IntoEnumIterator;

#[cfg(feature = "use_serialport")]
use marty_core::devices::serial::SerialPortDescriptor;

use crate::modal::ModalContext;

use crate::{
    file_dialogs::FileDialogFilter,
    widgets::big_icon::{BigIcon, IconType},
};
use egui::RichText;
use fluxfox::ImageFormatParser;
use marty_core::cpu_common::Register16;
use marty_frontend_common::thread_events::{FileOpenContext, FileSaveContext, FileSelectionContext};

impl GuiState {
    pub fn show_menu(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("Emulator", |ui| {
                ui.set_min_width(120.0);

                if !self.modal.is_open() {
                    if ui.button("⏱ Performance...").clicked() {
                        *self.window_flag(GuiWindow::PerfViewer) = true;
                        ui.close_menu();
                    }

                    if ui.button("❓ About...").clicked() {
                        *self.window_flag(GuiWindow::About) = true;
                        ui.close_menu();
                    }
                    ui.separator();
                }

                if ui.button("⎆ Quit").clicked() {
                    self.event_queue.send(GuiEvent::Exit);
                    ui.close_menu();
                }
            });

            // Only show the Emulator menu if a modal dialog is open.
            if self.modal.is_open() {
                return;
            }

            ui.menu_button("Machine", |ui| {
                ui.menu_button("Emulation Speed", |ui| {
                    ui.horizontal(|ui| {
                        let mut speed = self.option_floats.get_mut(&GuiFloat::EmulationSpeed).unwrap();

                        ui.label("Factor:");
                        ui.add(
                            egui::Slider::new(speed, 0.1..=2.0)
                                .show_value(true)
                                .min_decimals(2)
                                .max_decimals(2)
                                .suffix("x"),
                        );

                        self.event_queue.send(GuiEvent::VariableChanged(
                            GuiVariableContext::Global,
                            GuiVariable::Float(GuiFloat::EmulationSpeed, *speed),
                        ));
                    });
                });

                ui.menu_button("Input/Output", |ui| {
                    #[cfg(feature = "use_serialport")]
                    {
                        // Create a vector of ports that are currently bridged. We will use this to disable
                        // those ports from selection in the menu.
                        let bridged_ports = self
                            .serial_ports
                            .iter()
                            .filter_map(|port| port.brige_port_id)
                            .collect::<Vec<_>>();

                        for SerialPortDescriptor {
                            id: guest_port_id,
                            name: guest_port_name,
                            ..
                        } in self.serial_ports.clone().iter()
                        {
                            ui.menu_button(format!("Passthrough {}", guest_port_name), |ui| {
                                let mut selected = false;

                                for (host_port_id, host_port) in self.host_serial_ports.iter().enumerate() {
                                    if let Some(enum_mut) = self.get_option_enum(
                                        GuiEnum::SerialPortBridge(Default::default()),
                                        Some(GuiVariableContext::SerialPort(*guest_port_id)),
                                    ) {
                                        selected = *enum_mut == GuiEnum::SerialPortBridge(host_port_id);
                                    }

                                    let enabled = !bridged_ports.contains(&host_port_id);

                                    if ui
                                        .add_enabled(
                                            enabled,
                                            egui::RadioButton::new(selected, host_port.port_name.clone()),
                                        )
                                        .clicked()
                                    {
                                        self.event_queue.send(GuiEvent::BridgeSerialPort(
                                            *guest_port_id,
                                            host_port.port_name.clone(),
                                            host_port_id,
                                        ));
                                        ui.close_menu();
                                    }
                                }
                            });
                        }
                    }
                });

                ui.separator();

                let (is_on, is_paused) = match self.machine_state {
                    MachineState::On => (true, false),
                    MachineState::Paused => (true, true),
                    MachineState::Off => (false, false),
                    _ => (false, false),
                };

                ui.add_enabled_ui(!is_on, |ui| {
                    if ui.button("⚡ Power on").clicked() {
                        self.event_queue.send(GuiEvent::MachineStateChange(MachineState::On));
                        ui.close_menu();
                    }
                });

                if ui
                    .checkbox(&mut self.get_option_mut(GuiBoolean::TurboButton), "Turbo Button")
                    .clicked()
                {
                    let new_opt = self.get_option(GuiBoolean::TurboButton).unwrap();

                    self.event_queue.send(GuiEvent::VariableChanged(
                        GuiVariableContext::Global,
                        GuiVariable::Bool(GuiBoolean::TurboButton, new_opt),
                    ));
                    ui.close_menu();
                }

                ui.add_enabled_ui(is_on && !is_paused, |ui| {
                    if ui.button("⏸ Pause").clicked() {
                        self.event_queue
                            .send(GuiEvent::MachineStateChange(MachineState::Paused));
                        ui.close_menu();
                    }
                });

                ui.add_enabled_ui(is_on && is_paused, |ui| {
                    if ui.button("▶ Resume").clicked() {
                        self.event_queue
                            .send(GuiEvent::MachineStateChange(MachineState::Resuming));
                        ui.close_menu();
                    }
                });

                ui.add_enabled_ui(is_on, |ui| {
                    if ui.button("⟲ Reboot").clicked() {
                        self.event_queue
                            .send(GuiEvent::MachineStateChange(MachineState::Rebooting));
                        ui.close_menu();
                    }
                });

                ui.add_enabled_ui(is_on, |ui| {
                    if ui.button("⟲ CTRL-ALT-DEL").clicked() {
                        self.event_queue.send(GuiEvent::CtrlAltDel);
                        ui.close_menu();
                    }
                });

                ui.add_enabled_ui(is_on, |ui| {
                    if ui.button("🔌 Power off").clicked() {
                        self.event_queue.send(GuiEvent::MachineStateChange(MachineState::Off));
                        ui.close_menu();
                    }
                });
            });

            let _media_response = ui.menu_button("Media", |ui| {
                //ui.set_min_size(egui::vec2(240.0, 0.0));
                //ui.style_mut().spacing.item_spacing = egui::Vec2{ x: 6.0, y:6.0 };
                ui.set_width_range(egui::Rangef { min: 100.0, max: 240.0 });

                // Display option to rescan media folders if native.
                // We can't rescan anything in the browser - what we've got is what we've got.
                #[cfg(not(target_arch = "wasm32"))]
                if ui.button("⟲ Rescan Media Folders").clicked() {
                    self.event_queue.send(GuiEvent::RescanMediaFolders);
                }

                self.workspace_window_open_button(ui, GuiWindow::FloppyViewer, true, true);
                for i in 0..self.floppy_drives.len() {
                    self.draw_floppy_menu(ui, i);
                }

                for i in 0..self.hdds.len() {
                    self.draw_hdd_menu(ui, i);
                }

                for i in 0..self.carts.len() {
                    self.draw_cart_menu(ui, i);
                }

                #[cfg(not(target_arch = "wasm32"))]
                {
                    if ui.button("🖹 Create new VHD...").clicked() {
                        *self.window_flag(GuiWindow::VHDCreator) = true;
                        ui.close_menu();
                    };
                }
            });

            ui.menu_button("Sound", |ui| {
                ui.set_min_width(240.0);
                if !self.sound_sources.is_empty() {
                    self.draw_sound_menu(ui);
                }
                else {
                    ui.label(RichText::new("No sound sources available.").italics());
                }
            });

            ui.menu_button("Display", |ui| {
                ui.set_min_size(egui::vec2(240.0, 0.0));

                // If there is only one display, emit the display menu directly.
                // Otherwise, emit named menus for each display.
                if self.display_info.len() == 1 {
                    self.draw_display_menu(ui, DtHandle::default());
                }
                else if self.display_info.len() > 1 {
                    // Use index here to avoid borrowing issues.
                    for i in 0..self.display_info.len() {
                        ui.menu_button(format!("Display {}: {}", i, &self.display_info[i].name), |ui| {
                            self.draw_display_menu(ui, self.display_info[i].handle);
                        });
                    }
                }
            });

            ui.menu_button("Debug", |ui| {
                ui.menu_button("CPU", |ui| {
                    self.workspace_window_open_button(ui, GuiWindow::CpuControl, true, true);
                    self.workspace_window_open_button(ui, GuiWindow::CpuStateViewer, true, true);

                    ui.menu_button("CPU Debug Options", |ui| {
                        if ui
                            .checkbox(
                                &mut self.get_option_mut(GuiBoolean::CpuEnableWaitStates),
                                "Enable Wait States",
                            )
                            .clicked()
                        {
                            let new_opt = self.get_option(GuiBoolean::CpuEnableWaitStates).unwrap();

                            self.event_queue.send(GuiEvent::VariableChanged(
                                GuiVariableContext::Global,
                                GuiVariable::Bool(GuiBoolean::CpuEnableWaitStates, new_opt),
                            ));
                        }
                        if ui
                            .checkbox(
                                &mut self.get_option_mut(GuiBoolean::CpuInstructionHistory),
                                "Instruction History",
                            )
                            .clicked()
                        {
                            let new_opt = self.get_option(GuiBoolean::CpuInstructionHistory).unwrap();

                            self.event_queue.send(GuiEvent::VariableChanged(
                                GuiVariableContext::Global,
                                GuiVariable::Bool(GuiBoolean::CpuInstructionHistory, new_opt),
                            ));
                            ui.close_menu();
                        }
                        if ui
                            .checkbox(
                                &mut self.get_option_mut(GuiBoolean::CpuTraceLoggingEnabled),
                                "Trace Logging Enabled",
                            )
                            .clicked()
                        {
                            let new_opt = self.get_option(GuiBoolean::CpuTraceLoggingEnabled).unwrap();

                            self.event_queue.send(GuiEvent::VariableChanged(
                                GuiVariableContext::Global,
                                GuiVariable::Bool(GuiBoolean::CpuTraceLoggingEnabled, new_opt),
                            ));
                            ui.close_menu();
                        }
                        #[cfg(feature = "devtools")]
                        if ui.button("Delays...").clicked() {
                            *self.window_flag(GuiWindow::DelayAdjust) = true;
                            ui.close_menu();
                        }

                        if ui.button("Trigger NMI").clicked() {
                            self.event_queue.send(GuiEvent::SetNMI(true));
                            ui.close_menu();
                        }

                        if ui.button("Clear NMI").clicked() {
                            self.event_queue.send(GuiEvent::SetNMI(false));
                            ui.close_menu();
                        }
                    });

                    self.workspace_window_open_button(ui, GuiWindow::InstructionHistoryViewer, true, true);
                    self.workspace_window_open_button(ui, GuiWindow::CycleTraceViewer, true, true);
                    self.workspace_window_open_button(ui, GuiWindow::CallStack, true, true);
                    self.workspace_window_open_button(ui, GuiWindow::DisassemblyViewer, true, true);

                    // Don't show disassembly listing recording options on web.
                    // There's no place for the recording to go...
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        ui.menu_button("Disassembly Listing", |ui| {
                            if ui.button("⏺ Start Recording").clicked() {
                                self.event_queue.send(GuiEvent::StartRecordingDisassembly);
                                ui.close_menu();
                            }
                            if ui.button("⏹ Stop Recording and Save").clicked() {
                                self.event_queue.send(GuiEvent::StopRecordingDisassembly);
                                ui.close_menu();
                            }
                        });
                    }
                });

                ui.menu_button("Memory", |ui| {
                    self.workspace_window_open_button(ui, GuiWindow::MemoryViewer, true, true);
                    self.workspace_window_open_button(ui, GuiWindow::DataVisualizer, true, true);
                    self.workspace_window_open_button(ui, GuiWindow::IvtViewer, true, true);

                    ui.menu_button("Dump Memory", |ui| {
                        if ui.button("Video Memory").clicked() {
                            self.event_queue.send(GuiEvent::DumpVRAM);
                            ui.close_menu();
                        }
                        if ui.button("Code Segment (CS)").clicked() {
                            self.event_queue.send(GuiEvent::DumpSegment(Register16::CS));
                            ui.close_menu();
                        }
                        if ui.button("Data Segment (DS)").clicked() {
                            self.event_queue.send(GuiEvent::DumpSegment(Register16::DS));
                            ui.close_menu();
                        }
                        if ui.button("Extra Segment (ES)").clicked() {
                            self.event_queue.send(GuiEvent::DumpSegment(Register16::ES));
                            ui.close_menu();
                        }
                        if ui.button("Stack Segment (SS)").clicked() {
                            self.event_queue.send(GuiEvent::DumpSegment(Register16::SS));
                            ui.close_menu();
                        }
                        if ui.button("All Memory").clicked() {
                            self.event_queue.send(GuiEvent::DumpAllMem);
                            ui.close_menu();
                        }
                    });
                });

                ui.menu_button("Devices", |ui| {
                    #[cfg(feature = "devtools")]
                    if ui.button("Device control...").clicked() {
                        *self.window_flag(GuiWindow::DeviceControl) = true;
                        ui.close_menu();
                    }
                    self.workspace_window_open_button(ui, GuiWindow::IoStatsViewer, true, true);
                    self.workspace_window_open_button(ui, GuiWindow::PicViewer, true, true);
                    self.workspace_window_open_button(ui, GuiWindow::PitViewer, true, true);
                    self.workspace_window_open_button(ui, GuiWindow::PpiViewer, true, true);
                    self.workspace_window_open_button(ui, GuiWindow::DmaViewer, true, true);
                    self.workspace_window_open_button(ui, GuiWindow::SerialViewer, true, true);
                    self.workspace_window_open_button(ui, GuiWindow::FdcViewer, true, true);
                    self.workspace_window_open_button(ui, GuiWindow::VideoCardViewer, true, true);

                    /*
                    if ui
                        .checkbox(
                            &mut self.get_option_mut(GuiBoolean::ShowBackBuffer),
                            "Debug back buffer",
                        )
                        .clicked()
                    {
                        let new_opt = self.get_option(GuiBoolean::ShowBackBuffer).unwrap();

                        self.event_queue.send(GuiEvent::VariableChanged(
                            GuiVariableContext::Global,
                            GuiVariable::Bool(GuiBoolean::ShowBackBuffer, new_opt),
                        ));
                        ui.close_menu();
                    }
                     */
                });

                if ui
                    .checkbox(&mut self.get_option_mut(GuiBoolean::ShowBackBuffer), "Show Back Buffer")
                    .clicked()
                {
                    let new_opt = self.get_option(GuiBoolean::ShowBackBuffer).unwrap();

                    self.event_queue.send(GuiEvent::VariableChanged(
                        GuiVariableContext::Global,
                        GuiVariable::Bool(GuiBoolean::ShowBackBuffer, new_opt),
                    ));
                }

                if ui
                    .checkbox(
                        &mut self.get_option_mut(GuiBoolean::ShowRasterPosition),
                        "Show Raster Position",
                    )
                    .clicked()
                {
                    let new_opt = self.get_option(GuiBoolean::ShowRasterPosition).unwrap();

                    self.event_queue.send(GuiEvent::VariableChanged(
                        GuiVariableContext::Global,
                        GuiVariable::Bool(GuiBoolean::ShowRasterPosition, new_opt),
                    ));
                }

                if ui.button("Flush Trace Logs").clicked() {
                    self.event_queue.send(GuiEvent::FlushLogs);
                    ui.close_menu();
                }
            });

            // Draw drive indicators, etc.
            self.draw_status_widgets(ui);
        });
    }

    pub fn draw_floppy_menu(&mut self, ui: &mut egui::Ui, drive_idx: usize) {
        let floppy_name = match drive_idx {
            0 => format!("💾 Floppy Drive 0 - {} (A:)", self.floppy_drives[drive_idx].drive_type),
            1 => format!("💾 Floppy Drive 1 - {} (B:)", self.floppy_drives[drive_idx].drive_type),
            _ => format!(
                "💾 Floppy Drive {} - {}",
                drive_idx, self.floppy_drives[drive_idx].drive_type
            ),
        };

        let _menu_response = ui
            .menu_button(floppy_name, |ui| {
                self.event_queue.send(GuiEvent::QueryCompatibleFloppyFormats(drive_idx));

                ui.menu_button("🗁 Quick Access Image/Zip file", |ui| {
                    self.floppy_tree_menu.draw(ui, drive_idx, true, &mut |image_idx| {
                        //log::debug!("Clicked closure called with image_idx {}", image_idx);
                        self.event_queue.send(GuiEvent::LoadQuickFloppy(drive_idx, image_idx));
                    });
                });

                if ui.button("🗁 Browse for Image...").clicked() {
                    #[cfg(target_arch = "wasm32")]
                    {
                        self.event_queue.send(GuiEvent::RequestLoadFloppyDialog(drive_idx));
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let fc = FileOpenContext::FloppyDiskImage {
                            drive_select: drive_idx,
                            fsc: FileSelectionContext::Uninitialized,
                        };

                        let mut filter_vec = Vec::new();
                        let exts = fluxfox::supported_extensions();
                        filter_vec.push(FileDialogFilter::new("Floppy Disk Images", exts));
                        filter_vec.push(FileDialogFilter::new("Zip Files", vec!["zip"]));
                        filter_vec.push(FileDialogFilter::new("All Files", vec!["*"]));

                        self.open_file_dialog(fc, "Select Floppy Disk Image", filter_vec);

                        self.modal.open(ModalContext::Notice(
                            "A native File Open dialog is open.\nPlease make a selection or cancel to continue."
                                .to_string(),
                        ));
                    }
                    ui.close_menu();
                };

                #[cfg(not(target_arch = "wasm32"))]
                if !self.autofloppy_paths.is_empty() {
                    ui.menu_button("🗐 Create from Directory", |ui| {
                        for path in self.autofloppy_paths.iter() {
                            if ui.button(format!("📁 {}", path.name.to_string_lossy())).clicked() {
                                self.event_queue
                                    .send(GuiEvent::LoadAutoFloppy(drive_idx, path.full_path.clone()));
                                ui.close_menu();
                            }
                        }
                    });
                }

                ui.menu_button("🗋 Create New", |ui| {
                    for format in self.floppy_drives[drive_idx].drive_type.get_compatible_formats() {
                        let format_options = vec![("(Blank)", false), ("(Formatted)", true)];
                        for fo in format_options {
                            if ui.button(format!("💾{} {}", format, fo.0)).clicked() {
                                self.event_queue
                                    .send(GuiEvent::CreateNewFloppy(drive_idx, format, fo.1));
                                ui.close_menu();
                            }
                        }
                    }
                });

                ui.separator();

                let floppy_viewer_enabled = self.floppy_drives[drive_idx].filename().is_some()
                    || self.floppy_drives[drive_idx].is_new().is_some();

                if self.workspace_window_open_button(ui, GuiWindow::FloppyViewer, true, floppy_viewer_enabled) {
                    self.floppy_viewer.set_drive_idx(drive_idx);
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if let Some(floppy_name) = &self.floppy_drives[drive_idx].filename() {
                        let type_str = self.floppy_drives[drive_idx].type_string();
                        if ui.button(format!("⏏ Eject {}{}", type_str, floppy_name)).clicked() {
                            self.event_queue.send(GuiEvent::EjectFloppy(drive_idx));
                        }
                    }
                    else if let Some(format) = &self.floppy_drives[drive_idx].is_new() {
                        let type_str = self.floppy_drives[drive_idx].type_string();
                        if ui.button(format!("⏏ Eject {}{}", type_str, format)).clicked() {
                            self.event_queue.send(GuiEvent::EjectFloppy(drive_idx));
                        }
                    }
                    else {
                        ui.add_enabled(false, egui::Button::new("Eject image: <No Image>"));
                    }
                });

                // Add 'Save' option for native build to write back to the currently loaded disk image.
                // This is disabled in the browser due since we can't write to the loaded image.
                #[cfg(not(target_arch = "wasm32"))]
                {
                    ui.horizontal(|ui| {
                        if let Some(floppy_name) = &self.floppy_drives[drive_idx].filename() {
                            ui.add_enabled_ui(self.floppy_drives[drive_idx].is_writeable(), |ui| {
                                let type_str = self.floppy_drives[drive_idx].type_string();
                                if ui.button(format!("💾 Save {}{}", type_str, floppy_name)).clicked() {
                                    if let Some(floppy_path) = self.floppy_drives[drive_idx].file_path() {
                                        if let Some(fmt) = self.floppy_drives[drive_idx].source_format {
                                            self.event_queue.send(GuiEvent::SaveFloppyAs(
                                                drive_idx,
                                                fmt,
                                                floppy_path.clone(),
                                            ));
                                        }
                                    }
                                }
                            });
                        }
                        else {
                            ui.add_enabled(false, egui::Button::new("Save image: <No Image File>"));
                        }
                    });
                }

                // Add 'Save As' options for compatible formats.
                for format_tuple in &self.floppy_drives[drive_idx].supported_formats {
                    let fmt = format_tuple.0;
                    let fmt_name = fmt.to_string();
                    let extensions = &format_tuple.1;

                    if !extensions.is_empty() {
                        if ui
                            .button(format!("Save As .{}...", extensions[0].to_uppercase()))
                            .clicked()
                        {
                            #[cfg(target_arch = "wasm32")]
                            {
                                self.event_queue.send(GuiEvent::RequestSaveFloppyDialog(drive_idx, fmt));
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                let fc = FileSaveContext::FloppyDiskImage {
                                    drive_select: drive_idx,
                                    format: fmt,
                                    fsc: FileSelectionContext::Uninitialized,
                                };

                                let mut filter_vec = Vec::new();
                                let exts = fmt.extensions();
                                filter_vec.push(FileDialogFilter::new(fmt_name, exts));

                                self.save_file_dialog(fc, "Save Floppy Disk Image", filter_vec);

                                self.modal.open(ModalContext::Notice(
                                    "A native File Save dialog is open.\nPlease make a selection or cancel to continue."
                                        .to_string(),
                                ));
                                ui.close_menu();
                            }
                        }
                    }
                }

                if ui
                    .checkbox(&mut self.floppy_drives[drive_idx].write_protected, "Write Protect")
                    .changed()
                {
                    self.event_queue.send(GuiEvent::SetFloppyWriteProtect(
                        drive_idx,
                        self.floppy_drives[drive_idx].write_protected,
                    ));
                }
            })
            .response;
        ui.end_row();
    }

    pub fn draw_hdd_menu(&mut self, ui: &mut egui::Ui, drive_idx: usize) {
        let hdd_name = format!("🖴 Hard Disk {}", drive_idx);

        // Only enable VHD loading if machine is off to prevent corruption to VHD.
        ui.menu_button(hdd_name, |ui| {
            if self.machine_state.is_on() {
                // set 'color' to the appropriate warning color for current egui visuals
                let error_color = ui.visuals().error_fg_color;
                ui.horizontal(|ui| {
                    ui.add(egui::Label::new(
                        egui::RichText::new("Machine must be off to make changes").color(error_color),
                    ));
                });
            }
            ui.add_enabled_ui(!self.machine_state.is_on(), |ui| {
                ui.menu_button("Load image", |ui| {
                    self.hdd_tree_menu.draw(ui, drive_idx, true, &mut |image_idx| {
                        self.event_queue.send(GuiEvent::LoadVHD(drive_idx, image_idx));
                    });
                });

                let (have_vhd, detatch_string) = match &self.hdds[drive_idx].filename() {
                    Some(name) => (true, format!("Detach image: {}", name)),
                    None => (false, "Detach: <No Disk>".to_string()),
                };

                ui.add_enabled_ui(have_vhd, |ui| {
                    if ui.button(detatch_string).clicked() {
                        self.event_queue.send(GuiEvent::DetachVHD(drive_idx));
                    }
                });
            });
        });
    }

    pub fn draw_cart_menu(&mut self, ui: &mut egui::Ui, cart_idx: usize) {
        let cart_name = format!("📼 Cartridge Slot {}", cart_idx);

        ui.menu_button(cart_name, |ui| {
            ui.menu_button("Insert Cartridge", |ui| {
                self.cart_tree_menu.draw(ui, cart_idx, true, &mut |image_idx| {
                    self.event_queue.send(GuiEvent::InsertCartridge(cart_idx, image_idx));
                });
            });

            let (have_cart, detatch_string) = match &self.carts[cart_idx].filename() {
                Some(name) => (true, format!("Remove Cartridge: {}", name)),
                None => (false, "Remove Cartridge: <No Cart>".to_string()),
            };

            ui.add_enabled_ui(have_cart, |ui| {
                ui.horizontal(|ui| {
                    if ui.button(detatch_string).clicked() {
                        self.event_queue.send(GuiEvent::RemoveCartridge(cart_idx));
                    }
                });
            });
        });
    }

    pub fn draw_display_menu(&mut self, ui: &mut egui::Ui, display: DtHandle) {
        // TODO: Refactor all uses of display.into(), to use a hash map of DtHandle instead.
        //       Currently DtHandle is a wrapper around a usize index, but we should make it value
        //       agnostic.
        let vctx = GuiVariableContext::Display(display);

        #[cfg(feature = "scaler_ui")]
        {
            let mut dtype_opt = self
                .get_option_enum_mut(GuiEnum::DisplayType(Default::default()), Some(vctx))
                .and_then(|oe| {
                    if let GuiEnum::DisplayType(dt) = *oe {
                        Some(dt)
                    }
                    else {
                        None
                    }
                });

            ui.menu_button("Display Type", |ui| {
                for dtype in DisplayTargetType::iter() {
                    if let Some(enum_mut) =
                        self.get_option_enum_mut(GuiEnum::DisplayType(Default::default()), Some(vctx))
                    {
                        let checked = *enum_mut == GuiEnum::DisplayType(dtype);

                        if ui.add(egui::RadioButton::new(checked, format!("{}", dtype))).clicked() {
                            *enum_mut = GuiEnum::DisplayType(dtype);
                            self.event_queue.send(GuiEvent::VariableChanged(
                                GuiVariableContext::Display(display),
                                GuiVariable::Enum(GuiEnum::DisplayType(dtype)),
                            ));
                        }
                    }
                }
            });

            if dtype_opt == Some(DisplayTargetType::WindowBackground) {
                ui.menu_button("Scaler Mode", |ui| {
                    for (_scaler_idx, mode) in self.scaler_modes.clone().iter().enumerate() {
                        if let Some(enum_mut) =
                            self.get_option_enum_mut(GuiEnum::DisplayScalerMode(Default::default()), Some(vctx))
                        {
                            let checked = *enum_mut == GuiEnum::DisplayScalerMode(*mode);

                            if ui.add(egui::RadioButton::new(checked, format!("{:?}", mode))).clicked() {
                                *enum_mut = GuiEnum::DisplayScalerMode(*mode);
                                self.event_queue.send(GuiEvent::VariableChanged(
                                    GuiVariableContext::Display(display),
                                    GuiVariable::Enum(GuiEnum::DisplayScalerMode(*mode)),
                                ));
                            }
                        }
                    }
                });
            }
            else {
                ui.menu_button("Window Options", |ui| {
                    // Use a horizontal ui to avoid squished menu
                    ui.horizontal(|ui| {
                        if let Some(enum_mut) =
                            self.get_option_enum_mut(GuiEnum::WindowBezel(Default::default()), Some(vctx))
                        {
                            let mut checked = *enum_mut == GuiEnum::WindowBezel(true);

                            if ui.checkbox(&mut checked, "Bezel Overlay").changed() {
                                *enum_mut = GuiEnum::WindowBezel(checked);
                                self.event_queue.send(GuiEvent::VariableChanged(
                                    GuiVariableContext::Display(display),
                                    GuiVariable::Enum(GuiEnum::WindowBezel(checked)),
                                ));
                            }
                        }
                    });
                });
            }

            ui.menu_button("Scaler Presets", |ui| {
                for (_preset_idx, preset) in self.scaler_presets.clone().iter().enumerate() {
                    if ui.button(preset).clicked() {
                        self.set_option_enum(GuiEnum::DisplayScalerPreset(preset.clone()), Some(vctx));
                        self.event_queue.send(GuiEvent::VariableChanged(
                            GuiVariableContext::Display(display),
                            GuiVariable::Enum(GuiEnum::DisplayScalerPreset(preset.clone())),
                        ));
                        ui.close_menu();
                    }
                }
            });

            if ui.button("Scaler Adjustments...").clicked() {
                *self.window_flag(GuiWindow::ScalerAdjust) = true;
                self.scaler_adjust.select_card(display.into());
                ui.close_menu();
            }
        }

        ui.menu_button("Display Aperture", |ui| {
            let mut aperture_vec = Vec::new();
            if let Some(aperture_vec_ref) = self.display_apertures.get(&display.into()) {
                aperture_vec = aperture_vec_ref.clone()
            };

            for aperture in aperture_vec.iter() {
                if let Some(enum_mut) =
                    self.get_option_enum_mut(GuiEnum::DisplayAperture(Default::default()), Some(vctx))
                {
                    let checked = *enum_mut == GuiEnum::DisplayAperture(aperture.aper_enum);

                    if ui.add(egui::RadioButton::new(checked, aperture.name)).clicked() {
                        *enum_mut = GuiEnum::DisplayAperture(aperture.aper_enum);
                        self.event_queue.send(GuiEvent::VariableChanged(
                            GuiVariableContext::Display(display),
                            GuiVariable::Enum(GuiEnum::DisplayAperture(aperture.aper_enum)),
                        ));
                    }
                }
            }
        });

        let mut state_changed = false;
        let mut new_state = false;
        if let Some(GuiEnum::DisplayAspectCorrect(state)) =
            &mut self.get_option_enum_mut(GuiEnum::DisplayAspectCorrect(false), Some(vctx))
        {
            if ui.checkbox(state, "Correct Aspect Ratio").clicked() {
                //let new_opt = self.get_option_enum_mut()
                state_changed = true;
                new_state = *state;
                ui.close_menu();
            }
        }
        if state_changed {
            self.event_queue.send(GuiEvent::VariableChanged(
                GuiVariableContext::Display(display),
                GuiVariable::Enum(GuiEnum::DisplayAspectCorrect(new_state)),
            ));
        }

        // CGA-specific options.
        if matches!(self.display_info[usize::from(display)].vtype, Some(VideoType::CGA)) {
            let mut state_changed = false;
            let mut new_state = false;

            if let Some(GuiEnum::DisplayComposite(state)) =
                self.get_option_enum_mut(GuiEnum::DisplayComposite(Default::default()), Some(vctx))
            {
                if ui.checkbox(state, "Composite Monitor").clicked() {
                    state_changed = true;
                    new_state = *state;
                    ui.close_menu();
                }
            }
            if state_changed {
                self.event_queue.send(GuiEvent::VariableChanged(
                    GuiVariableContext::Display(display),
                    GuiVariable::Enum(GuiEnum::DisplayComposite(new_state)),
                ));
            }

            /* TODO: Snow should be set per-adapter, not per-display
            if ui
                .checkbox(&mut self.get_option_mut(GuiBoolean::EnableSnow), "Enable Snow")
                .clicked()
            {
                let new_opt = self.get_option(GuiBoolean::EnableSnow).unwrap();

                self.event_queue.send(GuiEvent::OptionChanged(GuiOption::Bool(
                    GuiBoolean::EnableSnow,
                    new_opt,
                )));

                ui.close_menu();
            }
             */

            if ui.button("Composite Adjustments...").clicked() {
                *self.window_flag(GuiWindow::CompositeAdjust) = true;
                self.composite_adjust.select_card(display.into());
                ui.close_menu();
            }
        }

        self.workspace_window_open_button_with(ui, GuiWindow::TextModeViewer, true, |state| {
            state.text_mode_viewer.select_card(display.into());
        });

        // On the web, fullscreen is basically free when the user hits f11 to go fullscreen.
        // We can't programmatically request fullscreen. So, we don't show the option.
        #[cfg(not(target_arch = "wasm32"))]
        if ui.button("🖵 Toggle Fullscreen").clicked() {
            self.event_queue.send(GuiEvent::ToggleFullscreen(display.into()));
            ui.close_menu();
        };

        ui.separator();

        if ui.button("🖼 Take Screenshot").clicked() {
            self.event_queue.send(GuiEvent::TakeScreenshot(display.into()));
            ui.close_menu();
        };
    }

    pub fn draw_sound_menu(&mut self, ui: &mut egui::Ui) {
        let mut sources = self.sound_sources.clone();

        for (snd_idx, source) in &mut sources.iter_mut().enumerate() {
            let icon = match source.muted {
                true => IconType::SpeakerMuted,
                false => IconType::Speaker,
            };

            let mut volume = source.volume;

            let sctx = GuiVariableContext::SoundSource(snd_idx);

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label(format!("{}", source.name));
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(BigIcon::new(icon, Some(icon.default_color(ui))).medium().text())
                                    .frame(true),
                            )
                            .clicked()
                        {
                            log::warn!("Mute button clicked");
                            source.muted = !source.muted;

                            if let Some(GuiEnum::AudioMuted(state)) =
                                self.get_option_enum_mut(GuiEnum::AudioMuted(Default::default()), Some(sctx))
                            {
                                *state = source.muted;
                                self.event_queue.send(GuiEvent::VariableChanged(
                                    GuiVariableContext::SoundSource(snd_idx),
                                    GuiVariable::Enum(GuiEnum::AudioMuted(source.muted)),
                                ));
                            }
                        };

                        if ui
                            .add(egui::Slider::new(&mut source.volume, 0.0..=1.0).text("Volume"))
                            .changed()
                        {
                            if let Some(GuiEnum::AudioVolume(vol)) =
                                self.get_option_enum_mut(GuiEnum::AudioVolume(Default::default()), Some(sctx))
                            {
                                *vol = source.volume;
                                self.event_queue.send(GuiEvent::VariableChanged(
                                    GuiVariableContext::SoundSource(snd_idx),
                                    GuiVariable::Enum(GuiEnum::AudioVolume(source.volume)),
                                ));
                            }
                        }
                    });
                    ui.label(format!("Sample Rate: {}Hz", source.sample_rate));
                    ui.label(format!("Latency: {:.0}ms", source.latency_ms));
                    // ui.label(format!("Samples: {}", source.sample_ct));
                    // ui.label(format!("Buffers: {}", source.len));
                });
            });
        }
    }

    pub fn draw_status_widgets(&mut self, _ui: &mut egui::Ui) {
        // Can we put stuff on the right hand side of the menu bar?
        // ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
        //     ui.label("💾");
        // });
        //
        // ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
        //     ui.label("🐢");
        // });
    }
}
