// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use crate::{theme, Element};
use apply::Apply;
use derive_setters::Setters;
use iced::{self, widget, Length};
use std::borrow::Cow;

#[must_use]
pub fn header_bar<'a, Message>() -> HeaderBar<'a, Message> {
    HeaderBar {
        title: Cow::Borrowed(""),
        on_close: None,
        on_drag: None,
        on_maximize: None,
        on_minimize: None,
        start: Vec::new(),
        center: Vec::new(),
        end: Vec::new(),
    }
}

#[derive(Setters)]
pub struct HeaderBar<'a, Message> {
    /// Defines the title of the window
    #[setters(skip)]
    title: Cow<'a, str>,

    /// A message emitted when the close button is pressed.
    #[setters(strip_option)]
    on_close: Option<Message>,

    /// A message emitted when dragged.
    #[setters(strip_option)]
    on_drag: Option<Message>,

    /// A message emitted when the maximize button is pressed.
    #[setters(strip_option)]
    on_maximize: Option<Message>,

    /// A message emitted when the minimize button is pressed.
    #[setters(strip_option)]
    on_minimize: Option<Message>,

    /// Elements packed at the start of the headerbar.
    #[setters(skip)]
    start: Vec<Element<'a, Message>>,

    /// Elements packed in the center of the headerbar.
    #[setters(skip)]
    center: Vec<Element<'a, Message>>,

    /// Elements packed at the end of the headerbar.
    #[setters(skip)]
    end: Vec<Element<'a, Message>>,
}

impl<'a, Message: Clone + 'static> HeaderBar<'a, Message> {
    /// Defines the title of the window
    #[must_use]
    pub fn title(mut self, title: impl Into<Cow<'a, str>> + 'a) -> Self {
        self.title = title.into();
        self
    }

    /// Pushes an element to the start region.
    #[must_use]
    pub fn start(mut self, widget: impl Into<Element<'a, Message>> + 'a) -> Self {
        self.start.push(widget.into());
        self
    }

    /// Pushes an element to the center region.
    #[must_use]
    pub fn center(mut self, widget: impl Into<Element<'a, Message>> + 'a) -> Self {
        self.center.push(widget.into());
        self
    }

    /// Pushes an element to the end region.
    #[must_use]
    pub fn end(mut self, widget: impl Into<Element<'a, Message>> + 'a) -> Self {
        self.end.push(widget.into());
        self
    }
}

impl<'a, Message: Clone + 'static> HeaderBar<'a, Message> {
    /// Converts the headerbar builder into an Iced element.
    pub fn into_element(mut self) -> Element<'a, Message> {
        let mut packed: Vec<Element<Message>> = Vec::with_capacity(4);

        // Take ownership of the regions to be packed.
        let start = std::mem::take(&mut self.start);
        let center = std::mem::take(&mut self.center);
        let mut end = std::mem::take(&mut self.end);

        // If elements exist in the start region, append them here.
        if !start.is_empty() {
            packed.push(
                iced::widget::row(start)
                    .align_items(iced::Alignment::Center)
                    .apply(iced::widget::container)
                    .align_x(iced::alignment::Horizontal::Left)
                    .into(),
            );
        }

        // If elements exist in the center region, use them here.
        // This will otherwise use the title as a widget if a title was defined.
        packed.push(if !center.is_empty() {
            iced::widget::row(center)
                .align_items(iced::Alignment::Center)
                .apply(iced::widget::container)
                .align_x(iced::alignment::Horizontal::Center)
                .into()
        } else if self.title.is_empty() {
            widget::horizontal_space(Length::Fill).into()
        } else {
            self.title_widget()
        });

        // Also packs the window controls at the very end.
        end.push(iced::widget::horizontal_space(Length::Fixed(12.0)).into());
        end.push(self.window_controls());
        packed.push(
            iced::widget::row(end)
                .align_items(iced::Alignment::Center)
                .apply(widget::container)
                .align_x(iced::alignment::Horizontal::Right)
                .into(),
        );

        // Creates the headerbar widget.
        let mut widget = widget::row(packed)
            .height(Length::Fixed(50.0))
            .padding(8)
            .spacing(8)
            .apply(widget::container)
            .style(crate::theme::Container::HeaderBar)
            .center_y()
            .apply(widget::mouse_area);

        // Assigns a message to emit when the headerbar is dragged.
        if let Some(message) = self.on_drag.clone() {
            widget = widget.on_press(message);
        }

        // Assigns a message to emit when the headerbar is double-clicked.
        if let Some(message) = self.on_maximize.clone() {
            widget = widget.on_release(message);
        }

        widget.into()
    }

    fn title_widget(&mut self) -> Element<'a, Message> {
        let mut title = Cow::default();
        std::mem::swap(&mut title, &mut self.title);

        super::text(title)
            .size(16)
            .font(crate::font::FONT_SEMIBOLD)
            .apply(widget::container)
            .center_x()
            .center_y()
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Creates the widget for window controls.
    fn window_controls(&mut self) -> Element<'a, Message> {
        let mut widgets: Vec<Element<_>> = Vec::with_capacity(3);

        let icon = |name, size, on_press| {
            super::icon(name, size)
                .force_svg(true)
                .style(crate::theme::Svg::SymbolicActive)
                .apply(widget::button)
                .style(theme::Button::Text)
                .on_press(on_press)
        };

        if let Some(message) = self.on_minimize.take() {
            widgets.push(icon("window-minimize-symbolic", 16, message).into());
        }

        if let Some(message) = self.on_maximize.take() {
            widgets.push(icon("window-maximize-symbolic", 16, message).into());
        }

        if let Some(message) = self.on_close.take() {
            widgets.push(icon("window-close-symbolic", 16, message).into());
        }

        widget::row(widgets)
            .spacing(8)
            .apply(widget::container)
            .height(Length::Fill)
            .center_y()
            .into()
    }
}

impl<'a, Message: Clone + 'static> From<HeaderBar<'a, Message>> for Element<'a, Message> {
    fn from(headerbar: HeaderBar<'a, Message>) -> Self {
        headerbar.into_element()
    }
}
