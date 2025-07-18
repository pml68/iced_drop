//! Encapsulates a widget that can be dragged and dropped.
use std::fmt::Debug;
use std::vec;

use iced::advanced::widget::{Operation, Tree, Widget};
use iced::advanced::{self, Layout, layout, mouse, overlay, renderer};
use iced::{Element, Pixels, Point, Rectangle, Size, Vector};

/// An element that can be dragged and dropped on a [`DropZone`]
pub struct Droppable<
    'a,
    Message,
    Theme = iced::Theme,
    Renderer = iced::Renderer,
> where
    Message: Clone,
    Renderer: renderer::Renderer,
{
    content: Element<'a, Message, Theme, Renderer>,
    id: Option<iced::advanced::widget::Id>,
    drag_threshold: f32,
    on_press: Option<Message>,
    on_click: Option<Message>,
    on_drop: Option<Box<dyn Fn(Point, Rectangle) -> Message + 'a>>,
    on_drag: Option<Box<dyn Fn(Point, Rectangle) -> Message + 'a>>,
    on_cancel: Option<Message>,
    drag_mode: Option<(bool, bool)>,
    drag_overlay: bool,
    drag_hide: bool,
    drag_center: bool,
    drag_size: Option<Size>,
    reset_delay: usize,
    status: Option<Status>,
}

impl<'a, Message, Theme, Renderer> Droppable<'a, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: renderer::Renderer,
{
    /// Creates a new [`Droppable`].
    pub fn new(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        Self {
            content: content.into(),
            id: None,
            drag_threshold: 5.0,
            on_press: None,
            on_click: None,
            on_drop: None,
            on_drag: None,
            on_cancel: None,
            drag_mode: Some((true, true)),
            drag_overlay: true,
            drag_hide: false,
            drag_center: false,
            drag_size: None,
            reset_delay: 0,
            status: None,
        }
    }

    /// Sets the unique identifier of the [`Droppable`].
    pub fn id(mut self, id: iced::advanced::widget::Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Sets the drag threshold of the [`Droppable`].
    ///
    /// This controls when [`on_press`] will be triggered (if set) after selecting a droppable and
    /// moving the cursor.
    ///
    /// [`on_press`]: Droppable::on_press
    pub fn drag_threshold(mut self, drag_threshold: impl Into<Pixels>) -> Self {
        self.drag_threshold = drag_threshold.into().0;
        self
    }

    /// Sets the message that will be produced when the [`Droppable`] is pressed, but not dragged.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Sets the message that will be produced when the [`Droppable`] is clicked.
    pub fn on_click(mut self, message: Message) -> Self {
        self.on_click = Some(message);
        self
    }

    /// Sets the message that will be produced when the [`Droppable`] is dropped on a [`DropZone`].
    ///
    /// Unless this is set, the [`Droppable`] will be disabled.
    pub fn on_drop<F>(mut self, message: F) -> Self
    where
        F: Fn(Point, Rectangle) -> Message + 'a,
    {
        self.on_drop = Some(Box::new(message));
        self
    }

    /// Sets the message that will be produced when the [`Droppable`] is dragged.
    pub fn on_drag<F>(mut self, message: F) -> Self
    where
        F: Fn(Point, Rectangle) -> Message + 'a,
    {
        self.on_drag = Some(Box::new(message));
        self
    }

    /// Sets the message that will be produced when the user right clicks while dragging the [`Droppable`].
    pub fn on_cancel(mut self, message: Message) -> Self {
        self.on_cancel = Some(message);
        self
    }

    /// Sets whether the [`Droppable`] should be drawn under the cursor while dragging.
    pub fn drag_overlay(mut self, drag_overlay: bool) -> Self {
        self.drag_overlay = drag_overlay;
        self
    }

    /// Sets whether the [`Droppable`] should be hidden while dragging.
    pub fn drag_hide(mut self, drag_hide: bool) -> Self {
        self.drag_hide = drag_hide;
        self
    }

    /// Sets whether the [`Droppable`] should be centered on the cursor while dragging.
    pub fn drag_center(mut self, drag_center: bool) -> Self {
        self.drag_center = drag_center;
        self
    }

    // Sets whether the [`Droppable`] can be dragged along individual axes.
    pub fn drag_mode(mut self, drag_x: bool, drag_y: bool) -> Self {
        self.drag_mode = Some((drag_x, drag_y));
        self
    }

    /// Sets whether the [`Droppable`] should be be resized to a given size while dragging.
    pub fn drag_size(mut self, hide_size: Size) -> Self {
        self.drag_size = Some(hide_size);
        self
    }

    /// Sets the number of frames/layout calls to wait before resetting the size of the [`Droppable`] after dropping.
    ///
    /// This is useful for cases where the [`Droppable`] is being moved to a new location after some widget operation.
    /// In this case, the [`Droppable`] will mainting the 'drag_size' for the given number of frames before resetting to its original size.
    /// This prevents the [`Droppable`] from 'jumping' back to its original size before the new location is rendered which
    /// prevents flickering.
    ///
    /// Warning: this should only be set if there's is some noticeble flickering when the [`Droppable`] is dropped. That is, if the
    /// [`Droppable`] returns to its original size before it's moved to it's new location.
    pub fn reset_delay(mut self, reset_delay: usize) -> Self {
        self.reset_delay = reset_delay;
        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Droppable<'a, Message, Theme, Renderer>
where
    Message: Clone,
    Renderer: renderer::Renderer,
{
    fn state(&self) -> iced::advanced::widget::tree::State {
        advanced::widget::tree::State::new(State::default())
    }

    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        advanced::widget::tree::Tag::of::<State>()
    }

    fn children(&self) -> Vec<iced::advanced::widget::Tree> {
        vec![advanced::widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut iced::advanced::widget::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content))
    }

    fn size(&self) -> iced::Size<iced::Length> {
        self.content.as_widget().size()
    }

    fn update(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        event: &iced::Event,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        _viewport: &iced::Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();

        if !matches!(state.action, Action::Drag(_, _)) {
            // handle the on event of the content first, in case that the droppable is nested
            self.content.as_widget_mut().update(
                &mut tree.children[0],
                event,
                layout,
                cursor,
                _renderer,
                _clipboard,
                shell,
                _viewport,
            );
            // this should really only be captured if the droppable is nested or it contains some other
            // widget that captures the event
            if shell.is_event_captured() {
                return;
            }
        }

        if let Some(on_drop) = self.on_drop.as_deref() {
            if let iced::Event::Mouse(mouse) = event {
                match mouse {
                    mouse::Event::ButtonPressed(btn) => {
                        if *btn == mouse::Button::Left
                            && cursor.is_over(layout.bounds())
                        {
                            // select the droppable and store the position of the widget before dragging
                            state.action =
                                Action::Select(cursor.position().unwrap());
                            let bounds = layout.bounds();
                            state.widget_pos = bounds.position();
                            state.overlay_bounds.width = bounds.width;
                            state.overlay_bounds.height = bounds.height;

                            if let Some(on_click) = self.on_click.clone() {
                                shell.publish(on_click);
                            }
                            shell.capture_event();
                        } else if *btn == mouse::Button::Right {
                            if let Action::Drag(_, _) = state.action {
                                shell.invalidate_layout();
                                state.action = Action::None;
                                if let Some(on_cancel) = self.on_cancel.clone()
                                {
                                    shell.publish(on_cancel);
                                }
                            }
                        }
                    }
                    &mouse::Event::CursorMoved { mut position } => {
                        let should_drag = match state.action {
                            Action::Select(start) => {
                                // Check if cursor has moved from the initial click position
                                // (in case there was a micro-movement, might happen on trackpad)
                                let distance = ((position.x - start.x).powi(2)
                                    + (position.y - start.y).powi(2))
                                .sqrt();

                                if distance >= self.drag_threshold {
                                    state.action =
                                        Action::Drag(start, position);
                                    true
                                } else {
                                    false
                                }
                            }
                            Action::Drag(start, _) => {
                                state.action = Action::Drag(start, position);
                                true
                            }
                            _ => false,
                        };

                        if should_drag {
                            if let Action::Drag(start, _) = state.action {
                                // Apply drag mode constraints
                                if let Some((drag_x, drag_y)) = self.drag_mode {
                                    position = Point {
                                        x: if drag_x {
                                            position.x
                                        } else {
                                            start.x
                                        },
                                        y: if drag_y {
                                            position.y
                                        } else {
                                            start.y
                                        },
                                    };
                                }

                                // update the position of the overlay since the cursor was moved
                                if self.drag_center {
                                    state.overlay_bounds.x = position.x
                                        - state.overlay_bounds.width / 2.0;
                                    state.overlay_bounds.y = position.y
                                        - state.overlay_bounds.height / 2.0;
                                } else {
                                    state.overlay_bounds.x = state.widget_pos.x
                                        + position.x
                                        - start.x;
                                    state.overlay_bounds.y = state.widget_pos.y
                                        + position.y
                                        - start.y;
                                }

                                // Send on_drag message
                                if let Some(on_drag) = self.on_drag.as_deref() {
                                    let message = (on_drag)(
                                        position,
                                        state.overlay_bounds,
                                    );
                                    shell.publish(message);
                                }

                                shell.request_redraw();
                            }
                        }
                    }
                    mouse::Event::ButtonReleased(mouse::Button::Left) => {
                        match state.action {
                            Action::Select(_) => {
                                if let Some(on_press) = self.on_press.clone() {
                                    shell.publish(on_press);
                                }
                                state.action = Action::None;
                            }
                            Action::Drag(_, current) => {
                                // send on drop msg
                                let message =
                                    (on_drop)(current, state.overlay_bounds);
                                shell.publish(message);

                                if self.reset_delay == 0 {
                                    state.action = Action::None;
                                } else {
                                    state.action =
                                        Action::Wait(self.reset_delay);
                                }
                            }
                            _ => (),
                        }
                    }
                    _ => {}
                }
            }
        }

        let current_status = if cursor.is_over(layout.bounds()) {
            if self.on_drop.is_none() {
                Status::Disabled
            } else {
                if let Action::Drag(_, _) = state.action {
                    Status::Dragged
                } else {
                    Status::Hovered
                }
            }
        } else {
            Status::Active
        };

        if let iced::Event::Window(iced::window::Event::RedrawRequested(_now)) =
            event
        {
            self.status = Some(current_status);
        } else if self.status.is_some_and(|status| status != current_status) {
            shell.request_redraw();
        }
    }

    fn layout(
        &self,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        let state: &mut State = tree.state.downcast_mut::<State>();
        let content_node = self.content.as_widget().layout(
            &mut tree.children[0],
            renderer,
            limits,
        );

        // Adjust the size of the original widget if it's being dragged or we're wating to reset the size
        if let Some(new_size) = self.drag_size {
            match state.action {
                Action::Drag(_, _) => {
                    return iced::advanced::layout::Node::with_children(
                        new_size,
                        content_node.children().to_vec(),
                    );
                }
                Action::Wait(reveal_index) => {
                    if reveal_index <= 1 {
                        state.action = Action::None;
                    } else {
                        state.action = Action::Wait(reveal_index - 1);
                    }

                    return iced::advanced::layout::Node::with_children(
                        new_size,
                        content_node.children().to_vec(),
                    );
                }
                _ => (),
            }
        }

        content_node
    }

    fn operate(
        &self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let state = tree.state.downcast_mut::<State>();
        operation.custom(self.id.as_ref(), layout.bounds(), state);
        operation.container(
            self.id.as_ref(),
            layout.bounds(),
            &mut |operation| {
                self.content.as_widget().operate(
                    &mut tree.children[0],
                    layout,
                    renderer,
                    operation,
                );
            },
        );
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        let state: &State = tree.state.downcast_ref::<State>();
        if let Action::Drag(_, _) = state.action {
            if self.drag_hide {
                return;
            }
        }

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            &viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        _viewport: &iced::Rectangle,
        _translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let state: &mut State = tree.state.downcast_mut::<State>();
        if self.drag_overlay {
            if let Action::Drag(_, _) = state.action {
                return Some(overlay::Element::new(Box::new(Overlay {
                    content: &self.content,
                    tree: &mut tree.children[0],
                    overlay_bounds: state.overlay_bounds,
                })));
            }
        }
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            _viewport,
            _translation,
        )
    }

    fn mouse_interaction(
        &self,
        tree: &iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        _viewport: &iced::Rectangle,
        _renderer: &Renderer,
    ) -> iced::advanced::mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();

        if let Action::Drag(_, _) = state.action {
            return mouse::Interaction::Grabbing;
        }

        let child_interact = self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            _viewport,
            _renderer,
        );

        if child_interact != mouse::Interaction::default() {
            return child_interact;
        }

        if cursor.is_over(layout.bounds()) {
            if self.on_drop.is_some() {
                if self.on_press.is_some() {
                    mouse::Interaction::Pointer
                } else {
                    mouse::Interaction::Grab
                }
            } else {
                mouse::Interaction::NotAllowed
            }
        } else {
            mouse::Interaction::default()
        }
    }
}

impl<'a, Message, Theme, Renderer> From<Droppable<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Theme: 'a,
    Renderer: 'a + renderer::Renderer,
{
    fn from(
        droppable: Droppable<'a, Message, Theme, Renderer>,
    ) -> Element<'a, Message, Theme, Renderer> {
        Element::new(droppable)
    }
}

#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub struct State {
    widget_pos: Point,
    overlay_bounds: Rectangle,
    action: Action,
}

#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub enum Status {
    #[default]
    Active,
    Hovered,
    Dragged,
    Disabled,
}

#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub enum Action {
    #[default]
    None,
    /// (point clicked)
    Select(Point),
    /// (start pos, current pos)
    Drag(Point, Point),
    /// (frames to wait)
    Wait(usize),
}

struct Overlay<'a, 'b, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    content: &'b Element<'a, Message, Theme, Renderer>,
    tree: &'b mut advanced::widget::Tree,
    overlay_bounds: Rectangle,
}

impl<'a, 'b, Message, Theme, Renderer>
    overlay::Overlay<Message, Theme, Renderer>
    for Overlay<'a, 'b, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn layout(&mut self, renderer: &Renderer, _bounds: Size) -> layout::Node {
        Widget::<Message, Theme, Renderer>::layout(
            self.content.as_widget(),
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, self.overlay_bounds.size()),
        )
        .move_to(self.overlay_bounds.position())
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        inherited_style: &renderer::Style,
        layout: Layout<'_>,
        cursor_position: mouse::Cursor,
    ) {
        Widget::<Message, Theme, Renderer>::draw(
            self.content.as_widget(),
            self.tree,
            renderer,
            theme,
            inherited_style,
            layout,
            cursor_position,
            &Rectangle::with_size(Size::INFINITY),
        );
    }
}
