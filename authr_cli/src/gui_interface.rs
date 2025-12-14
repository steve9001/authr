#[cfg(feature = "gui")]
use eframe::egui;
#[cfg(feature = "gui")]
use anyhow::Result;
#[cfg(feature = "gui")]
use authr_core::{model::Account, storage::load_accounts, totp};
#[cfg(feature = "gui")]
use std::time::{SystemTime, UNIX_EPOCH, Duration};

#[cfg(feature = "gui")]
pub fn run() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 600.0])
            .with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Authr",
        options,
        Box::new(|_cc| Ok(Box::new(AuthrApp::new()))),
    ).map_err(|e| anyhow::anyhow!("Eframe error: {}", e))?;
    Ok(())
}


#[cfg(feature = "gui")]
struct AuthrApp {
    accounts: Vec<Account>,
    filter: String,
    error: Option<String>,
    last_copied: Option<(String, SystemTime)>,
}

#[cfg(feature = "gui")]
impl AuthrApp {
    fn new() -> Self {
        match load_accounts() {
            Ok(accounts) => Self {
                accounts,
                filter: String::new(),
                error: None,
                last_copied: None,
            },
            Err(e) => Self {
                accounts: vec![],
                filter: String::new(),
                error: Some(e.to_string()),
                last_copied: None,
            }
        }
    }
}
// Note: Trait impl Default calls new(), so it's fine.

#[cfg(feature = "gui")]
impl eframe::App for AuthrApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Enforce dark mode
        ctx.set_visuals(egui::Visuals::dark());

        // Redraw loop
        ctx.request_repaint_after(Duration::from_millis(100));

        // Timer calculation
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let remaining = 30 - (now % 30);
        
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                
                // Prominent Timer
                let timer_color = if remaining < 5 {
                    egui::Color32::from_rgb(255, 80, 80) // Brighter Red
                } else {
                    egui::Color32::from_rgb(80, 255, 80) // Brighter Green
                };
                
                ui.label(
                    egui::RichText::new(format!("{}", remaining))
                        .size(64.0)
                        .strong()
                        .color(timer_color)
                );
                
                ui.add_space(15.0);
                ui.separator();
                ui.add_space(10.0);

                // Search Bar
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("🔍").size(18.0));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.filter)
                            .desired_width(f32::INFINITY)
                            .hint_text("Search accounts...")
                    ).request_focus();
                    ui.add_space(10.0);
                });
                
                ui.add_space(10.0);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(err) = &self.error {
                ui.centered_and_justified(|ui| {
                    ui.label(egui::RichText::new(format!("Error loading accounts: {}", err)).color(egui::Color32::RED));
                });
                return;
            }

            let filtered: Vec<&Account> = if self.filter.is_empty() {
                self.accounts.iter().collect()
            } else {
                self.accounts.iter()
                    .filter(|a| a.name.to_lowercase().contains(&self.filter.to_lowercase()))
                    .collect()
            };

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(10.0);
                for account in filtered {
                    let mut is_flashing = false;
                    if let Some((name, time)) = &self.last_copied {
                        if name == &account.name {
                            if let Ok(elapsed) = time.elapsed() {
                                if elapsed.as_millis() < 300 {
                                    is_flashing = true;
                                }
                            }
                        }
                    }

                    let bg_color = if is_flashing {
                        egui::Color32::from_rgb(60, 65, 80) // Visual feedback color
                    } else {
                        ui.style().visuals.faint_bg_color
                    };

                    let frame = egui::Frame::group(ui.style())
                        .rounding(10.0)
                        .inner_margin(10.0)
                        .fill(bg_color); // Use explicit fill

                    let response = frame.show(ui, |ui| {
                        ui.set_width(ui.available_width()); // Ensure frame content fills width for clicks
                        ui.horizontal(|ui| {
                            // Account Name
                            ui.label(egui::RichText::new(&account.name).size(18.0).strong());
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                // Generate code
                                let code = totp::generate_code(account).unwrap_or_else(|_| "ERROR".to_string());
                                
                                // Display Code (as simple label now)
                                ui.label(egui::RichText::new(&code).size(22.0).monospace().color(egui::Color32::LIGHT_BLUE));
                            });
                        });
                    });

                    // Make the entire frame content area clickable
                    let rect = response.response.rect.expand(2.0); // Slightly expand to cover margin edges
                    let interact = ui.interact(rect, ui.make_persistent_id(&account.name), egui::Sense::click());
                    
                    if interact.clicked() {
                        let code = totp::generate_code(account).unwrap_or_else(|_| "ERROR".to_string());
                        ui.output_mut(|o| o.copied_text = code);
                        self.last_copied = Some((account.name.clone(), SystemTime::now()));
                    }

                    // Enforce hand cursor on hover
                    if interact.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }

                    ui.add_space(5.0);
                }
                ui.add_space(10.0);
            });
        });
    }
}

#[cfg(feature = "gui")]
fn load_icon() -> egui::IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let icon = include_bytes!("../assets/icon.png");
        let image = image::load_from_memory(icon)
            .expect("Failed to load icon")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    egui::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}

#[cfg(not(feature = "gui"))]
#[allow(dead_code)]
pub fn run() -> anyhow::Result<()> {
    anyhow::bail!("GUI feature not enabled");
}
