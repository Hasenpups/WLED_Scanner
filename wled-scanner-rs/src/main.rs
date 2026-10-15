// Hide the console window on Windows release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::collections::HashSet;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use eframe::egui;
use egui_extras::{Column, TableBuilder};
use serde::Deserialize;
use socket2::{Domain, Protocol, Socket, Type};

/// UDP port WLED uses to broadcast node info ("Discover nodes").
const DISCOVERY_PORT: u16 = 65506;
const HTTP_TIMEOUT: Duration = Duration::from_secs(5);

/// Subset of WLED's `/json/info` response that the scanner displays.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct WledInfo {
    name: String,
    ver: String,
    arch: String,
    uptime: u64,
    ip: String,
}

struct Device {
    name: String,
    ip: String,
    version: String,
    arch: String,
    uptime: String,
}

enum Event {
    /// A discovery packet announced this IP.
    Announced(Ipv4Addr),
    /// `/json/info` for an IP was fetched (or failed). Tagged with the list generation.
    Info(u64, Ipv4Addr, Option<WledInfo>),
}

struct App {
    tx: Sender<Event>,
    rx: Receiver<Event>,
    ctx: egui::Context,
    devices: Vec<Device>,
    /// IPs already listed or currently being queried.
    known: HashSet<Ipv4Addr>,
    /// Bumped on "Clear list" so late HTTP results from before the clear are dropped.
    generation: u64,
    selected: Option<usize>,
    status: String,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = mpsc::channel();
        let ctx = cc.egui_ctx.clone();

        let status = match bind_discovery_socket() {
            Ok(socket) => {
                let (tx, ctx) = (tx.clone(), ctx.clone());
                thread::spawn(move || listen(socket, tx, ctx));
                format!("Listening for WLED devices on UDP port {DISCOVERY_PORT}…")
            }
            Err(e) => format!("Could not bind UDP port {DISCOVERY_PORT}: {e}"),
        };

        Self {
            tx,
            rx,
            ctx,
            devices: Vec::new(),
            known: HashSet::new(),
            generation: 0,
            selected: None,
            status,
        }
    }

    fn handle_events(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::Announced(ip) => {
                    if self.known.insert(ip) {
                        let (tx, ctx, generation) =
                            (self.tx.clone(), self.ctx.clone(), self.generation);
                        thread::spawn(move || {
                            let info = fetch_info(ip);
                            let _ = tx.send(Event::Info(generation, ip, info));
                            ctx.request_repaint();
                        });
                    }
                }
                Event::Info(generation, _, _) if generation != self.generation => {}
                Event::Info(_, ip, Some(info)) => self.devices.push(Device {
                    name: info.name,
                    // Prefer the IP the device reports, fall back to the announced one.
                    ip: if info.ip.is_empty() {
                        ip.to_string()
                    } else {
                        info.ip
                    },
                    version: info.ver,
                    arch: info.arch,
                    uptime: format_uptime(info.uptime),
                }),
                // Forget failed lookups so the next announcement retries.
                Event::Info(_, ip, None) => {
                    self.known.remove(&ip);
                }
            }
        }
    }

    fn clear(&mut self) {
        self.devices.clear();
        self.known.clear();
        self.selected = None;
        self.generation += 1;
    }

    fn open_selected(&self) {
        if let Some(device) = self.selected.and_then(|i| self.devices.get(i)) {
            let _ = open::that_detached(format!("http://{}", device.ip));
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.handle_events();

        egui::Panel::bottom("buttons").show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Clear list").clicked() {
                    self.clear();
                }
                ui.label(&self.status);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let open = ui.add_enabled(
                        self.selected.is_some(),
                        egui::Button::new("Open in browser"),
                    );
                    if open.clicked() {
                        self.open_selected();
                    }
                });
            });
            ui.add_space(6.0);
        });

        egui::CentralPanel::default().show(ui, |ui| {
            let mut open_requested = false;
            TableBuilder::new(ui)
                .striped(true)
                .sense(egui::Sense::click())
                .column(Column::auto().at_least(140.0).resizable(true))
                .column(Column::auto().at_least(110.0).resizable(true))
                .column(Column::auto().at_least(80.0).resizable(true))
                .column(Column::auto().at_least(90.0).resizable(true))
                .column(Column::remainder())
                .header(22.0, |mut header| {
                    for title in [
                        "Name",
                        "IP",
                        "Version",
                        "Architecture",
                        "Uptime (dd:hh:mm:ss)",
                    ] {
                        header.col(|ui| {
                            ui.strong(title);
                        });
                    }
                })
                .body(|body| {
                    body.rows(20.0, self.devices.len(), |mut row| {
                        let index = row.index();
                        row.set_selected(self.selected == Some(index));
                        let device = &self.devices[index];
                        for text in [
                            &device.name,
                            &device.ip,
                            &device.version,
                            &device.arch,
                            &device.uptime,
                        ] {
                            row.col(|ui| {
                                ui.add(egui::Label::new(text).selectable(false));
                            });
                        }
                        let response = row.response();
                        if response.clicked() {
                            self.selected = Some(index);
                        }
                        if response.double_clicked() {
                            self.selected = Some(index);
                            open_requested = true;
                        }
                    });
                });
            if open_requested {
                self.open_selected();
            }
        });
    }
}

/// Binds the discovery port with address reuse so other WLED tools can listen too.
fn bind_discovery_socket() -> std::io::Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    // macOS/BSD need SO_REUSEPORT for several sockets to share broadcast traffic.
    #[cfg(all(unix, not(any(target_os = "solaris", target_os = "illumos"))))]
    socket.set_reuse_port(true)?;
    socket.bind(&SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, DISCOVERY_PORT).into())?;
    Ok(socket.into())
}

fn listen(socket: UdpSocket, tx: Sender<Event>, ctx: egui::Context) {
    let mut buf = [0u8; 1500];
    loop {
        let Ok((len, _)) = socket.recv_from(&mut buf) else {
            continue;
        };
        if let Some(ip) = parse_announcement(&buf[..len]) {
            if tx.send(Event::Announced(ip)).is_err() {
                return;
            }
            ctx.request_repaint();
        }
    }
}

/// WLED node-info packet: byte 0 = 255 (token), byte 1 = 1 (node info), bytes 2..6 = IPv4 address.
fn parse_announcement(packet: &[u8]) -> Option<Ipv4Addr> {
    match packet {
        [255, 1, a, b, c, d, ..] => Some(Ipv4Addr::new(*a, *b, *c, *d)),
        _ => None,
    }
}

fn fetch_info(ip: Ipv4Addr) -> Option<WledInfo> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(HTTP_TIMEOUT))
        .build()
        .into();
    let body = agent
        .get(&format!("http://{ip}/json/info"))
        .call()
        .ok()?
        .body_mut()
        .read_to_string()
        .ok()?;
    serde_json::from_str(&body).ok()
}

fn format_uptime(seconds: u64) -> String {
    let (days, rest) = (seconds / 86_400, seconds % 86_400);
    format!(
        "{:02}:{:02}:{:02}:{:02}",
        days,
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("WLED Devices")
            // Wayland takes the icon from the .desktop file whose name matches the app id.
            .with_app_id("wled-scanner")
            .with_inner_size([720.0, 320.0])
            .with_min_inner_size([480.0, 200.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
                    .expect("embedded icon is a valid PNG"),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "WLED Devices",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_node_info_packet() {
        assert_eq!(
            parse_announcement(&[255, 1, 192, 168, 1, 42, 0, 0]),
            Some(Ipv4Addr::new(192, 168, 1, 42))
        );
        assert_eq!(parse_announcement(&[255, 1, 192, 168]), None);
        assert_eq!(parse_announcement(&[0, 1, 192, 168, 1, 42]), None);
    }

    #[test]
    fn formats_uptime() {
        assert_eq!(format_uptime(0), "00:00:00:00");
        assert_eq!(format_uptime(90_061), "01:01:01:01");
    }
}
