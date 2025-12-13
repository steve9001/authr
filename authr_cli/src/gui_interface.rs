#[cfg(feature = "gui")]
use eframe::egui;
#[cfg(feature = "gui")]
use anyhow::Result;

#[cfg(feature = "gui")]
pub fn run() -> Result<()> {
    // Basic eframe setup
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Authr",
        options,
        Box::new(|_cc| Ok(Box::new(AuthrApp::default()))),
    ).map_err(|e| anyhow::anyhow!("Eframe error: {}", e))?;
    Ok(())
}

#[cfg(feature = "gui")]
struct AuthrApp {
    // State will go here
}

#[cfg(feature = "gui")]
impl Default for AuthrApp {
    fn default() -> Self {
        Self {}
    }
}

#[cfg(feature = "gui")]
impl eframe::App for AuthrApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Authr (GUI Mode)");
            ui.label("Not implemented yet.");
        });
    }
}

#[cfg(not(feature = "gui"))]
#[allow(dead_code)]
pub fn run() -> anyhow::Result<()> {
    anyhow::bail!("GUI feature not enabled");
}
