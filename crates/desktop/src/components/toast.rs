//! Stacked toast notifications with fade in/out animations. Adapted from
//! iced's official `examples/toast` and extended with per-toast opacity
//! fades, a countdown progress bar, and hover-to-pause.
//!
//! Animation pipeline: persistent `ToastTimer`s in widget-tree state drive
//! per-frame `Rc<Cell<ToastVisuals>>` cells that style closures read at draw
//! time — no layout invalidation. Auto-dismiss is two-phase (timer →
//! fade-out → `on_close`); manual close publishes immediately. Hovering
//! pins to full visibility and cancels any in-flight dismissal.

use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use iced::{
    Alignment, Background, Border, Color, Element, Event, Fill, Length, Point, Rectangle, Size,
    Vector,
    advanced::{
        Layout, Shell, Widget,
        layout::{self, Limits, Node},
        mouse::{self, Cursor},
        overlay,
        renderer::{self, Quad},
        widget::{self, Tree},
    },
    border::Radius,
    widget::{button, column, container, row, text},
    window,
};

use crate::{
    components::icons,
    theme::{AppColors, AppTheme, RADIUS_MD},
};

// ── Tunables ───────────────────────────────────────────────────────────────

/// How long each toast stays visible after fade-in completes.
const TIMEOUT: Duration = Duration::from_secs(5);
const TOAST_MAX_WIDTH: f32 = 320.0;
const FADE_IN_MS: u64 = 150;
const FADE_OUT_MS: u64 = 150;
const PROGRESS_BAR_HEIGHT: f32 = 3.0;
/// ~30fps tick for the toast's full visible lifetime so the progress bar
/// shrinks smoothly and fades stay continuous.
const ANIMATION_TICK_MS: u64 = 33;
/// Peak opacity: toasts stay slightly translucent so content behind them
/// remains subtly visible.
const MAX_ALPHA: f32 = 0.95;
/// Top inset so the toast stack never rides under the 32px custom title bar.
const OVERLAY_TOP_PAD: f32 = 48.0;
const OVERLAY_SIDE_PAD: f32 = 16.0;

// ── State types ────────────────────────────────────────────────────────────

/// Per-toast animation timestamps. Stored in the widget tree state so they
/// survive across `view()` rebuilds.
#[derive(Debug, Clone, Copy)]
struct ToastTimer {
    created: Instant,
    /// Set when the auto-dismiss timer fires; the toast then fades out over
    /// `FADE_OUT_MS` and is published once complete.
    dismissing: Option<Instant>,
    /// True while the cursor is over the toast — pauses the countdown and
    /// pins the visible state to "fully visible".
    hovered: bool,
}

/// Per-frame animation values shared between the overlay (which writes) and
/// the toast row's style closures (which read). One cell per toast row,
/// refreshed on every `RedrawRequested` tick from the persisted `ToastTimer`.
#[derive(Debug, Clone, Copy)]
struct ToastVisuals {
    alpha: f32,
    progress: f32,
}

impl ToastVisuals {
    const HIDDEN: Self = Self {
        alpha: 0.0,
        progress: 0.0,
    };
}

/// Compute the current display alpha for a toast in `[0.0, MAX_ALPHA]`.
fn compute_alpha(timer: &ToastTimer, now: Instant) -> f32 {
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

/// Fraction of the visible-time budget remaining: 1.0 once fully visible,
/// 0.0 once auto-dismiss fires. Pinned to 1.0 while hovered. The visible
/// budget excludes the fade-in window so the bar starts at exactly 1.0
/// when the toast is fully opaque, and pins to 0.0 the moment dismissal
/// begins (so the bar doesn't drift below 0 during the fade-out).
fn compute_progress(timer: &ToastTimer, now: Instant) -> f32 {
    if timer.dismissing.is_some() {
        return 0.0;
    }
    if timer.hovered {
        return 1.0;
    }
    let fade_in_secs = FADE_IN_MS as f32 / 1000.0;
    let visible_budget = TIMEOUT.as_secs_f32() - fade_in_secs;
    let elapsed = now.saturating_duration_since(timer.created).as_secs_f32();
    let elapsed_visible = (elapsed - fade_in_secs).max(0.0);
    (1.0 - elapsed_visible / visible_budget).clamp(0.0, 1.0)
}

fn refresh_visuals(timer: &ToastTimer, now: Instant) -> ToastVisuals {
    ToastVisuals {
        alpha: compute_alpha(timer, now),
        progress: compute_progress(timer, now),
    }
}

// ── Public API ─────────────────────────────────────────────────────────────

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
    fn new(status: ToastStatus, body: impl Into<String>, title: Option<&str>) -> Self {
        Self {
            title: title.unwrap_or(status.title()).to_string(),
            body: body.into(),
            status,
        }
    }

    pub fn info(body: impl Into<String>, title: Option<&str>) -> Self {
        Self::new(ToastStatus::Info, body, title)
    }

    pub fn success(body: impl Into<String>, title: Option<&str>) -> Self {
        Self::new(ToastStatus::Success, body, title)
    }

    pub fn warning(body: impl Into<String>, title: Option<&str>) -> Self {
        Self::new(ToastStatus::Warning, body, title)
    }

    pub fn error(body: impl Into<String>, title: Option<&str>) -> Self {
        Self::new(ToastStatus::Error, body, title)
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

    /// Lighter tint of the severity background for the progress bar fill.
    fn progress_color(self, colors: &AppColors) -> Color {
        let bg = self.background(colors);
        // Linear-RGB lerp toward white at 60% — noticeably lighter on the
        // same hue.
        const T: f32 = 0.6;
        Color {
            r: bg.r + (1.0 - bg.r) * T,
            g: bg.g + (1.0 - bg.g) * T,
            b: bg.b + (1.0 - bg.b) * T,
            a: 1.0,
        }
    }

    fn icon(self) -> char {
        match self {
            ToastStatus::Info => icons::INFO_CIRCLE_FILL.char(),
            ToastStatus::Success => icons::CHECK_CIRCLE_FILL.char(),
            ToastStatus::Warning => icons::EXCLAMATION_TRIANGLE_FILL.char(),
            ToastStatus::Error => icons::X_CIRCLE_FILL.char(),
        }
    }

    /// Default title when the caller passes `None`.
    fn title(self) -> &'static str {
        match self {
            ToastStatus::Info => "Info",
            ToastStatus::Success => "Success",
            ToastStatus::Warning => "Warning",
            ToastStatus::Error => "Error",
        }
    }
}

// ── Manager widget ─────────────────────────────────────────────────────────

/// Wraps the app's main content and overlays a stack of toasts.
pub struct Manager<'a, Message> {
    content: Element<'a, Message, AppTheme>,
    toasts: Vec<Element<'a, Message, AppTheme>>,
    /// Per-toast animation cells aligned 1:1 with `toasts`. The overlay
    /// writes on each tick; row style closures read at draw time.
    cells: Vec<Rc<Cell<ToastVisuals>>>,
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
        // Cells start hidden; `diff()` reseeds them from the persisted
        // timer state for the first draw.
        let cells: Vec<Rc<Cell<ToastVisuals>>> = toasts
            .iter()
            .map(|_| Rc::new(Cell::new(ToastVisuals::HIDDEN)))
            .collect();

        let toast_elements = toasts
            .iter()
            .enumerate()
            .map(|(index, toast)| toast_view(index, toast, cells[index].clone(), &on_close))
            .collect();

        Self {
            content: content.into(),
            toasts: toast_elements,
            cells,
            on_close: Box::new(on_close),
        }
    }
}

/// `text::Style` closure painting `Color::WHITE` scaled by the visuals
/// cell's `alpha`, optionally further dimmed by `factor`.
fn white_alpha_style(
    cell: Rc<Cell<ToastVisuals>>,
    factor: f32,
) -> impl Fn(&AppTheme) -> iced::widget::text::Style {
    move |_theme: &AppTheme| iced::widget::text::Style {
        color: Some(Color::WHITE.scale_alpha(cell.get().alpha * factor)),
    }
}

fn toast_view<'a, Message: 'a + Clone>(
    index: usize,
    toast: &'a Toast,
    cell: Rc<Cell<ToastVisuals>>,
    on_close: &(impl Fn(usize) -> Message + 'a),
) -> Element<'a, Message, AppTheme> {
    let status = toast.status;

    let icon_elem = text(status.icon().to_string())
        .font(icons::FONT)
        .size(22.0)
        .style(white_alpha_style(cell.clone(), 1.0));

    let title_elem = text(toast.title.as_str())
        .font(crate::APP_FONT_BOLD)
        .size(14)
        .style(white_alpha_style(cell.clone(), 1.0));

    let body_elem = text(toast.body.as_str())
        .size(12)
        .style(white_alpha_style(cell.clone(), 0.92));

    let close_icon = text(icons::BWI_CLOSE.char().to_string())
        .font(icons::BWI_FONT)
        .size(18.0)
        .style(white_alpha_style(cell.clone(), 1.0));

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
        cell: cell.clone(),
        height: PROGRESS_BAR_HEIGHT,
        status,
    });

    let inner = column![body_row, progress_bar].width(Fill);

    let outer_cell = cell;
    container(inner)
        .max_width(TOAST_MAX_WIDTH)
        .style(move |theme: &AppTheme| {
            let alpha = outer_cell.get().alpha;
            container::Style::default()
                .background(status.background(&theme.colors).scale_alpha(alpha))
                .border(iced::border::rounded(RADIUS_MD))
                .shadow(iced::Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.30 * alpha),
                    offset: Vector::new(0.0, 2.0),
                    blur_radius: 6.0,
                })
        })
        .into()
}

// ── Progress bar widget ───────────────────────────────────────────────────
//
// Implemented as a custom widget so the fill ratio updates per-frame
// WITHOUT triggering re-layout — `draw` reads the cell at paint time.
// `container.width(Length::Fixed(...))` would force a relayout per tick.

struct ToastProgressBar {
    cell: Rc<Cell<ToastVisuals>>,
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

        let visuals = self.cell.get();
        let bounds = layout.bounds();
        let progress = visuals.progress.clamp(0.0, 1.0);
        let alpha = visuals.alpha;
        let filled_width = (bounds.width - 2.0) * progress;
        if filled_width <= 0.0 || alpha <= 0.0 {
            return;
        }
        let color = self.status.progress_color(&theme.colors);
        renderer.fill_quad(
            Quad {
                bounds: Rectangle {
                    x: bounds.x + 1.0,
                    y: bounds.y - 1.0,
                    width: filled_width,
                    height: bounds.height,
                },
                border: Border::default().rounded(Radius::default().bottom(bounds.height / 2.0)),
                shadow: iced::Shadow::default(),
                snap: false,
            },
            Background::Color(color.scale_alpha(alpha)),
        );
    }
}

// ── Manager: Widget impl ───────────────────────────────────────────────────

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

        // Drop dismissed entries (overlay nulls the slot when fade-out
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

        // Seed each visuals cell from the persisted timer state so the
        // first draw of a frame has correct values even if the overlay's
        // update hasn't run yet (e.g. mouse-event-triggered rebuilds).
        let now = Instant::now();
        for (cell, slot) in self.cells.iter().zip(timers.iter()) {
            let visuals = match slot {
                Some(timer) => refresh_visuals(timer, now),
                None => ToastVisuals::HIDDEN,
            };
            cell.set(visuals);
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
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
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
                cells: &self.cells,
                on_close: &self.on_close,
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
    cells: &'b [Rc<Cell<ToastVisuals>>],
    on_close: &'b dyn Fn(usize) -> Message,
}

impl<Message> overlay::Overlay<Message, AppTheme, iced::Renderer> for ToastOverlay<'_, '_, Message>
where
    Message: Clone,
{
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> Node {
        let limits = Limits::new(Size::ZERO, bounds);

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
        shell: &mut Shell<'_, Message>,
    ) {
        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            // Per-toast state machine: appearing → visible → dismissing →
            // dropped. Alpha and progress are refreshed every tick.
            let fade_out_d = Duration::from_millis(FADE_OUT_MS);
            let cursor_pos = cursor.position();
            let mut child_layouts = layout.children();

            for ((index, slot), cell) in self.timers.iter_mut().enumerate().zip(self.cells.iter()) {
                let child_layout = child_layouts
                    .next()
                    .expect("child layout exists for every timer");
                let Some(timer) = slot.as_mut() else {
                    continue;
                };

                // Hover detection: if the cursor is inside this toast's
                // bounds, pin to fully visible. Cancel any in-flight dismissal
                // and slide `created` forward so the timer resumes from full
                // when the cursor leaves.
                let cursor_over = cursor_pos
                    .map(|p| child_layout.bounds().contains(p))
                    .unwrap_or(false);
                timer.hovered = cursor_over;
                if cursor_over {
                    timer.dismissing = None;
                    // Past the fade-in so we don't re-fade-in on un-hover.
                    timer.created = *now - Duration::from_millis(FADE_IN_MS);
                }

                // Phase 1: kick off auto-dismiss once visible timer expires.
                if timer.dismissing.is_none() && timer.created.elapsed() >= TIMEOUT {
                    timer.dismissing = Some(*now);
                }

                // Phase 2: publish on_close once fade-out completes. The
                // toast drops from `App.toasts` next frame, so no need to
                // refresh the cell — its `Element` won't be drawn again.
                if let Some(start) = timer.dismissing
                    && now.saturating_duration_since(start) >= fade_out_d
                {
                    *slot = None;
                    shell.publish((self.on_close)(index));
                    continue;
                }

                cell.set(refresh_visuals(timer, *now));

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
                &mut local_shell,
                &viewport,
            );

            // Any local message (e.g. close button click) is an explicit
            // dismissal — drop the timer so the post-update sweep doesn't
            // re-publish on_close.
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
