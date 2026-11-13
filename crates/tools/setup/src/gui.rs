//! The setup wizard window: the pages of [`crate::wizard`] drawn with egui, the
//! same on Windows, Linux and macOS. Flat and quiet: one accent colour, round
//! corners, the steps listed down the left side. Where the window cannot
//! be opened (no OpenGL, no usable font) the caller asks in the terminal instead.

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

/// What the license page shows: the notice, the exception for compiled programs, then the GPL.
const LICENSE: &str = concat!(
    include_str!("../../../../NOTICE.txt"),
    "\n\n",
    include_str!("../../../../LICENSE-EXCEPTION.txt"),
    "\n\n",
    include_str!("../../../../LICENSE")
);
const ICON_PNG: &[u8] = include_bytes!("../../../../assets/icons/linux/QBNex.png");
/// Who makes QBNex, shown at the foot of the side strip.
const DEVELOPER: &str = "Thirawat Sinlapasomsak";
/// With this environment variable set the wizard clicks its own way through every
/// page and closes: the installer's test of the real window. When its value names a
/// folder, a picture of every page is saved there first (`page-1.png`, ...).
pub const SELF_DRIVE: &str = "QBNEX_SETUP_SELF_DRIVE";

const WINDOW: Vec2 = vec2(720.0, 480.0);
/// Width of the strip that lists the steps.
const SIDE: f32 = 204.0;
/// Height of the row of buttons.
const FOOTER: f32 = 76.0;
const BUTTON: Vec2 = vec2(104.0, 36.0);

const PAPER: Color32 = Color32::WHITE;
/// The blue and the green of the logo, top and bottom of the side strip.
const SIDE_TOP: Color32 = Color32::from_rgb(52, 84, 209);
const SIDE_BOTTOM: Color32 = Color32::from_rgb(28, 163, 118);
const SOFT: Color32 = Color32::from_rgb(248, 250, 252);
const HOVER: Color32 = Color32::from_rgb(241, 245, 249);
const LINE: Color32 = Color32::from_rgb(229, 233, 240);
const INK: Color32 = Color32::from_rgb(15, 23, 42);
const MUTED: Color32 = Color32::from_rgb(100, 116, 139);
const FAINT: Color32 = Color32::from_rgb(160, 172, 190);
const ACCENT: Color32 = Color32::from_rgb(37, 99, 235);
const ACCENT_HOT: Color32 = Color32::from_rgb(29, 78, 216);
const ACCENT_SOFT: Color32 = Color32::from_rgb(239, 244, 255);
const GREEN: Color32 = Color32::from_rgb(22, 163, 74);
const GREEN_SOFT: Color32 = Color32::from_rgb(220, 252, 231);
const DANGER: Color32 = Color32::from_rgb(220, 38, 38);
const DANGER_SOFT: Color32 = Color32::from_rgb(254, 226, 226);

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

/// How much a button stands out.
#[derive(Clone, Copy, PartialEq)]
enum Weight {
    /// The step forward: filled with the accent colour.
    Primary,
    /// An outlined button.
    Plain,
    /// Text only, for the way out.
    Quiet,
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
    /// The first frame was drawn.
    shown: bool,
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
    eframe::run_native("qbnex-setup", options, Box::new(move |cc| Ok(Box::new(App::new(start, &cc.egui_ctx)?))))
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

/// The system's own interface fonts: regular, bold and fixed width. The window is
/// written in them, so it looks like the rest of the desktop and the installer
/// carries no font of its own. None when the system has none of the known ones.
fn system_fonts() -> Option<[Vec<u8>; 3]> {
    let sets: &[[&str; 3]] = if cfg!(windows) {
        &[
            [r"C:\Windows\Fonts\segoeui.ttf", r"C:\Windows\Fonts\seguisb.ttf", r"C:\Windows\Fonts\consola.ttf"],
            [r"C:\Windows\Fonts\segoeui.ttf", r"C:\Windows\Fonts\segoeuib.ttf", r"C:\Windows\Fonts\consola.ttf"],
            [r"C:\Windows\Fonts\arial.ttf", r"C:\Windows\Fonts\arialbd.ttf", r"C:\Windows\Fonts\cour.ttf"],
        ]
    } else if cfg!(target_os = "macos") {
        &[
            ["/System/Library/Fonts/Supplemental/Arial.ttf", "/System/Library/Fonts/Supplemental/Arial Bold.ttf", "/System/Library/Fonts/Supplemental/Courier New.ttf"],
            ["/Library/Fonts/Arial.ttf", "/Library/Fonts/Arial Bold.ttf", "/Library/Fonts/Courier New.ttf"],
        ]
    } else {
        &[
            ["/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"],
            ["/usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf", "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans-Bold.ttf", "/usr/share/fonts/dejavu-sans-mono-fonts/DejaVuSansMono.ttf"],
            ["/usr/share/fonts/dejavu/DejaVuSans.ttf", "/usr/share/fonts/dejavu/DejaVuSans-Bold.ttf", "/usr/share/fonts/dejavu/DejaVuSansMono.ttf"],
            ["/usr/share/fonts/TTF/DejaVuSans.ttf", "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf", "/usr/share/fonts/TTF/DejaVuSansMono.ttf"],
            [
                "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
                "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
                "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
            ],
            [
                "/usr/share/fonts/liberation-sans/LiberationSans-Regular.ttf",
                "/usr/share/fonts/liberation-sans/LiberationSans-Bold.ttf",
                "/usr/share/fonts/liberation-mono/LiberationMono-Regular.ttf",
            ],
            ["/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf", "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf", "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf"],
            ["/usr/share/fonts/noto/NotoSans-Regular.ttf", "/usr/share/fonts/noto/NotoSans-Bold.ttf", "/usr/share/fonts/noto/NotoSansMono-Regular.ttf"],
            ["/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf", "/usr/share/fonts/truetype/ubuntu/Ubuntu-B.ttf", "/usr/share/fonts/truetype/ubuntu/UbuntuMono-R.ttf"],
            ["/usr/share/fonts/truetype/freefont/FreeSans.ttf", "/usr/share/fonts/truetype/freefont/FreeSansBold.ttf", "/usr/share/fonts/truetype/freefont/FreeMono.ttf"],
        ]
    };
    sets.iter().find_map(|[regular, bold, fixed]| Some([std::fs::read(regular).ok()?, std::fs::read(bold).ok()?, std::fs::read(fixed).ok()?]))
}

/// Sets fonts and colours; returns the family headings are written in. An error
/// when the system has no font the window can be written in.
fn set_style(ctx: &Context) -> Result<FontFamily, String> {
    let [regular, heavy, fixed] = system_fonts().ok_or("no usable system font was found")?;
    let mut fonts = egui::FontDefinitions::empty();
    let bold = FontFamily::Name("bold".into());
    for (name, bytes, family) in [("system", regular, FontFamily::Proportional), ("system-bold", heavy, bold.clone()), ("system-fixed", fixed, FontFamily::Monospace)] {
        fonts.font_data.insert(name.into(), Arc::new(egui::FontData::from_owned(bytes)));
        // a sign one font lacks is taken from the regular one
        fonts.families.insert(family, vec![name.to_string(), "system".to_string()]);
    }
    ctx.set_fonts(fonts);
    ctx.set_theme(egui::Theme::Light);
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = vec2(8.0, 10.0);
        style.spacing.interact_size.y = 24.0;
        style.spacing.icon_width = 18.0;
        style.spacing.icon_spacing = 10.0;
        style.text_styles.insert(egui::TextStyle::Body, FontId::proportional(14.0));
        style.text_styles.insert(egui::TextStyle::Button, FontId::proportional(14.0));
        style.text_styles.insert(egui::TextStyle::Small, FontId::proportional(12.5));
        style.text_styles.insert(egui::TextStyle::Monospace, FontId::monospace(12.0));
        let v = &mut style.visuals;
        v.override_text_color = Some(INK);
        v.selection.bg_fill = Color32::from_rgb(191, 219, 254);
        v.selection.stroke = Stroke::new(1.5, ACCENT);
        v.extreme_bg_color = SOFT;
        for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.noninteractive] {
            w.corner_radius = CornerRadius::same(6);
            w.expansion = 0.0;
        }
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
        v.widgets.inactive.bg_fill = PAPER;
        v.widgets.inactive.weak_bg_fill = PAPER;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(203, 213, 225));
        v.widgets.hovered.bg_fill = ACCENT_SOFT;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
        v.widgets.active.bg_fill = ACCENT_SOFT;
        v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT_HOT);
        v.widgets.inactive.fg_stroke = Stroke::new(1.6, ACCENT);
        v.widgets.hovered.fg_stroke = Stroke::new(1.6, ACCENT);
        v.widgets.active.fg_stroke = Stroke::new(1.6, ACCENT_HOT);
    });
    Ok(bold)
}

/// Lays `contents` out top down inside `rect`.
fn area(ui: &mut Ui, rect: Rect, contents: impl FnOnce(&mut Ui)) {
    let mut child = ui.new_child(UiBuilder::new().max_rect(rect).layout(Layout::top_down(Align::LEFT)));
    child.set_clip_rect(rect.expand(2.0));
    contents(&mut child);
}

/// A soft round panel for text that scrolls or lists.
fn panel() -> egui::Frame {
    egui::Frame::new().fill(SOFT).stroke(Stroke::new(1.0, LINE)).corner_radius(12).inner_margin(Margin::same(14))
}

/// A tick drawn with two strokes around `centre`.
fn tick(ui: &Ui, centre: egui::Pos2, size: f32, pen: Stroke) {
    let painter = ui.painter();
    painter.line_segment([centre + vec2(-0.45, 0.02) * size, centre + vec2(-0.12, 0.34) * size], pen);
    painter.line_segment([centre + vec2(-0.12, 0.34) * size, centre + vec2(0.46, -0.3) * size], pen);
}

/// The names of the steps down the side, and which one `page` is.
fn steps(mode: Mode, page: Page) -> (&'static [&'static str], usize) {
    match mode {
        Mode::Install => {
            let order = [Page::Welcome, Page::License, Page::Folder, Page::Tasks, Page::Ready, Page::Working, Page::Finished];
            (&["Welcome", "License", "Location", "Options", "Review", "Install", "Finish"], order.iter().position(|p| *p == page).unwrap_or(0))
        }
        Mode::Uninstall => {
            let order = [Page::Remove, Page::Working, Page::Finished];
            (&["Confirm", "Remove", "Finish"], order.iter().position(|p| *p == page).unwrap_or(0))
        }
    }
}

impl App {
    fn new(start: Start, ctx: &Context) -> Result<App, String> {
        let bold = set_style(ctx)?;
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
        Ok(App {
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
            shown: false,
        })
    }

    fn bold(&self, size: f32) -> FontId {
        FontId::new(size, self.bold.clone())
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

    /// A flat round button.
    fn button(&self, ui: &mut Ui, rect: Rect, label: &str, weight: Weight, enabled: bool) -> bool {
        let response = ui.allocate_rect(rect, if enabled { Sense::click() } else { Sense::hover() });
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, label));
        let pressed = enabled && response.is_pointer_button_down_on();
        let hot = enabled && response.hovered();
        let (fill, edge, ink) = match (weight, enabled) {
            (Weight::Primary, true) => (if hot || pressed { ACCENT_HOT } else { ACCENT }, None, Color32::WHITE),
            (Weight::Primary, false) => (Color32::from_rgb(226, 232, 240), None, FAINT),
            (Weight::Plain, true) => (if pressed { LINE } else if hot { HOVER } else { PAPER }, Some(Color32::from_rgb(214, 221, 232)), INK),
            (Weight::Plain, false) => (PAPER, Some(LINE), FAINT),
            (Weight::Quiet, true) => (if pressed { LINE } else if hot { HOVER } else { Color32::TRANSPARENT }, None, MUTED),
            (Weight::Quiet, false) => (Color32::TRANSPARENT, None, Color32::from_rgb(203, 213, 225)),
        };
        let painter = ui.painter();
        painter.rect_filled(rect, 10, fill);
        if let Some(edge) = edge {
            painter.rect_stroke(rect, 10, Stroke::new(1.0, edge), StrokeKind::Inside);
        }
        if response.has_focus() {
            painter.rect_stroke(rect.expand(2.5), 12, Stroke::new(1.5, Color32::from_rgb(147, 197, 253)), StrokeKind::Outside);
        }
        let font = if weight == Weight::Primary { self.bold(14.0) } else { FontId::proportional(14.0) };
        painter.text(rect.center(), Align2::CENTER_CENTER, label, font, ink);
        response.clicked()
    }

    /// The strip on the left, in the two colours of the logo: the product, the
    /// steps, and who makes it.
    fn side(&self, ui: &Ui, strip: Rect) {
        let mut blend = Mesh::default();
        blend.colored_vertex(strip.left_top(), SIDE_TOP);
        blend.colored_vertex(strip.right_top(), SIDE_TOP);
        blend.colored_vertex(strip.left_bottom(), SIDE_BOTTOM);
        blend.colored_vertex(strip.right_bottom(), SIDE_BOTTOM);
        blend.add_triangle(0, 1, 2);
        blend.add_triangle(1, 2, 3);
        ui.painter().add(blend);
        let painter = ui.painter();
        let white = |alpha: u8| Color32::from_white_alpha(alpha);
        let left = strip.min.x + 26.0;
        // the logo keeps its own colours on a white tile
        let tile = Rect::from_min_size(pos2(left, strip.min.y + 24.0), vec2(44.0, 44.0));
        painter.rect_filled(tile, 12, Color32::WHITE);
        if let Some(logo) = &self.logo {
            painter.image(logo.id(), tile.shrink(5.0), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }
        painter.text(pos2(left + 56.0, strip.min.y + 27.0), Align2::LEFT_TOP, "QBNex", self.bold(17.0), Color32::WHITE);
        painter.text(pos2(left + 56.0, strip.min.y + 49.0), Align2::LEFT_TOP, format!("Version {}", engine::VERSION), FontId::proportional(12.0), white(190));

        let (names, here) = steps(self.wizard.mode, self.wizard.page);
        let failed = self.wizard.page == Page::Finished && self.wizard.outcome != Some(Ok(()));
        // without files to install the wizard opens on its last page: the steps
        // before it never ran, so none of them is ticked
        let skipped = self.wizard.mode == Mode::Install && self.bundle.is_none();
        for (i, name) in names.iter().enumerate() {
            let centre = pos2(left + 10.0, strip.min.y + 118.0 + i as f32 * 36.0);
            if i + 1 < names.len() {
                painter.line_segment([centre + vec2(0.0, 13.0), centre + vec2(0.0, 23.0)], Stroke::new(1.5, white(if i < here && !skipped { 230 } else { 70 })));
            }
            let last_done = i == here && self.wizard.page == Page::Finished && !failed;
            if !skipped && (i < here || last_done) {
                painter.circle_filled(centre, 9.0, Color32::WHITE);
                tick(ui, centre, 9.0, Stroke::new(1.7, SIDE_TOP));
            } else if i == here {
                painter.circle_stroke(centre, 8.2, Stroke::new(1.7, Color32::WHITE));
                painter.circle_filled(centre, 3.6, Color32::WHITE);
            } else {
                painter.circle_stroke(centre, 8.2, Stroke::new(1.4, white(90)));
            }
            let (font, colour) = if i == here { (self.bold(13.5), Color32::WHITE) } else if i < here && !skipped { (FontId::proportional(13.5), white(235)) } else { (FontId::proportional(13.5), white(140)) };
            painter.text(centre + vec2(22.0, 0.0), Align2::LEFT_CENTER, *name, font, colour);
        }

        painter.text(pos2(left, strip.max.y - 46.0), Align2::LEFT_TOP, "Developed by", FontId::proportional(11.5), white(170));
        painter.text(pos2(left, strip.max.y - 30.0), Align2::LEFT_TOP, DEVELOPER, FontId::proportional(12.5), Color32::WHITE);
    }
    /// The title of a page and the line below it.
    fn title(&self, ui: &mut Ui, title: impl Into<String>, sub: Option<Key>) {
        ui.label(RichText::new(title).font(self.bold(23.0)).color(INK));
        if let Some(sub) = sub {
            ui.add_space(-4.0);
            ui.label(RichText::new(sub.text()).color(MUTED));
        }
        ui.add_space(10.0);
    }

    fn problem_line(&self, ui: &mut Ui) {
        if let Some(problem) = &self.problem {
            egui::Frame::new().fill(DANGER_SOFT).corner_radius(10).inner_margin(Margin::symmetric(12, 8)).show(ui, |ui| {
                ui.label(RichText::new(problem).color(Color32::from_rgb(153, 27, 27)));
            });
        }
    }

    fn welcome(&mut self, ui: &mut Ui) {
        ui.add_space(18.0);
        self.title(ui, fill(Key::WelcomeTitle.text(), engine::VERSION), None);
        ui.label(RichText::new(Key::WelcomeBody.text()).color(MUTED));
    }

    fn license(&mut self, ui: &mut Ui) {
        self.title(ui, Key::LicenseTitle.text(), Some(Key::LicenseSub));
        let text_height = ui.available_height() - 150.0;
        panel().show(ui, |ui| {
            egui::ScrollArea::vertical().max_height(text_height).auto_shrink([false, false]).show(ui, |ui| {
                ui.label(RichText::new(LICENSE).monospace().color(Color32::from_rgb(51, 65, 85)));
            });
        });
        ui.label(RichText::new(Key::LicenseNote.text()).small().color(FAINT));
        ui.radio_value(&mut self.wizard.accepted, true, Key::Accept.text());
        ui.radio_value(&mut self.wizard.accepted, false, Key::Decline.text());
    }

    fn folder(&mut self, ui: &mut Ui) {
        self.title(ui, Key::FolderTitle.text(), Some(Key::FolderSub));
        ui.label(RichText::new(Key::FolderBody.text()).color(MUTED));
        ui.add_space(4.0);
        let row = ui.available_width();
        ui.horizontal(|ui| {
            let edit = egui::TextEdit::singleline(&mut self.wizard.dir).desired_width(row - BUTTON.x - 6.0).margin(Margin::symmetric(12, 9));
            ui.scope(|ui| {
                let widgets = &mut ui.visuals_mut().widgets;
                for w in [&mut widgets.inactive, &mut widgets.hovered, &mut widgets.active] {
                    w.corner_radius = CornerRadius::same(10);
                }
                ui.visuals_mut().selection.stroke = Stroke::new(1.5, ACCENT);
                if ui.add(edit).changed() {
                    self.problem = None;
                }
            });
            let (rect, _) = ui.allocate_exact_size(BUTTON, Sense::hover());
            if self.button(ui, rect, Key::Browse.text(), Weight::Plain, true) {
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
                    egui::Frame::new().fill(HOVER).corner_radius(12).inner_margin(Margin::symmetric(11, 4)).show(ui, |ui| {
                        ui.label(RichText::new(note).small().color(MUTED));
                    });
                }
            });
        });
    }

    fn tasks(&mut self, ui: &mut Ui) {
        self.title(ui, Key::TasksTitle.text(), Some(Key::TasksSub));
        let w = &mut self.wizard;
        let mut tasks = vec![(&mut w.add_path, Key::TaskPath), (&mut w.menu, Key::TaskMenu), (&mut w.desktop, Key::TaskDesktop)];
        if w.can_associate {
            tasks.push((&mut w.associate, Key::TaskAssociate));
        }
        for (on, key) in tasks {
            let (fill, edge) = if *on { (ACCENT_SOFT, Color32::from_rgb(191, 213, 255)) } else { (PAPER, LINE) };
            egui::Frame::new().fill(fill).stroke(Stroke::new(1.0, edge)).corner_radius(12).inner_margin(Margin::symmetric(14, 10)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.checkbox(on, key.text());
            });
        }
    }

    fn ready(&mut self, ui: &mut Ui) {
        self.title(ui, Key::ReadyTitle.text(), Some(Key::ReadySub));
        let bold = self.bold(13.0);
        let folder = self.wizard.folder().display().to_string();
        let tasks = self.wizard.chosen_tasks();
        panel().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(ui.available_height() - 8.0);
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.label(RichText::new(Key::ReadyFolder.text().trim_end_matches(':')).font(bold.clone()).color(MUTED));
                ui.add_space(-4.0);
                ui.label(folder);
                ui.add_space(6.0);
                ui.label(RichText::new(Key::ReadyTasks.text().trim_end_matches(':')).font(bold.clone()).color(MUTED));
                ui.add_space(-4.0);
                if tasks.is_empty() {
                    ui.label(Key::ReadyNoTasks.text());
                }
                for task in tasks {
                    ui.horizontal(|ui| {
                        let (dot, _) = ui.allocate_exact_size(vec2(14.0, 18.0), Sense::hover());
                        tick(ui, dot.center(), 7.0, Stroke::new(1.6, ACCENT));
                        ui.label(task);
                    });
                }
            });
        });
    }

    fn remove(&mut self, ui: &mut Ui) {
        ui.add_space(18.0);
        self.title(ui, Key::RemoveTitle.text(), None);
        ui.label(RichText::new(Key::RemoveBody.text()).color(MUTED));
        ui.add_space(6.0);
        panel().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(self.wizard.folder().display().to_string()).monospace());
        });
    }

    fn working(&mut self, ui: &mut Ui) {
        let (title, sub) = if self.wizard.mode == Mode::Install { (Key::InstallingTitle, Key::InstallingSub) } else { (Key::RemoveTitle, Key::RemovingSub) };
        self.title(ui, title.text(), Some(sub));
        ui.add_space(36.0);
        let fraction = self.fraction.clamp(0.0, 1.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(self.step.text()).color(INK));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(format!("{:.0}%", fraction * 100.0)).font(self.bold(14.0)).color(ACCENT));
            });
        });
        let (track, _) = ui.allocate_exact_size(vec2(ui.available_width(), 8.0), Sense::hover());
        let painter = ui.painter();
        painter.rect_filled(track, 4, LINE);
        if fraction > 0.0 {
            painter.rect_filled(Rect::from_min_size(track.min, vec2((track.width() * fraction).max(8.0), track.height())), 4, ACCENT);
        }
        ui.add(egui::Label::new(RichText::new(&self.file).small().color(FAINT)).truncate());
    }

    /// A round badge with a tick or a cross above the title of the last page.
    fn verdict(&self, ui: &mut Ui, good: bool, title: Key) {
        ui.add_space(14.0);
        let (badge, _) = ui.allocate_exact_size(vec2(48.0, 48.0), Sense::hover());
        let (soft, colour) = if good { (GREEN_SOFT, GREEN) } else { (DANGER_SOFT, DANGER) };
        ui.painter().circle_filled(badge.center(), 24.0, soft);
        let pen = Stroke::new(2.6, colour);
        if good {
            tick(ui, badge.center(), 20.0, pen);
        } else {
            let c = badge.center();
            ui.painter().line_segment([c + vec2(-7.0, -7.0), c + vec2(7.0, 7.0)], pen);
            ui.painter().line_segment([c + vec2(-7.0, 7.0), c + vec2(7.0, -7.0)], pen);
        }
        ui.add_space(6.0);
        self.title(ui, title.text(), None);
    }

    fn finished(&mut self, ui: &mut Ui) {
        let install = self.wizard.mode == Mode::Install;
        match self.wizard.outcome.clone() {
            Some(Ok(())) if install => {
                self.verdict(ui, true, Key::DoneTitle);
                ui.add_space(-6.0);
                ui.label(RichText::new(fill(Key::DoneBody.text(), engine::VERSION)).color(MUTED));
                let reach = if self.wizard.add_path {
                    Key::DonePath.text().to_string()
                } else {
                    fill(Key::DoneNoPath.text(), &self.wizard.folder().join(platform::compiler_name()).to_string_lossy())
                };
                ui.add_space(4.0);
                panel().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new(reach).monospace());
                });
                ui.add_space(8.0);
                ui.checkbox(&mut self.open_readme, Key::OpenReadme.text());
                ui.checkbox(&mut self.open_folder, Key::OpenFolder.text());
            }
            Some(Ok(())) => {
                self.verdict(ui, true, Key::RemovedTitle);
                ui.add_space(-6.0);
                ui.label(RichText::new(Key::RemovedBody.text()).color(MUTED));
            }
            _ => {
                self.verdict(ui, false, if install { Key::FailedTitle } else { Key::RemoveFailedTitle });
                ui.add_space(-6.0);
                if install && self.bundle.is_some() {
                    ui.label(RichText::new(if self.wizard.cancelled() { Key::CancelledBody } else { Key::FailedBody }.text()).color(MUTED));
                }
                self.problem_line(ui);
            }
        }
    }

    /// The row of buttons at the bottom right.
    fn buttons(&mut self, ui: &mut Ui, row: Rect) {
        let ctx = ui.ctx().clone();
        let w = &self.wizard;
        let free = self.asking.is_none();
        let mut right = row.max.x;
        let mut place = |gap: f32| {
            let rect = Rect::from_min_size(pos2(right - BUTTON.x, row.center().y - BUTTON.y / 2.0), BUTTON);
            right -= BUTTON.x + gap;
            rect
        };
        let (page, can_next, can_back, can_cancel, label, install) = (w.page, w.can_next(), w.can_back(), w.can_cancel(), w.next_label(), w.mode == Mode::Install);
        if page != Page::Working {
            let rect = place(10.0);
            if self.button(ui, rect, label.text(), Weight::Primary, free && can_next) {
                self.next(&ctx);
            }
        }
        if install && can_back {
            let rect = place(10.0);
            if self.button(ui, rect, Key::Back.text(), Weight::Plain, free) {
                self.problem = None;
                self.wizard.back();
            }
        }
        if page != Page::Finished {
            // the way out sits apart, on the left
            let rect = Rect::from_min_size(pos2(row.min.x - 14.0, row.center().y - BUTTON.y / 2.0), vec2(84.0, BUTTON.y));
            if self.button(ui, rect, Key::Cancel.text(), Weight::Quiet, free && can_cancel) {
                self.cancel(&ctx);
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
        ui.painter().rect_filled(whole, 0, Color32::from_rgba_unmultiplied(15, 23, 42, 96));
        let sentence = match &asking {
            Asking::Exit => Key::ConfirmCancel.text().to_string(),
            Asking::Folder(text) => text.clone(),
        };
        let dialog = Rect::from_center_size(whole.center() - vec2(0.0, 12.0), vec2(420.0, 164.0));
        let soft = Shadow { offset: [0, 12], blur: 44, spread: 0, color: Color32::from_black_alpha(46) };
        ui.painter().add(soft.as_shape(dialog, CornerRadius::same(18)));
        ui.painter().rect_filled(dialog, 18, PAPER);
        area(ui, Rect::from_min_max(dialog.min + vec2(28.0, 28.0), dialog.max - vec2(28.0, 68.0)), |ui| {
            ui.label(RichText::new(sentence).size(15.0));
        });
        let no = Rect::from_min_size(dialog.max - vec2(24.0, 22.0) - BUTTON, BUTTON);
        let yes = no.translate(vec2(-(BUTTON.x + 10.0), 0.0));
        let ctx = ui.ctx().clone();
        if self.button(ui, yes, Key::Yes.text(), Weight::Primary, true) {
            self.answer(true, &ctx);
        } else if self.button(ui, no, Key::No.text(), Weight::Plain, true) {
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
        if !self.shown {
            // only now that the window is up: had it failed to open, the questions
            // would be asked in the console instead
            self.shown = true;
            platform::leave_own_console();
        }
        self.read_work();
        if ctx.input(|i| i.viewport().close_requested()) && !self.closing {
            // the close box is Cancel: it asks first, and an install is undone before the window goes
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            self.cancel(&ctx);
        }
        self.keys(&ctx);

        let whole = ui.max_rect();
        ui.painter().rect_filled(whole, 0, PAPER);
        let strip = Rect::from_min_size(whole.min, vec2(SIDE, whole.height()));
        let main = Rect::from_min_max(pos2(strip.max.x, whole.min.y), whole.max).shrink2(vec2(36.0, 0.0));
        let body = Rect::from_min_max(pos2(main.min.x, main.min.y + 34.0), pos2(main.max.x, main.max.y - FOOTER));
        let row = Rect::from_min_max(pos2(main.min.x, body.max.y), main.max);

        let asking = self.asking.is_some();
        let mut page = ui.new_child(UiBuilder::new().max_rect(whole));
        if asking {
            page.disable();
        }
        self.side(&page, strip);
        let draw: fn(&mut App, &mut Ui) = match self.wizard.page {
            Page::Welcome => App::welcome,
            Page::License => App::license,
            Page::Folder => App::folder,
            Page::Tasks => App::tasks,
            Page::Ready => App::ready,
            Page::Remove => App::remove,
            Page::Working => App::working,
            Page::Finished => App::finished,
        };
        area(&mut page, body, |ui| draw(self, ui));
        self.buttons(&mut page, row);
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

    #[test]
    fn the_side_strip_names_the_step_the_wizard_is_on() {
        assert_eq!(steps(Mode::Install, Page::Folder), (&["Welcome", "License", "Location", "Options", "Review", "Install", "Finish"][..], 2));
        assert_eq!(steps(Mode::Install, Page::Finished).1, 6);
        assert_eq!(steps(Mode::Uninstall, Page::Working), (&["Confirm", "Remove", "Finish"][..], 1));
    }
}
