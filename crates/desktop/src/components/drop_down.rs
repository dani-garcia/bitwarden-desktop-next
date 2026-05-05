//! Drop down menu widget
//!
//! Copied from iced_aw 0.13.1 to allow local modifications. The local
//! customisations on top of upstream are: extra `Alignment` variants
//! (`BelowLeft` / `BelowRight` / `AboveRight`), an auto-shadow wrap in
//! [`DropDown::new`], and a slide animation driven by an internal
//! [`lilt::Animated`] that participates in the global animation watermark.

use std::time::{Duration, Instant};

use iced::{
    Border, Color, Element, Event, Length, Point, Rectangle, Shadow, Size, Vector,
    advanced::{
        Layout, Shell, Widget,
        layout::{Limits, Node},
        mouse::{self, Cursor},
        overlay, renderer,
        widget::{self, Operation, Tree, tree},
    },
    keyboard::{self, key::Named},
    touch,
    widget::container,
};
use lilt::{Animated, Easing};

use crate::{services::animation, theme::RADIUS_LG};

/// Open animation duration. Snappier than the 180 ms modal slide because
/// dropdowns travel a shorter visual distance and a menu shouldn't keep
/// the user waiting before they can act on it. Close is a snap.
const ANIM_DURATION_MS: f32 = 80.0;
const ANIM_DURATION: Duration = Duration::from_millis(ANIM_DURATION_MS as u64);

/// Vertical slide distance. The panel starts offset by this amount along
/// its open direction (above for "below" alignments, below for "above")
/// and animates to its anchored position.
const SLIDE_PX: f32 = 8.0;

/// Drop shadow applied by [`DropDown::new`] behind panels. Border radius
/// matches [`RADIUS_LG`] so the shadow follows panels' rounded corners.
/// Composite overlays that paint more than one visual panel (e.g. the title
/// bar's menu + open submenu) should use [`DropDown::new_no_shadow`] and
/// apply this shadow per-panel instead — a single wrapping shadow would
/// span the gap between panels and any layout offsets.
pub const PANEL_SHADOW: Shadow = Shadow {
    color: Color::from_rgba(0.0, 0.0, 0.0, 0.25),
    offset: Vector::new(0.0, 4.0),
    blur_radius: 16.0,
};

// ── Alignment ──────────────────────────────────────────────────────────────

/// ```text
/// +-----------+-----------+-----------+
/// | TopStart  |   Top     |  TopEnd   |
/// +-----------+-----------+-----------+
/// |  Start    |           |   End     |
/// +-----------+-----------+-----------+
/// |BottomStart|  Bottom   | BottomEnd |
/// +-----------+-----------+-----------+
/// ```
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Alignment {
    TopStart,
    Top,
    TopEnd,

    End,

    BottomEnd,
    Bottom,
    BottomStart,

    Start,

    /// Below the trigger, overlay left edge = trigger left edge.
    BelowLeft,
    /// Below the trigger, overlay right edge = trigger right edge.
    BelowRight,
    /// Above the trigger, overlay right edge = trigger right edge.
    AboveRight,
}

// ── Offset ─────────────────────────────────────────────────────────────────

#[derive(Copy, Clone, Debug)]
pub struct Offset {
    pub x: f32,
    pub y: f32,
}

impl Offset {
    #[must_use]
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl From<f32> for Offset {
    fn from(float: f32) -> Self {
        Self { x: float, y: float }
    }
}

impl From<[f32; 2]> for Offset {
    fn from(array: [f32; 2]) -> Self {
        Self {
            x: array[0],
            y: array[1],
        }
    }
}

impl From<Offset> for Point {
    fn from(offset: Offset) -> Self {
        Self::new(offset.x, offset.y)
    }
}

impl From<&Offset> for Point {
    fn from(offset: &Offset) -> Self {
        Self::new(offset.x, offset.y)
    }
}

// ── DropDown widget ────────────────────────────────────────────────────────

/// Customizable drop down menu widget
pub struct DropDown<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Message: Clone,
    Renderer: renderer::Renderer,
{
    underlay: Element<'a, Message, Theme, Renderer>,
    overlay: Element<'a, Message, Theme, Renderer>,
    on_dismiss: Option<Message>,
    width: Option<Length>,
    height: Length,
    alignment: Alignment,
    offset: Offset,
    expanded: bool,
}

/// Per-instance state held in iced's widget tree. Drives the slide
/// animation and keeps the overlay mounted through the close transition.
#[derive(Debug)]
struct AnimState {
    anim: Animated<bool, Instant>,
}

impl Default for AnimState {
    fn default() -> Self {
        Self {
            anim: Animated::new(false)
                .duration(ANIM_DURATION_MS)
                .easing(Easing::EaseOut),
        }
    }
}

impl<'a, Message, Theme, Renderer> DropDown<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Renderer: renderer::Renderer + 'a,
{
    /// Create a new [`DropDown`].
    ///
    /// The overlay is wrapped in a shadow container ([`RADIUS_LG`]) and
    /// [`iced::widget::opaque`] so panel empty-space clicks don't fall
    /// through. Caller panels should match the shadow radius so the
    /// shadow follows their rounded corners.
    pub fn new<U, B>(underlay: U, overlay: B, expanded: bool) -> Self
    where
        U: Into<Element<'a, Message, Theme, Renderer>>,
        B: Into<Element<'a, Message, Theme, Renderer>>,
        Theme: container::Catalog + 'a,
        <Theme as container::Catalog>::Class<'a>: From<container::StyleFn<'a, Theme>>,
    {
        let shadowed = container(overlay).style(|_theme: &Theme| container::Style {
            shadow: PANEL_SHADOW,
            border: Border::default().rounded(RADIUS_LG),
            ..container::Style::default()
        });
        Self::new_no_shadow(underlay, shadowed, expanded)
    }

    /// Like [`new`](Self::new) but without the auto-shadow wrap. Use this
    /// when the overlay paints more than one visual panel side-by-side
    /// (e.g. a menu with an open submenu) and each panel needs its own
    /// shadow — apply [`PANEL_SHADOW`] inside each panel's container style.
    pub fn new_no_shadow<U, B>(underlay: U, overlay: B, expanded: bool) -> Self
    where
        U: Into<Element<'a, Message, Theme, Renderer>>,
        B: Into<Element<'a, Message, Theme, Renderer>>,
        Theme: 'a,
    {
        DropDown {
            underlay: underlay.into(),
            overlay: iced::widget::opaque(overlay),
            expanded,
            on_dismiss: None,
            width: None,
            height: Length::Shrink,
            alignment: Alignment::Bottom,
            offset: Offset::from(5.0),
        }
    }

    #[must_use]
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    #[must_use]
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    #[must_use]
    pub fn alignment(mut self, alignment: impl Into<Alignment>) -> Self {
        self.alignment = alignment.into();
        self
    }

    #[must_use]
    pub fn offset(mut self, offset: impl Into<Offset>) -> Self {
        self.offset = offset.into();
        self
    }

    /// Click-outside-to-dismiss is opt-in: omit this on click-to-toggle
    /// triggers (e.g. a button that opens the panel) so the same click that
    /// opened the overlay doesn't immediately close it.
    #[must_use]
    pub fn on_dismiss(mut self, message: Message) -> Self {
        self.on_dismiss = Some(message);
        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for DropDown<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + renderer::Renderer,
{
    fn size(&self) -> Size<Length> {
        self.underlay.as_widget().size()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        self.underlay
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        state: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        self.underlay.as_widget().draw(
            &state.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.underlay), Tree::new(&self.overlay)]
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<AnimState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(AnimState::default())
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[&self.underlay, &self.overlay]);

        // Sync the animator's target with the prop. Open animates;
        // close is a snap — mutually-exclusive menus (title bar) would
        // overlap visually if both the closing and opening panels
        // animated, and click-outside dismissal feels laggier with a
        // close transition.
        let state = tree.state.downcast_mut::<AnimState>();
        if state.anim.value != self.expanded {
            state.anim.transition(self.expanded, Instant::now());
            if self.expanded {
                animation::extend(ANIM_DURATION);
            }
        }
    }

    fn operate<'b>(
        &'b mut self,
        state: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation<()>,
    ) {
        self.underlay
            .as_widget_mut()
            .operate(&mut state.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        state: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        // Self-drive frames while the open animation is in flight. The
        // App-level `animation::extend` watermark is queried from
        // `subscription()` BEFORE the next `diff()` runs, so we can't
        // rely on it to start frames the same tick as the transition —
        // request the next frame here instead. Same pattern as `spinner`.
        // Close is a snap (see `diff`), so only need to drive while the
        // animator is targeting `true`.
        if let Event::Window(iced::window::Event::RedrawRequested(_)) = event {
            let anim = &state.state.downcast_ref::<AnimState>().anim;
            if anim.value && anim.in_progress(Instant::now()) {
                shell.request_redraw();
            }
        }

        self.underlay.as_widget_mut().update(
            &mut state.children[0],
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
        state: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.underlay.as_widget().mouse_interaction(
            &state.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        state: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        // Open animates (slide-in over `ANIM_DURATION`); close snaps —
        // see `diff` for the rationale. Visibility tracks `anim.value`
        // alone so the overlay unmounts the moment a close is requested.
        let now = Instant::now();
        let anim = &state.state.downcast_ref::<AnimState>().anim;
        if !anim.value {
            return self.underlay.as_widget_mut().overlay(
                &mut state.children[0],
                layout,
                renderer,
                viewport,
                translation,
            );
        }
        let progress = anim.animate_bool(0.0, 1.0, now);

        Some(overlay::Element::new(Box::new(DropDownOverlay::new(
            &mut state.children[1],
            &mut self.overlay,
            self.on_dismiss.as_ref(),
            self.width.as_ref(),
            &self.height,
            &self.alignment,
            &self.offset,
            layout.bounds(),
            layout.position() + translation,
            *viewport,
            progress,
        ))))
    }
}

impl<'a, Message, Theme: 'a, Renderer> From<DropDown<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + renderer::Renderer,
{
    fn from(drop_down: DropDown<'a, Message, Theme, Renderer>) -> Self {
        Element::new(drop_down)
    }
}

// ── Overlay ────────────────────────────────────────────────────────────────

struct DropDownOverlay<'a, 'b, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Message: Clone,
{
    state: &'b mut Tree,
    element: &'b mut Element<'a, Message, Theme, Renderer>,
    on_dismiss: Option<&'b Message>,
    width: Option<&'b Length>,
    height: &'b Length,
    alignment: &'b Alignment,
    offset: &'b Offset,
    underlay_bounds: Rectangle,
    position: Point,
    viewport: Rectangle,
    /// 0.0 = fully closed (overlay slid to its start position),
    /// 1.0 = fully open (overlay at its anchored position).
    progress: f32,
}

impl<'a, 'b, Message, Theme, Renderer> DropDownOverlay<'a, 'b, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: renderer::Renderer,
{
    #[allow(clippy::too_many_arguments)]
    fn new(
        state: &'b mut Tree,
        element: &'b mut Element<'a, Message, Theme, Renderer>,
        on_dismiss: Option<&'b Message>,
        width: Option<&'b Length>,
        height: &'b Length,
        alignment: &'b Alignment,
        offset: &'b Offset,
        underlay_bounds: Rectangle,
        position: Point,
        viewport: Rectangle,
        progress: f32,
    ) -> Self {
        DropDownOverlay {
            state,
            element,
            on_dismiss,
            width,
            height,
            alignment,
            offset,
            underlay_bounds,
            position,
            viewport,
            progress,
        }
    }
}

impl<Message, Theme, Renderer> overlay::Overlay<Message, Theme, Renderer>
    for DropDownOverlay<'_, '_, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: renderer::Renderer,
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> Node {
        let limits = Limits::new(Size::ZERO, bounds)
            .width(
                *self
                    .width
                    .unwrap_or(&Length::Fixed(self.underlay_bounds.width)),
            )
            .height(*self.height);

        let previous_position = self.position;
        let max = limits.max();

        let height_above = (previous_position.y - self.offset.y).max(0.0);
        let height_below =
            (max.height - previous_position.y - self.underlay_bounds.height - self.offset.y)
                .max(0.0);

        let ref_center_y = previous_position.y + self.underlay_bounds.height / 2.0;
        let max_height_symmetric = (ref_center_y.min(max.height - ref_center_y) * 2.0).max(0.0);

        let limits = match self.alignment {
            Alignment::Top | Alignment::AboveRight => limits.max_height(height_above),
            Alignment::TopStart | Alignment::TopEnd => {
                limits.max_height((height_above + self.underlay_bounds.height).max(0.0))
            }
            Alignment::Bottom | Alignment::BelowLeft | Alignment::BelowRight => {
                limits.max_height(height_below)
            }
            Alignment::BottomEnd | Alignment::BottomStart => {
                limits.max_height((height_below + self.underlay_bounds.height).max(0.0))
            }
            Alignment::Start | Alignment::End => limits.max_height(max_height_symmetric),
        };

        let node = self
            .element
            .as_widget_mut()
            .layout(self.state, renderer, &limits);

        let mut new_position = match self.alignment {
            Alignment::TopStart => Point::new(
                previous_position.x - node.bounds().width - self.offset.x,
                previous_position.y - node.bounds().height + self.underlay_bounds.height
                    - self.offset.y,
            ),
            Alignment::Top => Point::new(
                previous_position.x + self.underlay_bounds.width / 2.0 - node.bounds().width / 2.0,
                previous_position.y - node.bounds().height - self.offset.y,
            ),
            Alignment::TopEnd => Point::new(
                previous_position.x + self.underlay_bounds.width + self.offset.x,
                previous_position.y - node.bounds().height + self.underlay_bounds.height
                    - self.offset.y,
            ),
            Alignment::End => Point::new(
                previous_position.x + self.underlay_bounds.width + self.offset.x,
                previous_position.y + self.underlay_bounds.height / 2.0
                    - node.bounds().height / 2.0,
            ),
            Alignment::BottomEnd => Point::new(
                previous_position.x + self.underlay_bounds.width + self.offset.x,
                previous_position.y + self.offset.y,
            ),
            Alignment::Bottom => Point::new(
                previous_position.x + self.underlay_bounds.width / 2.0 - node.bounds().width / 2.0,
                previous_position.y + self.underlay_bounds.height + self.offset.y,
            ),
            Alignment::BottomStart => Point::new(
                previous_position.x - node.bounds().width - self.offset.x,
                previous_position.y + self.offset.y,
            ),
            Alignment::Start => Point::new(
                previous_position.x - node.bounds().width - self.offset.x,
                previous_position.y + self.underlay_bounds.height / 2.0
                    - node.bounds().height / 2.0,
            ),
            Alignment::BelowLeft => Point::new(
                previous_position.x + self.offset.x,
                previous_position.y + self.underlay_bounds.height + self.offset.y,
            ),
            Alignment::BelowRight => Point::new(
                previous_position.x + self.underlay_bounds.width - node.bounds().width
                    + self.offset.x,
                previous_position.y + self.underlay_bounds.height + self.offset.y,
            ),
            Alignment::AboveRight => Point::new(
                previous_position.x + self.underlay_bounds.width - node.bounds().width
                    + self.offset.x,
                previous_position.y - node.bounds().height - self.offset.y,
            ),
        };

        if new_position.x + node.bounds().width > self.viewport.width {
            new_position.x -= node.bounds().width;
        }

        if new_position.x < 0.0 {
            new_position.x = 0.0;
        }

        if new_position.y + node.bounds().height > self.viewport.height {
            new_position.y -= node.bounds().height;
        }
        if new_position.y < 0.0 {
            new_position.y = 0.0;
        }

        // Slide the panel along its open direction. Applied after
        // viewport clamping so the in-flight position can briefly cross
        // the viewport edge — that's the visual the animation wants.
        let slide_amount = SLIDE_PX * (1.0 - self.progress);
        let slide_dy = match self.alignment {
            Alignment::Top
            | Alignment::TopStart
            | Alignment::TopEnd
            | Alignment::AboveRight => slide_amount,
            Alignment::Bottom
            | Alignment::BottomEnd
            | Alignment::BottomStart
            | Alignment::BelowLeft
            | Alignment::BelowRight => -slide_amount,
            Alignment::Start | Alignment::End => 0.0,
        };
        new_position.y += slide_dy;

        node.move_to(new_position)
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
    ) {
        // Force a sub-layer for the panel content. Within a single iced
        // layer the renderer batches text and quad pipelines separately
        // (all backgrounds first, then all text), so two visible
        // overlays sharing a layer would composite as "all backgrounds,
        // then all text" — text from the lower overlay can leak above
        // the upper overlay's background. `with_layer` flushes pending
        // primitives before/after, keeping each panel's quads + text
        // composited as a unit. See CLAUDE.md → Iced Gotchas.
        let bounds = layout.bounds();
        renderer.with_layer(bounds, |renderer| {
            self.element
                .as_widget()
                .draw(self.state, renderer, theme, style, layout, cursor, &bounds);
        });
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &Renderer,
        shell: &mut Shell<Message>,
    ) {
        self.underlay_bounds = Rectangle {
            x: self.position.x,
            y: self.position.y,
            width: self.underlay_bounds.width,
            height: self.underlay_bounds.height,
        };

        if let Some(on_dismiss) = self.on_dismiss {
            match &event {
                Event::Keyboard(keyboard::Event::KeyPressed { key, .. })
                    if key == &keyboard::Key::Named(Named::Escape) =>
                {
                    shell.publish(on_dismiss.clone());
                }

                Event::Mouse(iced::mouse::Event::ButtonPressed(
                    iced::mouse::Button::Left | iced::mouse::Button::Right,
                ))
                | Event::Touch(touch::Event::FingerPressed { .. })
                    if !cursor.is_over(layout.bounds())
                        && !cursor.is_over(self.underlay_bounds) =>
                {
                    shell.publish(on_dismiss.clone());
                }

                _ => {}
            }
        }

        self.element.as_widget_mut().update(
            self.state,
            event,
            layout,
            cursor,
            renderer,
            shell,
            &layout.bounds(),
        );
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.element.as_widget().mouse_interaction(
            self.state,
            layout,
            cursor,
            &self.viewport,
            renderer,
        )
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.element
            .as_widget_mut()
            .operate(self.state, layout, renderer, operation);
    }
}
