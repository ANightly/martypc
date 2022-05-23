use egui::{ClippedMesh, Context, TexturesDelta};
use egui_wgpu_backend::{BackendError, RenderPass, ScreenDescriptor};
use pixels::{wgpu, PixelsContext};
use winit::window::Window;
use crate::{
    cpu::CpuStringState, 
    pit::PitStringState, 
    pic::PicStringState,
    ppi::PpiStringState};

//use crate::syntax_highlighting::code_view_ui;

/// Manages all state required for rendering egui over `Pixels`.
pub(crate) struct Framework {
    // State for egui.
    egui_ctx: Context,
    egui_state: egui_winit::State,
    screen_descriptor: ScreenDescriptor,
    rpass: RenderPass,
    paint_jobs: Vec<ClippedMesh>,
    textures: TexturesDelta,

    // State for the GUI
    pub gui: Gui,
}

/// Example application state. A real application will need a lot more state than this.
pub(crate) struct Gui {
    /// Only show the egui window when true.
    window_open: bool,
    error_dialog_open: bool,
    cpu_control_dialog_open: bool,
    memory_viewer_open: bool,
    register_viewer_open: bool,
    trace_viewer_open: bool,
    disassembly_viewer_open: bool,
    pit_viewer_open: bool,
    pic_viewer_open: bool,
    ppi_viewer_open: bool,

    cpu_single_step: bool,
    cpu_step_flag: bool,

    error_string: String,
    pub memory_viewer_address: String,
    pub cpu_state: CpuStringState,
    pub breakpoint: String,
    pub pit_state: PitStringState,
    pub pic_state: PicStringState,
    pub ppi_state: PpiStringState,
    memory_viewer_dump: String,
    disassembly_viewer_string: String,
    disassembly_viewer_address: String,
    trace_string: String
}

impl Framework {
    /// Create egui.
    pub(crate) fn new(width: u32, height: u32, scale_factor: f32, pixels: &pixels::Pixels) -> Self {
        let max_texture_size = pixels.device().limits().max_texture_dimension_2d as usize;

        let egui_ctx = Context::default();
        let egui_state = egui_winit::State::from_pixels_per_point(max_texture_size, scale_factor);
        let screen_descriptor = ScreenDescriptor {
            physical_width: width,
            physical_height: height,
            scale_factor,
        };
        let rpass = RenderPass::new(pixels.device(), pixels.render_texture_format(), 1);
        let textures = TexturesDelta::default();
        let gui = Gui::new();

        Self {
            egui_ctx,
            egui_state,
            screen_descriptor,
            rpass,
            paint_jobs: Vec::new(),
            textures,
            gui,
        }
    }

    pub(crate) fn has_focus(&self) -> bool {
        match self.egui_ctx.memory().focus() {
            Some(_) => true,
            None => false
        }
    }

    /// Handle input events from the window manager.
    pub(crate) fn handle_event(&mut self, event: &winit::event::WindowEvent) {
        self.egui_state.on_event(&self.egui_ctx, event);
    }

    /// Resize egui.
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.screen_descriptor.physical_width = width;
            self.screen_descriptor.physical_height = height;
        }
    }

    /// Update scaling factor.
    pub(crate) fn scale_factor(&mut self, scale_factor: f64) {
        self.screen_descriptor.scale_factor = scale_factor as f32;
    }

    /// Prepare egui.
    pub(crate) fn prepare(&mut self, window: &Window) {
        // Run the egui frame and create all paint jobs to prepare for rendering.
        let raw_input = self.egui_state.take_egui_input(window);
        let output = self.egui_ctx.run(raw_input, |egui_ctx| {
            // Draw the demo application.
            self.gui.ui(egui_ctx);
        });

        self.textures.append(output.textures_delta);
        self.egui_state
            .handle_platform_output(window, &self.egui_ctx, output.platform_output);
        self.paint_jobs = self.egui_ctx.tessellate(output.shapes);
    }

    /// Render egui.
    pub(crate) fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        render_target: &wgpu::TextureView,
        context: &PixelsContext,
    ) -> Result<(), BackendError> {
        // Upload all resources to the GPU.
        self.rpass
            .add_textures(&context.device, &context.queue, &self.textures)?;
        self.rpass.update_buffers(
            &context.device,
            &context.queue,
            &self.paint_jobs,
            &self.screen_descriptor,
        );

        // Record all render passes.
        self.rpass.execute(
            encoder,
            render_target,
            &self.paint_jobs,
            &self.screen_descriptor,
            None,
        )?;

        // Cleanup
        let textures = std::mem::take(&mut self.textures);
        self.rpass.remove_textures(textures)
    }
}

impl Gui {
    /// Create a `Gui`.
    fn new() -> Self {
        Self { 
            window_open: false, 
            error_dialog_open: false,
            cpu_control_dialog_open: true,
            memory_viewer_open: false,
            register_viewer_open: true,
            disassembly_viewer_open: true,
            trace_viewer_open: false,
            pit_viewer_open: false,
            pic_viewer_open: false,
            ppi_viewer_open: false,

            cpu_single_step: true,
            cpu_step_flag: false,

            error_string: String::new(),
            memory_viewer_address: String::new(),
            memory_viewer_dump: String::new(),
            cpu_state: Default::default(),
            breakpoint: String::new(),
            pit_state: Default::default(),
            pic_state: Default::default(),
            ppi_state: Default::default(),
            disassembly_viewer_string: String::new(),
            disassembly_viewer_address: "cs:ip".to_string(),
            trace_string: String::new(),

        }
    }

    pub fn get_cpu_single_step(&self) -> bool {
        self.cpu_single_step
    }

    pub fn set_cpu_single_step(&mut self) {
        self.cpu_single_step = true
    }

    pub fn get_cpu_step_flag(&mut self) -> bool {
        let flag = self.cpu_step_flag;
        self.cpu_step_flag = false;
        return flag
    }

    pub fn show_error(&mut self, err_str: &str) {
        self.error_dialog_open = true;
        self.error_string = err_str.to_string();
    }

    pub fn update_memory_view(&mut self, mem_str: String) {
        self.memory_viewer_dump = mem_str;
    }

    pub fn get_memory_view_address(&mut self) -> &str {
        &self.memory_viewer_address
    }

    pub fn show_disassembly_view(&mut self) {
        self.disassembly_viewer_open = true
    }

    pub fn get_disassembly_view_address(&mut self) -> &str {
        &self.disassembly_viewer_address
    }

    pub fn update_dissassembly_view(&mut self, disassembly_string: String) {
        self.disassembly_viewer_string = disassembly_string;
    }

    pub fn update_cpu_state(&mut self, state: CpuStringState) {
        self.cpu_state = state.clone();
    }

    pub fn update_pic_state(&mut self, state: PicStringState) {
        self.pic_state = state;
    }

    pub fn get_breakpoint(&mut self) -> &str {
        &self.breakpoint
    }

    pub fn update_pit_state(&mut self, state: PitStringState) {
        self.pit_state = state.clone();
    }

    pub fn update_trace_state(&mut self, trace_string: String) {
        self.trace_string = trace_string;
    }

    pub fn update_ppi_state(&mut self, state: PpiStringState) {
        self.ppi_state = state;
    }
    /// Create the UI using egui.
    fn ui(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("menubar_container").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("About...").clicked() {
                        self.window_open = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Debug", |ui| {
                    if ui.button("CPU Control...").clicked() {
                        self.cpu_control_dialog_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Memory...").clicked() {
                        self.memory_viewer_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Registers...").clicked() {
                        self.register_viewer_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Instruction Trace...").clicked() {
                        self.trace_viewer_open = true;
                        ui.close_menu();
                    }
                    if ui.button("Disassembly...").clicked() {
                        self.disassembly_viewer_open = true;
                        ui.close_menu();
                    }
                    if ui.button("PIC...").clicked() {
                        self.pic_viewer_open = true;
                        ui.close_menu();
                    }    
                    if ui.button("PIT...").clicked() {
                        self.pit_viewer_open = true;
                        ui.close_menu();
                    }
                    if ui.button("PPI...").clicked() {
                        self.ppi_viewer_open = true;
                        ui.close_menu();
                    }    
                
                });
            });
        });

        egui::Window::new("Hello, egui!")
            .open(&mut self.window_open)
            .show(ctx, |ui| {
                ui.label("This example demonstrates using egui with pixels.");
                ui.label("Made with 💖 in San Francisco!");

                ui.separator();

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x /= 2.0;
                    ui.label("Learn more about egui at");
                    ui.hyperlink("https://docs.rs/egui");
                });
            });

        egui::Window::new("Error")
            .open(&mut self.error_dialog_open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("❎").color(egui::Color32::RED).font(egui::FontId::proportional(40.0)));
                    ui.label(&self.error_string);
                });
                
            });

        egui::Window::new("CPU Control")
            .open(&mut self.cpu_control_dialog_open)
            .show(ctx, |ui| {
                ui.horizontal(|ui|{
                    if ui.button(egui::RichText::new("⏸").font(egui::FontId::proportional(20.0))).clicked() {
                        self.cpu_single_step = true;
                    };
                    if ui.button(egui::RichText::new("⏭").font(egui::FontId::proportional(20.0))).clicked() {
                        self.cpu_step_flag = true;
                        //println!("step")
                    };
                    if ui.button(egui::RichText::new("▶").font(egui::FontId::proportional(20.0))).clicked() {
                        self.cpu_single_step = false;
                    };
                });
                ui.separator();
                ui.horizontal(|ui|{
                    ui.label("Breakpoint: ");
                    ui.text_edit_singleline(&mut self.breakpoint);
                });
            });

        egui::Window::new("Memory View")
            .open(&mut self.memory_viewer_open)
            .resizable(true)
            .default_width(540.0)
            .show(ctx, |ui| {

                ui.horizontal(|ui| {
                    ui.label("Address: ");
                    ui.text_edit_singleline(&mut self.memory_viewer_address);
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.add_sized(ui.available_size(), 
                        egui::TextEdit::multiline(&mut self.memory_viewer_dump)
                            .font(egui::TextStyle::Monospace));
                    ui.end_row()
                });
            });

        egui::Window::new("Trace View")
            .open(&mut self.trace_viewer_open)
            .resizable(true)
            .default_width(540.0)
            .show(ctx, |ui| {

                ui.horizontal(|ui| {
                    ui.add_sized(ui.available_size(), 
                        egui::TextEdit::multiline(&mut self.trace_string)
                            .font(egui::TextStyle::Monospace));
                    ui.end_row()
                });
            });       

        egui::Window::new("Disassembly View")
            .open(&mut self.disassembly_viewer_open)
            .resizable(true)
            .default_width(540.0)
            .show(ctx, |ui| {

                ui.horizontal(|ui| {
                    ui.label("Address: ");
                    ui.text_edit_singleline(&mut self.disassembly_viewer_address);
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.add_sized(ui.available_size(), 
                        egui::TextEdit::multiline(&mut self.disassembly_viewer_string)
                            .font(egui::TextStyle::Monospace));
                    ui.end_row()
                });
            });             

        egui::Window::new("Register View")
            .open(&mut self.register_viewer_open)
            .resizable(false)
            .default_width(220.0)
            .show(ctx, |ui| {
                egui::Grid::new("reg_general")
                    .striped(true)
                    .min_col_width(100.0)
                    .show(ui, |ui| {

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("AH:").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.cpu_state.ah).font(egui::TextStyle::Monospace));
                    });
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("AL:").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.cpu_state.al).font(egui::TextStyle::Monospace));
                    });
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("AX:").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.cpu_state.ax).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("BH:").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.cpu_state.bh).font(egui::TextStyle::Monospace));
                    });
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("BL:").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.cpu_state.bl).font(egui::TextStyle::Monospace));
                    });
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("BX:").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.cpu_state.bx).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("CH:").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.cpu_state.ch).font(egui::TextStyle::Monospace));
                    });
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("CL:").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.cpu_state.cl).font(egui::TextStyle::Monospace));
                    });
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("CX:").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.cpu_state.cx).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("DH:").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.cpu_state.dh).font(egui::TextStyle::Monospace));
                    });
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("DL:").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.cpu_state.dl).font(egui::TextStyle::Monospace));
                    });
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("DX:").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.cpu_state.dx).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();
                                       
                });

                ui.separator();

                egui::Grid::new("reg_segment")
                    .striped(true)
                    .min_col_width(100.0)
                    .show(ui, |ui| {

                        ui.horizontal( |ui| {
                            //ui.add(egui::Label::new("SP:"));
                            ui.label(egui::RichText::new("SP:").text_style(egui::TextStyle::Monospace));
                            ui.add(egui::TextEdit::singleline(&mut self.cpu_state.sp).font(egui::TextStyle::Monospace));
                        });
                        ui.horizontal( |ui| {
                            ui.label(egui::RichText::new("ES:").text_style(egui::TextStyle::Monospace));
                            ui.add(egui::TextEdit::singleline(&mut self.cpu_state.es).font(egui::TextStyle::Monospace));
                        });                        
                        ui.end_row();  
                        ui.horizontal( |ui| {
                            ui.label(egui::RichText::new("BP:").text_style(egui::TextStyle::Monospace));
                            ui.add(egui::TextEdit::singleline(&mut self.cpu_state.bp).font(egui::TextStyle::Monospace));
                        });
                        ui.horizontal( |ui| {
                            ui.label(egui::RichText::new("CS:").text_style(egui::TextStyle::Monospace));
                            ui.add(egui::TextEdit::singleline(&mut self.cpu_state.cs).font(egui::TextStyle::Monospace));
                        });                         
                        ui.end_row();  
                        ui.horizontal( |ui| {
                            ui.label(egui::RichText::new("SI:").text_style(egui::TextStyle::Monospace));
                            ui.add(egui::TextEdit::singleline(&mut self.cpu_state.si).font(egui::TextStyle::Monospace));
                        });
                        ui.horizontal( |ui| {
                            ui.label(egui::RichText::new("SS:").text_style(egui::TextStyle::Monospace));
                            ui.add(egui::TextEdit::singleline(&mut self.cpu_state.ss).font(egui::TextStyle::Monospace));
                        });                         
                        ui.end_row();  
                        ui.horizontal( |ui| {
                            ui.label(egui::RichText::new("DI:").text_style(egui::TextStyle::Monospace));
                            ui.add(egui::TextEdit::singleline(&mut self.cpu_state.di).font(egui::TextStyle::Monospace));
                        });
                        ui.horizontal( |ui| {
                            ui.label(egui::RichText::new("DS:").text_style(egui::TextStyle::Monospace));
                            ui.add(egui::TextEdit::singleline(&mut self.cpu_state.ds).font(egui::TextStyle::Monospace));
                        });                         
                        ui.end_row();  
                        ui.label("");
                        ui.horizontal( |ui| {
                            ui.label(egui::RichText::new("IP:").text_style(egui::TextStyle::Monospace));
                            ui.add(egui::TextEdit::singleline(&mut self.cpu_state.ip).font(egui::TextStyle::Monospace));
                            //ui.text_edit_singleline(&mut self.memory_viewer_address);
                        }); 
                        ui.end_row();  
                    });

                ui.separator();

                egui::Grid::new("reg_flags")
                    .striped(true)
                    .max_col_width(15.0)
                    .show(ui, |ui| {
                        //const CPU_FLAG_CARRY: u16      = 0b0000_0000_0001;
                        //const CPU_FLAG_RESERVED1: u16  = 0b0000_0000_0010;
                        //const CPU_FLAG_PARITY: u16     = 0b0000_0000_0100;
                        //const CPU_FLAG_AUX_CARRY: u16  = 0b0000_0001_0000;
                        //const CPU_FLAG_ZERO: u16       = 0b0000_0100_0000;
                        //const CPU_FLAG_SIGN: u16       = 0b0000_1000_0000;
                        //const CPU_FLAG_TRAP: u16       = 0b0001_0000_0000;
                        //const CPU_FLAG_INT_ENABLE: u16 = 0b0010_0000_0000;
                        //const CPU_FLAG_DIRECTION: u16  = 0b0100_0000_0000;
                        //const CPU_FLAG_OVERFLOW: u16   = 0b1000_0000_0000;

                        ui.horizontal( |ui| {
                            //ui.add(egui::Label::new("SP:"));
                            ui.label(egui::RichText::new("O:").text_style(egui::TextStyle::Monospace));
                            ui.add(egui::TextEdit::singleline(&mut self.cpu_state.o_fl).font(egui::TextStyle::Monospace));
                            ui.label(egui::RichText::new("D:").text_style(egui::TextStyle::Monospace));
                            ui.add(egui::TextEdit::singleline(&mut self.cpu_state.d_fl).font(egui::TextStyle::Monospace)); 
                            ui.label(egui::RichText::new("I:").text_style(egui::TextStyle::Monospace));
                            ui.add(egui::TextEdit::singleline(&mut self.cpu_state.i_fl).font(egui::TextStyle::Monospace));  
                            ui.label(egui::RichText::new("T:").text_style(egui::TextStyle::Monospace));
                            ui.add(egui::TextEdit::singleline(&mut self.cpu_state.t_fl).font(egui::TextStyle::Monospace));
                            ui.label(egui::RichText::new("S:").text_style(egui::TextStyle::Monospace));
                            ui.add(egui::TextEdit::singleline(&mut self.cpu_state.s_fl).font(egui::TextStyle::Monospace));
                            ui.label(egui::RichText::new("Z:").text_style(egui::TextStyle::Monospace));
                            ui.add(egui::TextEdit::singleline(&mut self.cpu_state.z_fl).font(egui::TextStyle::Monospace));      
                            ui.label(egui::RichText::new("A:").text_style(egui::TextStyle::Monospace));
                            ui.add(egui::TextEdit::singleline(&mut self.cpu_state.a_fl).font(egui::TextStyle::Monospace));  
                            ui.label(egui::RichText::new("P:").text_style(egui::TextStyle::Monospace));
                            ui.add(egui::TextEdit::singleline(&mut self.cpu_state.p_fl).font(egui::TextStyle::Monospace));             
                            ui.label(egui::RichText::new("C:").text_style(egui::TextStyle::Monospace));
                            ui.add(egui::TextEdit::singleline(&mut self.cpu_state.c_fl).font(egui::TextStyle::Monospace));                                        
                        });

                        ui.end_row();  
                    });
            });        
            
        egui::Window::new("PIT View")
            .open(&mut self.pit_viewer_open)
            .resizable(true)
            .default_width(600.0)
            .show(ctx, |ui| {
                egui::Grid::new("pit_view")
                    .striped(true)
                    .min_col_width(300.0)
                    .show(ui, |ui| {

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("#0 Access Mode: ").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.pit_state.c0_access_mode).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("#0 Channel Mode:").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.pit_state.c0_channel_mode).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("#0 Counter:     ").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.pit_state.c0_value).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("#0 Reload Val:  ").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.pit_state.c0_reload_value).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();
                    
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("#1 Access Mode: ").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.pit_state.c1_access_mode).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("#1 Channel Mode:").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.pit_state.c1_channel_mode).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("#1 Counter:     ").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.pit_state.c1_value).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("#1 Reload Val:  ").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.pit_state.c1_reload_value).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();  
                    
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("#2 Access Mode: ").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.pit_state.c2_access_mode).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("#2 Channel Mode:").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.pit_state.c2_channel_mode).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("#2 Counter:     ").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.pit_state.c2_value).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("#2 Reload Val:  ").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.pit_state.c2_reload_value).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();                       
                });
            });               

            egui::Window::new("PIC View")
            .open(&mut self.pic_viewer_open)
            .resizable(true)
            .default_width(600.0)
            .show(ctx, |ui| {
                egui::Grid::new("pic_view")
                    .striped(true)
                    .min_col_width(300.0)
                    .show(ui, |ui| {

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("IMR Register: ").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.pic_state.imr).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("IRR Register: ").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.pic_state.irr).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("ISR Register: ").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.pic_state.isr).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();

                    for i in 0..self.pic_state.interrupt_stats.len() {
                        ui.horizontal(|ui| {
                            let label_str = format!("IRQ {} IMR Masked: ", i );
                            ui.label(egui::RichText::new(label_str).text_style(egui::TextStyle::Monospace));
                            ui.add(egui::TextEdit::singleline(&mut self.pic_state.interrupt_stats[i].0).font(egui::TextStyle::Monospace));
                        });
                        ui.end_row();
                        ui.horizontal(|ui| {
                            let label_str = format!("IRQ {} ISR Masked: ", i );
                            ui.label(egui::RichText::new(label_str).text_style(egui::TextStyle::Monospace));
                            ui.add(egui::TextEdit::singleline(&mut self.pic_state.interrupt_stats[i].1).font(egui::TextStyle::Monospace));
                        });
                        ui.end_row();
                        ui.horizontal(|ui| {
                            let label_str = format!("IRQ {} Serviced:   ", i );
                            ui.label(egui::RichText::new(label_str).text_style(egui::TextStyle::Monospace));
                            ui.add(egui::TextEdit::singleline(&mut self.pic_state.interrupt_stats[i].2).font(egui::TextStyle::Monospace));
                        });
                        ui.end_row();                                                
                    }
                      
                });
            });           
            
            egui::Window::new("PPI View")
            .open(&mut self.ppi_viewer_open)
            .resizable(true)
            .default_width(600.0)
            .show(ctx, |ui| {
                egui::Grid::new("ppi_view")
                    .striped(true)
                    .min_col_width(300.0)
                    .show(ui, |ui| {

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Port A Mode:  ").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.ppi_state.port_a_mode).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Port A Value: ").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.ppi_state.port_a_value_bin).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Port A Value: ").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.ppi_state.port_a_value_hex).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Port C Mode:  ").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.ppi_state.port_c_mode).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Port C Value: ").text_style(egui::TextStyle::Monospace));
                        ui.add(egui::TextEdit::singleline(&mut self.ppi_state.port_c_value).font(egui::TextStyle::Monospace));
                    });
                    ui.end_row();
                });
            });           
    }
}