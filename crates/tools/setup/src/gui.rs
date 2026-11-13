//! The setup wizard window: the pages of [`crate::wizard`] drawn with egui, the
//! same on Windows, Linux and macOS.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use eframe::egui::epaint::{Mesh, Shadow};
use eframe::egui::{
    self, Align, Align2, Color32, Context, CornerRadius, FontFamily, FontId, Layout, Margin, Rect, RichText, Sense, Stroke, StrokeKind, Ui, UiBuilder,
    Vec2, ViewportCommand, pos2, vec2,
};

use crate::engine::{self, Bundle, Event, Options, Progress, Step};
use crate::platform;
use crate::text::{Key, fill};
use crate::wizard::{Mode, Page, Stop, ThisMachine, Wizard};

const LICENSE: &str = include_str!("../../../../LICENSE");
const ICON_PNG: &[u8] = include_bytes!("../../../../assets/icons/linux/QBNex.png");
/// With this environment variable set the wizard clicks its own way through every
/// page and closes: the installer's test of the real window. When its value names a
/// folder, a picture of every page is saved there first (`page-1.png`, ...).
pub const SELF_DRIVE: &str = "QBNEX_SETUP_SELF_DRIVE";

const WINDOW: Vec2 = vec2(660.0, 480.0);
const FOOTER: f32 = 62.0;
const HEADER: f32 = 68.0;
const BAND: f32 = 208.0;

const NAVY: Color32 = Color32::from_rgb(13, 34, 71);
const BLUE: Color32 = Color32::from_rgb(28, 100, 206);
const BLUE_HOT: Color32 = Color32::from_rgb(45, 122, 232);
const BLUE_DEEP: Color32 = Color32::from_rgb(20, 78, 168);
const SKY: Color32 = Color32::from_rgb(120, 184, 255);
const INK: Color32 = Color32::from_rgb(26, 34, 50);
const MUTED: Color32 = Color32::from_rgb(98, 110, 132);
const LINE: Color32 = Color32::from_rgb(212, 220, 232);
const PAPER_TOP: Color32 = Color32::from_rgb(247, 249, 253);
const PAPER_BOTTOM: Color32 = Color32::from_rgb(226, 233, 245);
const INSET: Color32 = Color32::from_rgb(244, 247, 252);
const DANGER: Color32 = Color32::from_rgb(196, 43, 43);
const GREEN: Color32 = Color32::from_rgb(34, 160, 92);

/// What the window is opened for.
pub struct Start {
    pub mode: Mode,
    pub options: Options,
    /// None for an uninstall, or when this program carries no files to install.
    pub bundle: Option<Bundle>,
}

/// How the wizard ended; set when the window closes.
static OUTCOME: Mutex<Option<Result<(), String>>> = Mutex::new(None);

/// What the working thread tells the window.
enum Work {
    Event(Event),
    Done(Result<(), String>),
}

/// A question shown over the page.
#[derive(Clone, Debug, PartialEq)]
enum Asking {
    /// Leave Setup?
    Exit,
    /// Use a folder that holds other files?
    Folder(String),
}

/// A wizard that clicks itself through its pages.
struct Drive {
    /// Where the pictures of the pages go, and how many were saved.
    shots: Option<(PathBuf, u32)>,
    /// Frames to let the page settle before the next click.
    wait: u32,
    /// A picture was asked for and is not saved yet.
    picture_pending: Arc<AtomicBool>,
    /// The picture of the page being shown is saved.
    pictured: bool,
}

struct App {
    wizard: Wizard,
    bundle: Option<Arc<Bundle>>,
    asking: Option<Asking>,
    /// What is wrong with the page, shown on it.
    problem: Option<String>,
    step: Key,
    file: String,
    fraction: f32,
    cancel: Arc<AtomicBool>,
    work: Option<Receiver<Work>>,
    open_readme: bool,
    open_folder: bool,
    logo: Option<egui::TextureHandle>,
    bold: FontFamily,
    drive: Option<Drive>,
    closing: bool,
}

/// The icon as pixels: (width, height, RGBA).
fn icon_pixels(size: u32) -> Option<(u32, u32, Vec<u8>)> {
    let decoded = image::load_from_memory_with_format(ICON_PNG, image::ImageFormat::Png).ok()?;
    let small = decoded.resize(size, size, image::imageops::FilterType::Lanczos3).into_rgba8();
    Some((small.width(), small.height(), small.into_raw()))
}

/// Shows the wizard and returns how the install or uninstall ended. The outer
/// error means no window could be opened.
pub fn run(start: Start) -> Result<Result<(), String>, String> {
    let title = if start.mode == Mode::Install { Key::WindowTitle } else { Key::UninstallTitle }.text();
    let mut viewport = egui::ViewportBuilder::default().with_title(title).with_inner_size(WINDOW).with_resizable(false).with_maximize_button(false);
    if let Some((width, height, rgba)) = icon_pixels(64) {
        viewport = viewport.with_icon(egui::IconData { rgba, width, height });
    }
    let options = eframe::NativeOptions { viewport, centered: true, persist_window: false, ..eframe::NativeOptions::default() };
    eframe::run_native("qbnex-setup", options, Box::new(move |cc| Ok(Box::new(App::new(start, &cc.egui_ctx)))))
        .map_err(|e| e.to_string())?;
    let outcome = OUTCOME.lock().ok().and_then(|mut o| o.take());
    Ok(outcome.unwrap_or_else(|| Err(engine::CANCELLED.into())))
}

/// Sends the engine's events to the window and carries Cancel the other way.
struct Channel {
    sender: Sender<Work>,
    cancel: Arc<AtomicBool>,
    ctx: Context,
}

impl Progress for Channel {
    fn event(&mut self, event: Event) {
        let _ = self.sender.send(Work::Event(event));
        self.ctx.request_repaint();
    }

    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

/// The system's own interface font, regular and bold, where it is found: text then
/// looks like the rest of the desktop. Without it egui's built-in font is used.
fn system_fonts() -> Option<(Vec<u8>, Vec<u8>)> {
    let pairs: &[(&str, &str)] = if cfg!(windows) {
        &[(r"C:\Windows\Fonts\segoeui.ttf", r"C:\Windows\Fonts\segoeuib.ttf")]
    } else if cfg!(target_os = "macos") {
        &[("/System/Library/Fonts/Supplemental/Arial.ttf", "/System/Library/Fonts/Supplemental/Arial Bold.ttf")]
    } else {
        &[
            ("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"),
            ("/usr/share/fonts/dejavu/DejaVuSans.ttf", "/usr/share/fonts/dejavu/DejaVuSans-Bold.ttf"),
            ("/usr/share/fonts/TTF/DejaVuSans.ttf", "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf"),
            ("/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf", "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf"),
        ]
    };
    pairs.iter().find_map(|(regular, bold)| Some((std::fs::read(regular).ok()?, std::fs::read(bold).ok()?)))
}

/// Sets fonts and colours; returns the family headings are written in.
fn set_style(ctx: &Context) -> FontFamily {
    let mut fonts = egui::FontDefinitions::default();
    let mut bold = FontFamily::Proportional;
    if let Some((regular, heavy)) = system_fonts() {
        fonts.font_data.insert("system".into(), Arc::new(egui::FontData::from_owned(regular)));
        fonts.font_data.insert("system-bold".into(), Arc::new(egui::FontData::from_owned(heavy)));
        let builtin = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
        let mut regular_first = vec!["system".to_string()];
        let mut bold_first = vec!["system-bold".to_string()];
        regular_first.extend(builtin.iter().cloned());
        bold_first.extend(builtin);
        fonts.families.insert(FontFamily::Proportional, regular_first);
        bold = FontFamily::Name("bold".into());
        fonts.families.insert(bold.clone(), bold_first);
    }
    ctx.set_fonts(fonts);
    ctx.set_theme(egui::Theme::Light);
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = vec2(8.0, 9.0);
        style.spacing.interact_size.y = 24.0;
        style.spacing.icon_width = 18.0;
        style.text_styles.insert(egui::TextStyle::Body, FontId::proportional(14.5));
        style.text_styles.insert(egui::TextStyle::Button, FontId::proportional(14.5));
        style.text_styles.insert(egui::TextStyle::Small, FontId::proportional(12.5));
        style.text_styles.insert(egui::TextStyle::Monospace, FontId::monospace(12.0));
        let v = &mut style.visuals;
        v.override_text_color = Some(INK);
        v.selection.bg_fill = Color32::from_rgb(170, 208, 255);
        v.selection.stroke = Stroke::new(1.0, BLUE);
        v.extreme_bg_color = Color32::WHITE;
        for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.noninteractive] {
            w.corner_radius = CornerRadius::same(5);
        }
        v.widgets.inactive.bg_fill = Color32::from_rgb(232, 238, 247);
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(170, 182, 200));
        v.widgets.hovered.bg_fill = Color32::from_rgb(216, 229, 248);
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, BLUE);
        v.widgets.active.bg_fill = Color32::from_rgb(196, 216, 246);
        v.widgets.active.bg_stroke = Stroke::new(1.0, BLUE_DEEP);
    });
    bold
}

/// A rectangle filled with a blend of the colours given for its four corners.
fn gradient(ui: &Ui, rect: Rect, top_left: Color32, top_right: Color32, bottom_left: Color32, bottom_right: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), top_left);
    mesh.colored_vertex(rect.right_top(), top_right);
    mesh.colored_vertex(rect.left_bottom(), bottom_left);
    mesh.colored_vertex(rect.right_bottom(), bottom_right);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    ui.painter().add(mesh);
}

/// A rectangle whose colour runs from `top` to `bottom`.
fn fade(ui: &Ui, rect: Rect, top: Color32, bottom: Color32) {
    gradient(ui, rect, top, top, bottom, bottom);
}

fn shadow(ui: &Ui, rect: Rect, radius: u8, blur: u8, drop: i8, alpha: u8) {
    let shape = Shadow { offset: [0, drop], blur, spread: 0, color: Color32::from_black_alpha(alpha) };
    ui.painter().add(shape.as_shape(rect, CornerRadius::same(radius)));
}

/// A raised white card; returns the area inside its padding.
fn card(ui: &Ui, rect: Rect) -> Rect {
    shadow(ui, rect, 12, 18, 5, 42);
    ui.painter().rect_filled(rect, 12, Color32::WHITE);
    ui.painter().rect_stroke(rect, 12, Stroke::new(1.0, Color32::from_rgb(222, 229, 240)), StrokeKind::Inside);
    rect.shrink2(vec2(22.0, 18.0))
}

/// Lays `contents` out top down inside `rect`.
fn area(ui: &mut Ui, rect: Rect, contents: impl FnOnce(&mut Ui)) {
    let mut child = ui.new_child(UiBuilder::new().max_rect(rect).layout(Layout::top_down(Align::LEFT)));
    child.set_clip_rect(rect.expand(2.0));
    contents(&mut child);
}

/// A recessed panel for text that scrolls or lists.
fn inset() -> egui::Frame {
    egui::Frame::new().fill(INSET).stroke(Stroke::new(1.0, LINE)).corner_radius(8).inner_margin(Margin::same(10))
}

impl App {
    fn new(start: Start, ctx: &Context) -> App {
        let bold = set_style(ctx);
        let required = start.bundle.as_ref().map_or(0, Bundle::installed_size);
        let mut wizard = Wizard::new(start.mode, &start.options, required, platform::can_associate());
        let mut problem = None;
        if start.mode == Mode::Install && start.bundle.is_none() {
            // nothing to install: the only page says so
            problem = Some(Key::NoBundle.text().to_string());
            wizard.finish(Err(Key::NoBundle.text().to_string()));
        }
        let drive = std::env::var_os(SELF_DRIVE).map(|value| {
            let folder = PathBuf::from(value);
            Drive { shots: folder.is_dir().then_some((folder, 0)), wait: 0, picture_pending: Arc::new(AtomicBool::new(false)), pictured: false }
        });
        let logo = icon_pixels(256).map(|(w, h, rgba)| {
            let pixels = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
            ctx.load_texture("logo", pixels, egui::TextureOptions::LINEAR)
        });
        App {
            wizard,
            bundle: start.bundle.map(Arc::new),
            asking: None,
            problem,
            step: Key::StepPrepare,
            file: String::new(),
            fraction: 0.0,
            cancel: Arc::new(AtomicBool::new(false)),
            work: None,
            open_readme: false,
            open_folder: false,
            logo,
            bold,
            drive,
            closing: false,
        }
    }

    fn heading(&self, text: impl Into<String>, size: f32) -> RichText {
        RichText::new(text).font(FontId::new(size, self.bold.clone())).color(INK)
    }

    /// Runs the install or uninstall on a thread of its own; its events come back through a channel.
    fn start_work(&mut self, ctx: &Context) {
        let (sender, receiver) = mpsc::channel();
        let mut channel = Channel { sender: sender.clone(), cancel: self.cancel.clone(), ctx: ctx.clone() };
        let options = self.wizard.options();
        let bundle = self.bundle.clone();
        let mode = self.wizard.mode;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = match (mode, bundle) {
                (Mode::Install, Some(bundle)) => engine::install(&options, &bundle, &mut channel),
                (Mode::Install, None) => Err("nothing to install".into()),
                (Mode::Uninstall, _) => engine::uninstall(&options.dir, &mut channel),
            };
            let _ = sender.send(Work::Done(result));
            ctx.request_repaint();
        });
        self.work = Some(receiver);
    }

    /// Takes in what the working thread reported since the last frame.
    fn read_work(&mut self) {
        let Some(receiver) = &self.work else {
            return;
        };
        let reports: Vec<Work> = receiver.try_iter().collect();
        for report in reports {
            match report {
                Work::Event(Event::Step(step)) => {
                    if !self.cancel.load(Ordering::Relaxed) {
                        self.step = match step {
                            Step::Prepare => Key::StepPrepare,
                            Step::Extract => Key::StepExtract,
                            Step::Register => Key::StepRegister,
                            Step::Remove => Key::StepRemove,
                        };
                    }
                    if step == Step::Register {
                        self.file.clear();
                        self.fraction = 1.0;
                    }
                }
                Work::Event(Event::File { name, done, total }) => {
                    self.file = name;
                    self.fraction = if total == 0 { 0.0 } else { done as f32 / total as f32 };
                }
                Work::Done(result) => {
                    self.asking = None;
                    self.problem = result.as_ref().err().filter(|e| e.as_str() != engine::CANCELLED).cloned();
                    self.wizard.finish(result);
                    self.work = None;
                }
            }
        }
    }

    /// Ends the wizard: the window closes and [`run`] returns the outcome.
    fn close(&mut self, ctx: &Context) {
        if self.closing {
            return;
        }
        self.closing = true;
        let done = self.wizard.page == Page::Finished && self.wizard.outcome == Some(Ok(()));
        let dir = self.wizard.folder();
        if done && self.wizard.mode == Mode::Install {
            if self.open_readme {
                platform::open(&dir.join("README.md"));
            }
            if self.open_folder {
                platform::open(&dir);
            }
        }
        if done && self.wizard.mode == Mode::Uninstall {
            // only now: the uninstaller deletes itself as the window closes
            platform::remove_install_dir(&dir);
        }
        if let Ok(mut outcome) = OUTCOME.lock() {
            *outcome = Some(self.wizard.outcome.clone().unwrap_or_else(|| Err(engine::CANCELLED.into())));
        }
        ctx.send_viewport_cmd(ViewportCommand::Close);
    }

    fn next(&mut self, ctx: &Context) {
        if self.wizard.page == Page::Finished {
            return self.close(ctx);
        }
        match self.wizard.next(&ThisMachine) {
            Ok(()) => self.problem = None,
            Err(Stop::Problem(text)) => self.problem = Some(text),
            Err(Stop::Question(text)) => self.asking = Some(Asking::Folder(text)),
        }
        if self.wizard.page == Page::Working && self.work.is_none() && self.wizard.outcome.is_none() {
            self.step = if self.wizard.mode == Mode::Install { Key::StepPrepare } else { Key::StepRemove };
            self.start_work(ctx);
        }
    }

    fn cancel(&mut self, ctx: &Context) {
        if self.wizard.page == Page::Finished {
            self.close(ctx);
        } else if self.wizard.can_cancel() && self.asking.is_none() {
            self.asking = Some(Asking::Exit);
        }
    }

    fn answer(&mut self, yes: bool, ctx: &Context) {
        match (self.asking.take(), yes) {
            (Some(Asking::Exit), true) if self.wizard.page == Page::Working => {
                // the engine notices between two files and puts everything back
                self.cancel.store(true, Ordering::Relaxed);
                self.step = Key::Cancelling;
            }
            (Some(Asking::Exit), true) => self.close(ctx),
            (Some(Asking::Folder(_)), true) => {
                self.wizard.confirm();
                self.next(ctx);
            }
            _ => {}
        }
    }

    /// One step of a wizard that drives itself: picture the page, then accept,
    /// answer yes or press Next.
    fn self_drive(&mut self, ctx: &Context) {
        let Some(drive) = &mut self.drive else {
            return;
        };
        ctx.request_repaint();
        if drive.picture_pending.load(Ordering::Relaxed) {
            return;
        }
        drive.wait += 1;
        if drive.wait < 6 {
            return;
        }
        let working = self.wizard.page == Page::Working && self.asking.is_none();
        if let (Some((folder, count)), false, false) = (&mut drive.shots, drive.pictured, working) {
            *count += 1;
            let path = folder.join(format!("page-{count}.png"));
            let pending = drive.picture_pending.clone();
            pending.store(true, Ordering::Relaxed);
            drive.pictured = true;
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(egui::ScreenshotCallback::new(move |picture: Arc<egui::ColorImage>| {
                let [width, height] = picture.size;
                let _ = image::save_buffer(path, picture.as_raw(), width as u32, height as u32, image::ColorType::Rgba8);
                pending.store(false, Ordering::Relaxed);
            })));
            return;
        }
        drive.wait = 0;
        drive.pictured = false;
        if self.asking.is_some() {
            return self.answer(true, ctx);
        }
        if self.problem.is_some() && self.wizard.page != Page::Finished {
            // a page that cannot be passed ends the drive with its message
            self.wizard.finish(Err(self.problem.clone().unwrap_or_default()));
            return self.close(ctx);
        }
        match self.wizard.page {
            Page::Working => {}
            Page::Finished => self.close(ctx),
            Page::License if !self.wizard.accepted => self.wizard.accepted = true,
            _ => self.next(ctx),
        }
    }

    /// A push button drawn with depth: shadow, sheen and a pressed state.
    fn button(&self, ui: &mut Ui, rect: Rect, label: &str, primary: bool, enabled: bool) -> bool {
        let response = ui.allocate_rect(rect, if enabled { Sense::click() } else { Sense::hover() });
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, label));
        let pressed = enabled && response.is_pointer_button_down_on();
        let hot = enabled && response.hovered();
        let (fill, edge, ink) = match (primary, enabled) {
            (true, true) => (if pressed { BLUE_DEEP } else if hot { BLUE_HOT } else { BLUE }, BLUE_DEEP, Color32::WHITE),
            (true, false) => (Color32::from_rgb(166, 196, 236), Color32::from_rgb(150, 182, 226), Color32::WHITE),
            (false, true) => {
                let face = if pressed { Color32::from_rgb(222, 229, 240) } else if hot { Color32::from_rgb(240, 246, 255) } else { Color32::from_rgb(250, 251, 253) };
                (face, Color32::from_rgb(176, 188, 206), INK)
            }
            (false, false) => (Color32::from_rgb(240, 243, 248), LINE, Color32::from_rgb(160, 170, 186)),
        };
        let face = if pressed { rect.translate(vec2(0.0, 1.0)) } else { rect };
        if enabled && !pressed {
            shadow(ui, face, 7, if hot { 12 } else { 8 }, if hot { 4 } else { 2 }, if primary { 70 } else { 36 });
        }
        let painter = ui.painter();
        painter.rect_filled(face, 7, fill);
        let sheen = Rect::from_min_max(face.min, pos2(face.max.x, face.center().y));
        painter.rect_filled(sheen, CornerRadius { nw: 7, ne: 7, sw: 0, se: 0 }, Color32::from_white_alpha(if primary { 26 } else { 150 }));
        painter.rect_stroke(face, 7, Stroke::new(1.0, edge), StrokeKind::Inside);
        if response.has_focus() {
            painter.rect_stroke(face.expand(2.5), 9, Stroke::new(1.5, SKY), StrokeKind::Outside);
        }
        let family = if primary { self.bold.clone() } else { FontFamily::Proportional };
        painter.text(face.center(), Align2::CENTER_CENTER, label, FontId::new(14.5, family), ink);
        response.clicked()
    }

    fn logo(&self, ui: &Ui, rect: Rect) {
        if let Some(logo) = &self.logo {
            ui.painter().image(logo.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }
    }

    /// The page with the coloured band on the left: the first and the last.
    fn banner_page(&mut self, ui: &mut Ui, top: Rect, page: fn(&mut App, &mut Ui)) {
        let band = Rect::from_min_size(top.min, vec2(BAND, top.height()));
        let rest = Rect::from_min_max(pos2(band.max.x, top.min.y), top.max);
        shadow(ui, band, 0, 22, 0, 90);
        gradient(ui, band, NAVY, Color32::from_rgb(18, 52, 110), Color32::from_rgb(22, 74, 158), BLUE);
        let glow = ui.painter().with_clip_rect(band);
        glow.circle_filled(band.left_bottom() + vec2(20.0, -10.0), 150.0, Color32::from_white_alpha(9));
        glow.circle_filled(band.right_top() + vec2(-10.0, 30.0), 110.0, Color32::from_white_alpha(8));
        glow.circle_filled(band.left_bottom() + vec2(60.0, 30.0), 90.0, Color32::from_white_alpha(10));
        // the logo on a glass plate
        let plate = Rect::from_center_size(pos2(band.center().x, band.min.y + 118.0), vec2(136.0, 136.0));
        shadow(ui, plate, 22, 26, 8, 110);
        ui.painter().rect_filled(plate, 22, Color32::from_white_alpha(28));
        ui.painter().rect_stroke(plate, 22, Stroke::new(1.0, Color32::from_white_alpha(70)), StrokeKind::Inside);
        self.logo(ui, plate.shrink(14.0));
        let name_at = pos2(band.center().x, plate.max.y + 34.0);
        ui.painter().text(name_at + vec2(0.0, 2.0), Align2::CENTER_CENTER, "QBNex", FontId::new(30.0, self.bold.clone()), Color32::from_black_alpha(90));
        ui.painter().text(name_at, Align2::CENTER_CENTER, "QBNex", FontId::new(30.0, self.bold.clone()), Color32::WHITE);
        let pill = Rect::from_center_size(name_at + vec2(0.0, 36.0), vec2(86.0, 24.0));
        ui.painter().rect_filled(pill, 12, Color32::from_white_alpha(34));
        ui.painter().text(pill.center(), Align2::CENTER_CENTER, format!("v{}", engine::VERSION), FontId::proportional(13.0), Color32::WHITE);
        ui.painter().text(pos2(band.center().x, band.max.y - 20.0), Align2::CENTER_CENTER, "BASIC compiler", FontId::proportional(12.0), Color32::from_white_alpha(150));

        let inner = card(ui, rest.shrink2(vec2(20.0, 22.0)));
        area(ui, inner, |ui| page(self, ui));
    }

    /// A page with the white strip on top that names the step.
    fn header_page(&mut self, ui: &mut Ui, top: Rect, title: Key, sub: Key, page: fn(&mut App, &mut Ui)) {
        let head = Rect::from_min_size(top.min, vec2(top.width(), HEADER));
        shadow(ui, head, 0, 14, 3, 40);
        fade(ui, head, Color32::WHITE, Color32::from_rgb(244, 247, 252));
        let rule = Rect::from_min_size(head.left_bottom() - vec2(0.0, 3.0), vec2(head.width(), 3.0));
        gradient(ui, rule, BLUE_DEEP, SKY, BLUE_DEEP, SKY);
        ui.painter().text(head.min + vec2(24.0, 13.0), Align2::LEFT_TOP, title.text(), FontId::new(18.0, self.bold.clone()), INK);
        ui.painter().text(head.min + vec2(24.0, 38.0), Align2::LEFT_TOP, sub.text(), FontId::proportional(13.5), MUTED);
        self.logo(ui, Rect::from_center_size(pos2(head.max.x - 42.0, head.center().y - 2.0), vec2(46.0, 46.0)));

        let body = Rect::from_min_max(pos2(top.min.x, head.max.y), top.max);
        let inner = card(ui, body.shrink2(vec2(22.0, 18.0)));
        area(ui, inner, |ui| page(self, ui));
    }

    fn problem_line(&self, ui: &mut Ui) {
        if let Some(problem) = &self.problem {
            ui.label(RichText::new(problem).color(DANGER));
        }
    }

    fn welcome(&mut self, ui: &mut Ui) {
        ui.add_space(6.0);
        ui.label(self.heading(fill(Key::WelcomeTitle.text(), engine::VERSION), 22.0));
        ui.add_space(6.0);
        ui.label(RichText::new(Key::WelcomeBody.text()).color(MUTED));
    }

    fn license(&mut self, ui: &mut Ui) {
        let text_height = ui.available_height() - 118.0;
        inset().show(ui, |ui| {
            egui::ScrollArea::vertical().max_height(text_height).auto_shrink([false, false]).show(ui, |ui| {
                ui.label(RichText::new(LICENSE).monospace().color(INK));
            });
        });
        ui.label(RichText::new(Key::LicenseNote.text()).small().color(MUTED));
        ui.radio_value(&mut self.wizard.accepted, true, Key::Accept.text());
        ui.radio_value(&mut self.wizard.accepted, false, Key::Decline.text());
    }

    fn folder(&mut self, ui: &mut Ui) {
        ui.label(RichText::new(Key::FolderBody.text()).color(MUTED));
        ui.add_space(4.0);
        let row = ui.available_width();
        ui.horizontal(|ui| {
            let edit = egui::TextEdit::singleline(&mut self.wizard.dir).desired_width(row - 138.0).margin(Margin::symmetric(8, 7));
            if ui.add(edit).changed() {
                self.problem = None;
            }
            let (rect, _) = ui.allocate_exact_size(vec2(104.0, 32.0), Sense::hover());
            if self.button(ui, rect, Key::Browse.text(), false, true) {
                let mut dialog = rfd::FileDialog::new().set_title(Key::FolderTitle.text());
                if let Some(existing) = self.wizard.folder().ancestors().find(|p| p.is_dir()) {
                    dialog = dialog.set_directory(existing);
                }
                if let Some(picked) = dialog.pick_folder() {
                    self.problem = None;
                    self.wizard.dir = install_folder_in(picked).to_string_lossy().into_owned();
                }
            }
        });
        self.problem_line(ui);
        let notes = self.wizard.folder_notes(&ThisMachine);
        ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
            ui.horizontal_wrapped(|ui| {
                for note in &notes {
                    egui::Frame::new().fill(Color32::from_rgb(232, 240, 252)).corner_radius(11).inner_margin(Margin::symmetric(10, 4)).show(ui, |ui| {
                        ui.label(RichText::new(note).small().color(BLUE_DEEP));
                    });
                }
            });
        });
    }

    fn tasks(&mut self, ui: &mut Ui) {
        let w = &mut self.wizard;
        let mut tasks = vec![(&mut w.add_path, Key::TaskPath), (&mut w.menu, Key::TaskMenu), (&mut w.desktop, Key::TaskDesktop)];
        if w.can_associate {
            tasks.push((&mut w.associate, Key::TaskAssociate));
        }
        for (on, key) in tasks {
            let fill = if *on { Color32::from_rgb(234, 242, 254) } else { INSET };
            let edge = if *on { Color32::from_rgb(150, 190, 245) } else { LINE };
            egui::Frame::new().fill(fill).stroke(Stroke::new(1.0, edge)).corner_radius(8).inner_margin(Margin::symmetric(12, 9)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.checkbox(on, key.text());
            });
        }
    }

    fn ready(&mut self, ui: &mut Ui) {
        ui.label(RichText::new(Key::ReadyBody.text()).color(MUTED));
        ui.add_space(2.0);
        let bold = self.bold.clone();
        let folder = self.wizard.folder().display().to_string();
        let tasks = self.wizard.chosen_tasks();
        inset().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(ui.available_height());
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.label(RichText::new(Key::ReadyFolder.text()).font(FontId::new(14.5, bold.clone())));
                ui.label(format!("      {folder}"));
                ui.add_space(4.0);
                ui.label(RichText::new(Key::ReadyTasks.text()).font(FontId::new(14.5, bold.clone())));
                if tasks.is_empty() {
                    ui.label(format!("      {}", Key::ReadyNoTasks.text()));
                }
                for task in tasks {
                    ui.label(format!("      \u{2022}  {task}"));
                }
            });
        });
    }

    fn remove(&mut self, ui: &mut Ui) {
        ui.add_space(6.0);
        ui.label(self.heading(Key::RemoveTitle.text(), 22.0));
        ui.add_space(6.0);
        ui.label(RichText::new(Key::RemoveBody.text()).color(MUTED));
        ui.add_space(4.0);
        inset().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(self.wizard.folder().display().to_string()).monospace());
        });
    }

    fn working(&mut self, ui: &mut Ui) {
        ui.add_space(10.0);
        ui.label(self.heading(self.step.text(), 15.5));
        ui.add_space(6.0);
        let (track, _) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(track, 11, Color32::from_rgb(222, 229, 240));
        painter.rect_stroke(track, 11, Stroke::new(1.0, Color32::from_rgb(196, 206, 222)), StrokeKind::Inside);
        let fraction = self.fraction.clamp(0.0, 1.0);
        if fraction > 0.0 {
            let bar = Rect::from_min_size(track.min, vec2((track.width() * fraction).max(22.0), track.height())).shrink(2.0);
            painter.rect_filled(bar, 9, BLUE);
            painter.rect_filled(Rect::from_min_max(bar.min, pos2(bar.max.x, bar.center().y)), CornerRadius { nw: 9, ne: 9, sw: 0, se: 0 }, Color32::from_white_alpha(60));
        }
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{:.0}%", fraction * 100.0)).color(BLUE_DEEP).font(FontId::new(13.5, self.bold.clone())));
            ui.add(egui::Label::new(RichText::new(&self.file).small().color(MUTED)).truncate());
        });
    }

    /// A round badge with a tick or a cross, then the title beside it.
    fn verdict(&self, ui: &mut Ui, good: bool, title: Key) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let (badge, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
            let colour = if good { GREEN } else { DANGER };
            shadow(ui, badge, 20, 10, 3, 60);
            let painter = ui.painter();
            painter.circle_filled(badge.center(), 20.0, colour);
            painter.circle_filled(badge.center() - vec2(0.0, 6.0), 13.0, Color32::from_white_alpha(28));
            let c = badge.center();
            let pen = Stroke::new(3.2, Color32::WHITE);
            if good {
                painter.line_segment([c + vec2(-9.0, 0.5), c + vec2(-3.0, 6.5)], pen);
                painter.line_segment([c + vec2(-3.0, 6.5), c + vec2(9.0, -6.0)], pen);
            } else {
                painter.line_segment([c + vec2(-7.0, -7.0), c + vec2(7.0, 7.0)], pen);
                painter.line_segment([c + vec2(-7.0, 7.0), c + vec2(7.0, -7.0)], pen);
            }
            ui.add_space(4.0);
            ui.label(self.heading(title.text(), 19.0));
        });
        ui.add_space(6.0);
    }

    fn finished(&mut self, ui: &mut Ui) {
        let install = self.wizard.mode == Mode::Install;
        match self.wizard.outcome.clone() {
            Some(Ok(())) if install => {
                self.verdict(ui, true, Key::DoneTitle);
                ui.label(fill(Key::DoneBody.text(), engine::VERSION));
                let reach = if self.wizard.add_path {
                    Key::DonePath.text().to_string()
                } else {
                    fill(Key::DoneNoPath.text(), &self.wizard.folder().join(platform::compiler_name()).to_string_lossy())
                };
                inset().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new(reach).monospace());
                });
                ui.add_space(6.0);
                ui.checkbox(&mut self.open_readme, Key::OpenReadme.text());
                ui.checkbox(&mut self.open_folder, Key::OpenFolder.text());
            }
            Some(Ok(())) => {
                self.verdict(ui, true, Key::RemovedTitle);
                ui.label(RichText::new(Key::RemovedBody.text()).color(MUTED));
            }
            _ => {
                self.verdict(ui, false, if install { Key::FailedTitle } else { Key::RemoveFailedTitle });
                if install && self.bundle.is_some() {
                    ui.label(RichText::new(if self.wizard.cancelled() { Key::CancelledBody } else { Key::FailedBody }.text()).color(MUTED));
                }
                self.problem_line(ui);
            }
        }
    }

    /// The strip at the bottom: where the wizard is, and the buttons.
    fn footer(&mut self, ui: &mut Ui, bar: Rect) {
        shadow(ui, bar, 0, 16, -2, 34);
        fade(ui, bar, Color32::WHITE, Color32::from_rgb(240, 244, 250));
        ui.painter().hline(bar.x_range(), bar.min.y, Stroke::new(1.0, LINE));
        let ctx = ui.ctx().clone();
        let w = &self.wizard;
        let free = self.asking.is_none();

        if w.mode == Mode::Install {
            // one dot a page: done, here, to come
            let pages = [Page::Welcome, Page::License, Page::Folder, Page::Tasks, Page::Ready, Page::Working, Page::Finished];
            let here = pages.iter().position(|p| *p == w.page).unwrap_or(0);
            for i in 0..pages.len() {
                let centre = pos2(bar.min.x + 30.0 + i as f32 * 20.0, bar.center().y);
                if i > 0 {
                    let colour = if i <= here { BLUE } else { LINE };
                    ui.painter().line_segment([centre - vec2(14.0, 0.0), centre - vec2(6.0, 0.0)], Stroke::new(2.0, colour));
                }
                if i == here {
                    ui.painter().circle_filled(centre, 8.0, Color32::from_rgb(200, 222, 252));
                    ui.painter().circle_filled(centre, 5.0, BLUE);
                } else {
                    ui.painter().circle_filled(centre, 4.5, if i < here { BLUE } else { Color32::from_rgb(196, 206, 222) });
                }
            }
        }

        let size = vec2(108.0, 34.0);
        let mut right = bar.max.x - 20.0;
        let mut place = |gap: f32| {
            let rect = Rect::from_min_size(pos2(right - size.x, bar.center().y - size.y / 2.0), size);
            right -= size.x + gap;
            rect
        };
        let (page, can_next, can_back, can_cancel, label, install) = (w.page, w.can_next(), w.can_back(), w.can_cancel(), w.next_label(), w.mode == Mode::Install);
        if page != Page::Finished {
            let rect = place(18.0);
            if self.button(ui, rect, Key::Cancel.text(), false, free && can_cancel) {
                self.cancel(&ctx);
            }
        }
        if page != Page::Working {
            let rect = place(8.0);
            if self.button(ui, rect, label.text(), true, free && can_next) {
                self.next(&ctx);
            }
        }
        if install && !matches!(page, Page::Finished | Page::Working) {
            let rect = place(8.0);
            if self.button(ui, rect, Key::Back.text(), false, free && can_back) {
                self.problem = None;
                self.wizard.back();
            }
        }
    }

    /// The question over the dimmed page.
    fn question(&mut self, ui: &mut Ui, whole: Rect) {
        let Some(asking) = self.asking.clone() else {
            return;
        };
        // swallow clicks meant for the page below
        ui.allocate_rect(whole, Sense::click());
        ui.painter().rect_filled(whole, 0, Color32::from_rgba_unmultiplied(12, 24, 48, 120));
        let sentence = match &asking {
            Asking::Exit => Key::ConfirmCancel.text().to_string(),
            Asking::Folder(text) => text.clone(),
        };
        let dialog = Rect::from_center_size(whole.center() - vec2(0.0, 16.0), vec2(430.0, 164.0));
        shadow(ui, dialog, 14, 40, 12, 120);
        ui.painter().rect_filled(dialog, 14, Color32::WHITE);
        ui.painter().rect_filled(Rect::from_min_size(dialog.min, vec2(dialog.width(), 5.0)), CornerRadius { nw: 14, ne: 14, sw: 0, se: 0 }, BLUE);
        area(ui, Rect::from_min_max(dialog.min + vec2(26.0, 28.0), dialog.max - vec2(26.0, 64.0)), |ui| {
            ui.label(RichText::new(sentence).size(15.5));
        });
        let size = vec2(108.0, 34.0);
        let no = Rect::from_min_size(dialog.max - vec2(26.0, 20.0) - size, size);
        let yes = no.translate(vec2(-(size.x + 10.0), 0.0));
        let ctx = ui.ctx().clone();
        if self.button(ui, yes, Key::Yes.text(), true, true) {
            self.answer(true, &ctx);
        } else if self.button(ui, no, Key::No.text(), false, true) {
            self.answer(false, &ctx);
        }
    }

    fn keys(&mut self, ctx: &Context) {
        let (enter, escape) = ctx.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
        if self.asking.is_some() {
            if enter || escape {
                self.answer(enter, ctx);
            }
        } else if enter && self.wizard.can_next() && self.wizard.page != Page::Working {
            self.next(ctx);
        } else if escape {
            self.cancel(ctx);
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.read_work();
        if ctx.input(|i| i.viewport().close_requested()) && !self.closing {
            // the close box is Cancel: it asks first, and an install is undone before the window goes
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            self.cancel(&ctx);
        }
        self.keys(&ctx);

        let whole = ui.max_rect();
        fade(ui, whole, PAPER_TOP, PAPER_BOTTOM);
        let top = Rect::from_min_max(whole.min, pos2(whole.max.x, whole.max.y - FOOTER));
        let bar = Rect::from_min_max(pos2(whole.min.x, top.max.y), whole.max);
        let asking = self.asking.is_some();
        let mut page = ui.new_child(UiBuilder::new().max_rect(whole));
        if asking {
            page.disable();
        }
        match self.wizard.page {
            Page::Welcome => self.banner_page(&mut page, top, App::welcome),
            Page::Remove => self.banner_page(&mut page, top, App::remove),
            Page::Finished => self.banner_page(&mut page, top, App::finished),
            Page::License => self.header_page(&mut page, top, Key::LicenseTitle, Key::LicenseSub, App::license),
            Page::Folder => self.header_page(&mut page, top, Key::FolderTitle, Key::FolderSub, App::folder),
            Page::Tasks => self.header_page(&mut page, top, Key::TasksTitle, Key::TasksSub, App::tasks),
            Page::Ready => self.header_page(&mut page, top, Key::ReadyTitle, Key::ReadySub, App::ready),
            Page::Working if self.wizard.mode == Mode::Install => self.header_page(&mut page, top, Key::InstallingTitle, Key::InstallingSub, App::working),
            Page::Working => self.header_page(&mut page, top, Key::RemoveTitle, Key::RemovingSub, App::working),
        }
        self.footer(&mut page, bar);
        if asking {
            let mut over = ui.new_child(UiBuilder::new().max_rect(whole));
            self.question(&mut over, whole);
        }
        self.self_drive(&ctx);
    }
}

/// The folder to install into when the user picked `picked`: the folder itself when
/// it is an install or is named for QBNex, else a `QBNex` folder inside it.
fn install_folder_in(picked: PathBuf) -> PathBuf {
    let named = picked.file_name().is_some_and(|n| n.to_string_lossy().to_lowercase().contains("qbnex"));
    if named || picked.join(engine::MANIFEST).is_file() { picked } else { picked.join("QBNex") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picked_folder_gets_a_folder_for_qbnex() {
        let base = std::env::temp_dir().join(format!("qbnex-setup-pick-{}", std::process::id()));
        assert_eq!(install_folder_in(base.join("Apps")), base.join("Apps").join("QBNex"));
        assert_eq!(install_folder_in(base.join("my-qbnex")), base.join("my-qbnex"));
        std::fs::create_dir_all(base.join("tools")).unwrap();
        std::fs::write(base.join("tools").join(engine::MANIFEST), "").unwrap();
        assert_eq!(install_folder_in(base.join("tools")), base.join("tools"));
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn the_icon_decodes() {
        let (w, h, rgba) = icon_pixels(64).unwrap();
        assert_eq!((w.max(h), rgba.len()), (64, (w * h * 4) as usize));
    }
}
