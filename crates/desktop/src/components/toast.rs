//! Stacked toast notifications with fade in/out animations.
//!
//! Adapted from iced's official `examples/toast` example
//! (https://github.com/iced-rs/iced/blob/master/examples/toast/src/main.rs)
//! and extended with per-toast opacity that fades in on appearance and out
//! when the timer expires.
//!
//! `Manager` wraps the app's content `Element` and overlays a vertical stack of
//! toasts in the lower-right corner. All animation timing and opacity state
//! lives inside this module — call sites only push `Toast` values onto a `Vec`
//! and let the manager handle the rest.
//!
//! ## How the fade works
//!
//! - Animation state lives in two places:
//!   1. The widget tree state (`Vec<Option<ToastTimer>>`) holds creation +
//!      dismissal timestamps. This persists across frames because iced's
//!      tree state outlives `Manager` instances (which are rebuilt every
//!      `view()` call).
//!   2. A per-row `Rc<Cell<f32>>` opacity cell, captured by the style closures
//!      that build the toast's container/text. The cell is rebuilt every
//!      frame; the overlay refreshes it from tree state on every
//!      `RedrawRequested` tick before the children draw themselves.
//! - On every redraw the overlay computes
//!   `alpha = fade_in_factor * fade_out_factor`
//!   from the timer struct and writes it into the cell. The style closures
//!   pick it up via `Color::scale_alpha`.
//! - Auto-dismiss is two-phase: first the timer expires and a `dismissing`
//!   instant is recorded; once the fade-out completes, the manager publishes
//!   `on_close(idx)` so `App` can drop the toast from its `Vec`.
//! - Manual close (clicking the × button) is instant — the button publishes
//!   `on_close(idx)` directly, and the next frame the toast is gone.

use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use iced::{
    Alignment, Background, Border, Color, Element, Event, Fill, Length, Point, Rectangle, Size,
    Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget,
        layout::{self, Limits, Node},
        mouse::{self, Cursor},
        overlay,
        renderer::{self, Quad},
        widget::{self, Tree},
    },
    widget::{button, column, container, row, text},
    window,
};

use crate::{
    components::icons,
    theme::{AppColors, AppTheme, RADIUS_MD},
};

const DEFAULT_TIMEOUT_SECS: u64 = 5;
const TOAST_MAX_WIDTH: f32 = 320.0;
const FADE_IN_MS: u64 = 150;
const FADE_OUT_MS: u64 = 150;
const PROGRESS_BAR_HEIGHT: f32 = 3.0;
/// Approximately 30fps. We tick this fast for the entire visible lifetime of a
/// toast so the progress bar shrinks smoothly and the fades look continuous.
const ANIMATION_TICK_MS: u64 = 33;
/// Peak opacity: toasts never quite reach fully opaque so the content behind
/// them stays subtly visible.
const MAX_ALPHA: f32 = 0.95;
/// Space reserved at the top of the overlay so the toast stack never rides
/// under the 32px custom title bar. Bottom/side padding stays tight.
const OVERLAY_TOP_PAD: f32 = 48.0;
const OVERLAY_SIDE_PAD: f32 = 16.0;

/// Per-toast animation timestamps. Stored in the widget tree state so they
/// survive across `view()` rebuilds.
#[derive(Debug, Clone, Copy)]
struct ToastTimer {
    created: Instant,
    /// Set when the auto-dismiss timer fires; the toast then fades out over
    /// `FADE_OUT_MS` and is published once that completes.
    dismissing: Option<Instant>,
    /// True while the cursor is over the toast — pauses the auto-dismiss
    /// countdown and pins the visible state to "fully visible / full progress".
    hovered: bool,
}

/// Compute the current display alpha for a toast in `[0.0, MAX_ALPHA]`. The
/// fade-in/out animation factors are always scaled by `MAX_ALPHA` so the peak
/// opacity is 90%, keeping the toast a bit translucent even when "fully
/// visible".
fn compute_alpha(timer: &ToastTimer, now: Instant) -> f32 {
    // While hovered, the toast is pinned at the peak opacity.
    if timer.hovered && timer.dismissing.is_none() {
        return MAX_ALPHA;
    }

    let appear_ms = now.saturating_duration_since(timer.created).as_millis() as f32;
    let fade_in = (appear_ms / FADE_IN_MS as f32).clamp(0.0, 1.0);

    let fade_out = match timer.dismissing {
        Some(start) => {
            let dismiss_ms = now.saturating_duration_since(start).as_millis() as f32;
            (1.0 - (dismiss_ms / FADE_OUT_MS as f32)).clamp(0.0, 1.0)
        }
        None => 1.0,
    };

    fade_in * fade_out * MAX_ALPHA
}

/// Fraction of the visible-time budget remaining: 1.0 at creation, 0.0 once the
/// auto-dismiss fires. Used to drive the bottom progress bar. While hovered the
/// progress is pinned to 1.0.
fn compute_progress(timer: &ToastTimer, now: Instant, timeout: Duration) -> f32 {
    if timer.hovered && timer.dismissing.is_none() {
        return 1.0;
    }
    let elapsed = now.saturating_duration_since(timer.created).as_secs_f32();
    let total = timeout.as_secs_f32();
    if total <= 0.0 {
        0.0
    } else {
        (1.0 - elapsed / total).clamp(0.0, 1.0)
    }
}

/// A user-facing notification.
#[derive(Debug, Clone)]
pub struct Toast {
    pub title: String,
    pub body: String,
    pub status: ToastStatus,
}

#[derive(Debug, Clone, Copy)]
pub enum ToastStatus {
    Info,
    Success,
    Warning,
    Error,
}

impl Toast {
    pub fn info(body: impl Into<String>) -> Self {
        Self {
            title: "Info".to_string(),
            body: body.into(),
            status: ToastStatus::Info,
        }
    }

    pub fn success(body: impl Into<String>) -> Self {
        Self {
            title: "Success".to_string(),
            body: body.into(),
            status: ToastStatus::Success,
        }
    }

    pub fn warning(body: impl Into<String>) -> Self {
        Self {
            title: "Warning".to_string(),
            body: body.into(),
            status: ToastStatus::Warning,
        }
    }

    pub fn error(body: impl Into<String>) -> Self {
        Self {
            title: "Error".to_string(),
            body: body.into(),
            status: ToastStatus::Error,
        }
    }
}

impl ToastStatus {
    fn background(self, colors: &AppColors) -> Color {
        match self {
            ToastStatus::Info => colors.toast_info_bg,
            ToastStatus::Success => colors.toast_success_bg,
            ToastStatus::Warning => colors.toast_warning_bg,
            ToastStatus::Error => colors.toast_error_bg,
        }
    }

    /// Lighter tint of the severity background, used for the progress bar fill
    /// so it sits on the same hue as the toast without being jarring.
    fn progress_color(self, colors: &AppColors) -> Color {
        let bg = self.background(colors);
        // Linear-RGB lerp toward white at 60%; gives a noticeably lighter
        // version of the same hue.
        const T: f32 = 0.6;
        Color {
            r: bg.r + (1.0 - bg.r) * T,
            g: bg.g + (1.0 - bg.g) * T,
            b: bg.b + (1.0 - bg.b) * T,
            a: 1.0,
        }
    }
}

// ── Manager widget ─────────────────────────────────────────────────────────

/// Wraps the app's main content and overlays a stack of toasts.
pub struct Manager<'a, Message> {
    content: Element<'a, Message, AppTheme>,
    toasts: Vec<Element<'a, Message, AppTheme>>,
    /// Per-toast opacity, shared between the overlay (which writes) and the
    /// style closures (which read). Rebuilt every frame; current values are
    /// re-derived from tree state on each `RedrawRequested`.
    opacities: Vec<Rc<Cell<f32>>>,
    /// Per-toast progress bar value (1.0 → 0.0 over the visible lifetime),
    /// updated alongside `opacities` from the overlay tick.
    progresses: Vec<Rc<Cell<f32>>>,
    timeout_secs: u64,
    on_close: Box<dyn Fn(usize) -> Message + 'a>,
}

impl<'a, Message> Manager<'a, Message>
where
    Message: 'a + Clone,
{
    pub fn new(
        content: impl Into<Element<'a, Message, AppTheme>>,
        toasts: &'a [Toast],
        on_close: impl Fn(usize) -> Message + 'a,
    ) -> Self {
        // Cells start at "fully transparent / fully empty"; `diff()` immediately
        // reseeds them from the persisted timer state for the first draw.
        let opacities: Vec<Rc<Cell<f32>>> =
            toasts.iter().map(|_| Rc::new(Cell::new(0.0))).collect();
        let progresses: Vec<Rc<Cell<f32>>> =
            toasts.iter().map(|_| Rc::new(Cell::new(1.0))).collect();

        let toast_elements = toasts
            .iter()
            .enumerate()
            .map(|(index, toast)| {
                toast_view(
                    index,
                    toast,
                    opacities[index].clone(),
                    progresses[index].clone(),
                    &on_close,
                )
            })
            .collect();

        Self {
            content: content.into(),
            toasts: toast_elements,
            opacities,
            progresses,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            on_close: Box::new(on_close),
        }
    }

    pub fn timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

fn toast_view<'a, Message: 'a + Clone>(
    index: usize,
    toast: &Toast,
    opacity: Rc<Cell<f32>>,
    progress: Rc<Cell<f32>>,
    on_close: &(impl Fn(usize) -> Message + 'a),
) -> Element<'a, Message, AppTheme> {
    let status = toast.status;

    // Pick the severity icon (white-on-color filled glyph from Bootstrap Icons).
    let severity_icon: char = match status {
        ToastStatus::Info => icons::INFO_CIRCLE_FILL.char(),
        ToastStatus::Success => icons::CHECK_CIRCLE_FILL.char(),
        ToastStatus::Warning => icons::EXCLAMATION_TRIANGLE_FILL.char(),
        ToastStatus::Error => icons::X_CIRCLE_FILL.char(),
    };

    // Each style closure captures its own clone of the opacity Rc so it can
    // read the current alpha at draw time. Rc::clone is cheap (refcount bump).
    let icon_opacity = opacity.clone();
    let title_opacity = opacity.clone();
    let body_opacity = opacity.clone();
    let close_opacity = opacity.clone();
    let outer_opacity = opacity.clone();

    let icon_elem = text(severity_icon.to_string())
        .font(icons::FONT)
        .size(22.0)
        .style(move |_theme: &AppTheme| iced::widget::text::Style {
            color: Some(Color::WHITE.scale_alpha(icon_opacity.get())),
        });

    let title_elem = text(toast.title.clone())
        .font(crate::APP_FONT_BOLD)
        .size(14)
        .style(move |_theme: &AppTheme| iced::widget::text::Style {
            color: Some(Color::WHITE.scale_alpha(title_opacity.get())),
        });

    let body_elem = text(toast.body.clone())
        .size(13)
        .style(move |_theme: &AppTheme| iced::widget::text::Style {
            // Body text slightly dimmer than the title for hierarchy.
            color: Some(Color::WHITE.scale_alpha(body_opacity.get() * 0.92)),
        });

    let close_icon = text(icons::BWI_CLOSE.char().to_string())
        .font(icons::BWI_FONT)
        .size(18.0)
        .style(move |_theme: &AppTheme| iced::widget::text::Style {
            color: Some(Color::WHITE.scale_alpha(close_opacity.get())),
        });

    let close_btn = button(close_icon)
        .on_press((on_close)(index))
        .padding([2, 4])
        .style(|_theme: &AppTheme, _status| button::Style {
            background: None,
            text_color: Color::WHITE,
            border: Border::default(),
            shadow: iced::Shadow::default(),
            snap: false,
        });

    let body_row = container(
        row![
            icon_elem,
            column![title_elem, body_elem].spacing(2).width(Fill),
            close_btn,
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding([10, 14])
    .width(Fill);

    let progress_bar: Element<'a, Message, AppTheme> = Element::new(ToastProgressBar {
        progress: progress.clone(),
        opacity: opacity.clone(),
        height: PROGRESS_BAR_HEIGHT,
        status,
    });

    let inner = column![body_row, progress_bar].width(Fill);

    container(inner)
        .max_width(TOAST_MAX_WIDTH)
        .style(move |theme: &AppTheme| {
            let alpha = outer_opacity.get();
            container::Style {
                background: Some(Background::Color(
                    status.background(&theme.colors).scale_alpha(alpha),
                )),
                border: Border {
                    radius: RADIUS_MD.into(),
                    ..Default::default()
                },
                shadow: iced::Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.30 * alpha),
                    offset: Vector::new(0.0, 2.0),
                    blur_radius: 6.0,
                },
                ..Default::default()
            }
        })
        .into()
}

// ── Progress bar widget ───────────────────────────────────────────────────
//
// Tiny custom widget that paints a horizontal fill rectangle whose width is
// proportional to the shared `progress` cell. Implementing this as a widget
// (rather than using `container.width(Length::Fixed(...))`) lets us update the
// fill ratio every frame WITHOUT triggering a re-layout — the closure inside
// `draw` reads the current cell value at paint time.

struct ToastProgressBar {
    progress: Rc<Cell<f32>>,
    opacity: Rc<Cell<f32>>,
    height: f32,
    status: ToastStatus,
}

impl<Message> Widget<Message, AppTheme, iced::Renderer> for ToastProgressBar {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fixed(self.height))
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &iced::Renderer, limits: &Limits) -> Node {
        let max_width = limits.max().width;
        Node::new(Size::new(max_width, self.height))
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &AppTheme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: Cursor,
        _viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;

        let bounds = layout.bounds();
        let progress = self.progress.get().clamp(0.0, 1.0);
        let alpha = self.opacity.get();
        let filled_width = bounds.width * progress;
        if filled_width <= 0.0 || alpha <= 0.0 {
            return;
        }
        let color = self.status.progress_color(&theme.colors);
        renderer.fill_quad(
            Quad {
                bounds: Rectangle {
                    x: bounds.x,
                    y: bounds.y,
                    width: filled_width,
                    height: bounds.height,
                },
                border: Border::default(),
                shadow: iced::Shadow::default(),
                snap: false,
            },
            Background::Color(color.scale_alpha(alpha)),
        );
    }
}

impl<Message> Widget<Message, AppTheme, iced::Renderer> for Manager<'_, Message>
where
    Message: Clone,
{
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &iced::Renderer, limits: &Limits) -> Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn tag(&self) -> widget::tree::Tag {
        struct Marker;
        widget::tree::Tag::of::<Marker>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(Vec::<Option<ToastTimer>>::new())
    }

    fn children(&self) -> Vec<Tree> {
        std::iter::once(Tree::new(&self.content))
            .chain(self.toasts.iter().map(Tree::new))
            .collect()
    }

    fn diff(&self, tree: &mut Tree) {
        let timers = tree.state.downcast_mut::<Vec<Option<ToastTimer>>>();

        // Drop dismissed entries (the overlay nulls out the slot when fade-out
        // completes) before re-syncing with the new toast list.
        timers.retain(Option::is_some);

        match (timers.len(), self.toasts.len()) {
            (old, new) if old > new => timers.truncate(new),
            (old, new) if old < new => {
                let now = Instant::now();
                timers.extend(std::iter::repeat_n(
                    Some(ToastTimer {
                        created: now,
                        dismissing: None,
                        hovered: false,
                    }),
                    new - old,
                ));
            }
            _ => {}
        }

        // Seed each opacity + progress cell from the persisted timer state so
        // the first draw of a frame has the correct values even if the
        // overlay's update hasn't run yet (e.g. when the rebuild was triggered
        // by a mouse event rather than a `RedrawRequested`).
        let now = Instant::now();
        let timeout = Duration::from_secs(self.timeout_secs);
        for (idx, slot) in timers.iter().enumerate() {
            let (alpha, progress) = match slot {
                Some(timer) => (
                    compute_alpha(timer, now),
                    compute_progress(timer, now, timeout),
                ),
                None => (0.0, 0.0),
            };
            if let Some(opacity) = self.opacities.get(idx) {
                opacity.set(alpha);
            }
            if let Some(prog) = self.progresses.get(idx) {
                prog.set(progress);
            }
        }

        let children: Vec<&Element<'_, Message, AppTheme>> = std::iter::once(&self.content)
            .chain(self.toasts.iter())
            .collect();
        tree.diff_children(&children);
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &AppTheme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn operate<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation<()>,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, AppTheme, iced::Renderer>> {
        let timers = tree.state.downcast_mut::<Vec<Option<ToastTimer>>>();
        let (content_state, toasts_state) = tree.children.split_at_mut(1);

        let content_overlay = self.content.as_widget_mut().overlay(
            &mut content_state[0],
            layout,
            renderer,
            viewport,
            translation,
        );

        let toasts_overlay = (!self.toasts.is_empty()).then(|| {
            overlay::Element::new(Box::new(ToastOverlay {
                position: layout.bounds().position() + translation,
                viewport: *viewport,
                toasts: &mut self.toasts,
                trees: toasts_state,
                timers,
                opacities: &self.opacities,
                progresses: &self.progresses,
                on_close: &self.on_close,
                timeout_secs: self.timeout_secs,
            }))
        });

        let overlays: Vec<_> = content_overlay.into_iter().chain(toasts_overlay).collect();
        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

impl<'a, Message: 'a + Clone> From<Manager<'a, Message>>
    for Element<'a, Message, AppTheme, iced::Renderer>
{
    fn from(manager: Manager<'a, Message>) -> Self {
        Element::new(manager)
    }
}

// ── Overlay ────────────────────────────────────────────────────────────────

struct ToastOverlay<'a, 'b, Message> {
    position: Point,
    viewport: Rectangle,
    toasts: &'b mut [Element<'a, Message, AppTheme>],
    trees: &'b mut [Tree],
    timers: &'b mut [Option<ToastTimer>],
    opacities: &'b [Rc<Cell<f32>>],
    progresses: &'b [Rc<Cell<f32>>],
    on_close: &'b dyn Fn(usize) -> Message,
    timeout_secs: u64,
}

impl<Message> overlay::Overlay<Message, AppTheme, iced::Renderer> for ToastOverlay<'_, '_, Message>
where
    Message: Clone,
{
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> Node {
        let limits = Limits::new(Size::ZERO, bounds);

        // Bigger top inset so the stack never slides under the 32px custom
        // title bar if it grows tall.
        let padding = iced::Padding {
            top: OVERLAY_TOP_PAD,
            right: OVERLAY_SIDE_PAD,
            bottom: 0.0,
            left: OVERLAY_SIDE_PAD,
        };

        layout::flex::resolve(
            layout::flex::Axis::Vertical,
            renderer,
            &limits,
            Length::Fill,
            Length::Fill,
            padding,
            10.0,
            Alignment::End,
            self.toasts,
            self.trees,
        )
        .translate(Vector::new(self.position.x, self.position.y))
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            // Drive the per-toast animation state machine. The timer for each
            // toast walks: appearing → visible → dismissing → dropped. Both
            // the opacity and progress cells are refreshed every tick so the
            // next draw picks them up. We schedule another frame at ~60fps for
            // the entire visible lifetime — the progress bar shrinks smoothly
            // and fades stay continuous.
            let timeout = Duration::from_secs(self.timeout_secs);
            let fade_out_d = Duration::from_millis(FADE_OUT_MS);
            let cursor_pos = cursor.position();
            let mut child_layouts = layout.children();

            for (index, slot) in self.timers.iter_mut().enumerate() {
                let child_layout = child_layouts.next();
                let Some(timer) = slot.as_mut() else {
                    continue;
                };

                // Hover detection: if the cursor is inside this toast's
                // bounds, pin it to "fully visible / full progress" so it
                // doesn't disappear while the user is reading it. Cancel any
                // in-flight dismissal and slide `created` forward so when the
                // cursor leaves, the timer naturally resumes from full.
                let cursor_over = cursor_pos
                    .and_then(|p| child_layout.map(|cl| cl.bounds().contains(p)))
                    .unwrap_or(false);
                timer.hovered = cursor_over;
                if cursor_over {
                    timer.dismissing = None;
                    // Past the fade-in so we don't re-fade-in on un-hover.
                    timer.created = *now - Duration::from_millis(FADE_IN_MS);
                }

                // Phase 1: kick off auto-dismiss once the visible timer expires.
                if timer.dismissing.is_none() && timer.created.elapsed() >= timeout {
                    timer.dismissing = Some(*now);
                }

                // Phase 2: publish on_close once the fade-out completes.
                if let Some(start) = timer.dismissing
                    && now.saturating_duration_since(start) >= fade_out_d
                {
                    *slot = None;
                    if let Some(opacity) = self.opacities.get(index) {
                        opacity.set(0.0);
                    }
                    if let Some(progress) = self.progresses.get(index) {
                        progress.set(0.0);
                    }
                    shell.publish((self.on_close)(index));
                    continue;
                }

                // Refresh the shared cells for the upcoming draw call.
                let alpha = compute_alpha(timer, *now);
                let progress = compute_progress(timer, *now, timeout);
                if let Some(opacity) = self.opacities.get(index) {
                    opacity.set(alpha);
                }
                if let Some(prog) = self.progresses.get(index) {
                    prog.set(progress);
                }

                shell.request_redraw_at(*now + Duration::from_millis(ANIMATION_TICK_MS));
            }
        }

        let viewport = layout.bounds();

        for (((child, state), child_layout), timer_slot) in self
            .toasts
            .iter_mut()
            .zip(self.trees.iter_mut())
            .zip(layout.children())
            .zip(self.timers.iter_mut())
        {
            let mut local_messages = Vec::new();
            let mut local_shell = Shell::new(&mut local_messages);

            child.as_widget_mut().update(
                state,
                event,
                child_layout,
                cursor,
                renderer,
                clipboard,
                &mut local_shell,
                &viewport,
            );

            // Any local message (e.g. close button click) is an explicit
            // dismissal — drop the timer so the post-update sweep doesn't
            // re-publish on_close after the toast is already gone.
            if !local_shell.is_empty() {
                *timer_slot = None;
            }
            shell.merge(local_shell, std::convert::identity);
        }
    }

    fn draw(
        &self,
        renderer: &mut iced::Renderer,
        theme: &AppTheme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
    ) {
        let viewport = layout.bounds();
        for ((child, tree), child_layout) in self
            .toasts
            .iter()
            .zip(self.trees.iter())
            .zip(layout.children())
        {
            child.as_widget().draw(
                tree,
                renderer,
                theme,
                style,
                child_layout,
                cursor,
                &viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.toasts
            .iter()
            .zip(self.trees.iter())
            .zip(layout.children())
            .map(|((child, state), child_layout)| {
                child.as_widget().mouse_interaction(
                    state,
                    child_layout,
                    cursor,
                    &self.viewport,
                    renderer,
                )
            })
            .max()
            .unwrap_or_default()
    }
}
