use std::sync::Arc;

use crate::{
    app::Core,
    cosmic_config::CosmicConfigEntry,
    cosmic_theme::util::CssColor,
    iced::{
        self,
        alignment::{Horizontal, Vertical},
        widget::Container,
        window, Color, Length, Limits, Rectangle,
    },
    iced_style, iced_widget, sctk,
    theme::{self, Button, THEME},
    Application, Element, Renderer,
};
pub use cosmic_panel_config;
use cosmic_panel_config::{CosmicPanelBackground, PanelAnchor, PanelSize};
use iced_style::{button::StyleSheet, container::Appearance};
use iced_widget::runtime::command::platform_specific::wayland::popup::{
    SctkPopupSettings, SctkPositioner,
};
use sctk::reexports::protocols::xdg::shell::client::xdg_positioner::{Anchor, Gravity};
use tracing::error;

use super::cosmic;

const APPLET_PADDING: u32 = 8;

#[must_use]
pub fn applet_button_theme() -> Button {
    Button::Custom {
        active: Box::new(|t| iced_style::button::Appearance {
            border_radius: 0.0.into(),
            ..t.active(&Button::Text)
        }),
        hover: Box::new(|t| iced_style::button::Appearance {
            border_radius: 0.0.into(),
            ..t.hovered(&Button::Text)
        }),
    }
}

#[derive(Debug, Clone)]
pub struct CosmicAppletHelper {
    pub size: Size,
    pub anchor: PanelAnchor,
    pub background: CosmicPanelBackground,
    pub output_name: String,
}

#[derive(Clone, Debug)]
pub enum Size {
    PanelSize(PanelSize),
    // (width, height)
    Hardcoded((u16, u16)),
}

impl Default for CosmicAppletHelper {
    fn default() -> Self {
        Self {
            size: Size::PanelSize(
                std::env::var("COSMIC_PANEL_SIZE")
                    .ok()
                    .and_then(|size| ron::from_str(size.as_str()).ok())
                    .unwrap_or(PanelSize::S),
            ),
            anchor: std::env::var("COSMIC_PANEL_ANCHOR")
                .ok()
                .and_then(|size| ron::from_str(size.as_str()).ok())
                .unwrap_or(PanelAnchor::Top),
            background: std::env::var("COSMIC_PANEL_BACKGROUND")
                .ok()
                .and_then(|size| ron::from_str(size.as_str()).ok())
                .unwrap_or(CosmicPanelBackground::ThemeDefault),
            output_name: std::env::var("COSMIC_PANEL_OUTPUT").unwrap_or_default(),
        }
    }
}

impl CosmicAppletHelper {
    #[must_use]
    pub fn suggested_size(&self) -> (u16, u16) {
        match &self.size {
            Size::PanelSize(size) => match size {
                PanelSize::XL => (64, 64),
                PanelSize::L => (36, 36),
                PanelSize::M => (24, 24),
                PanelSize::S => (16, 16),
                PanelSize::XS => (12, 12),
            },
            Size::Hardcoded((width, height)) => (*width, *height),
        }
    }

    // Set the default window size. Helper for application init with hardcoded size.
    pub fn window_size(&mut self, width: u16, height: u16) {
        self.size = Size::Hardcoded((width, height));
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn window_settings(&self) -> super::Settings {
        let (width, height) = self.suggested_size();
        let width = u32::from(width);
        let height = u32::from(height);
        let mut settings = super::Settings::default()
            .size((width + APPLET_PADDING * 2, height + APPLET_PADDING * 2))
            .size_limits(
                Limits::NONE
                    .min_height(height as f32 + APPLET_PADDING as f32 * 2.0)
                    .max_height(height as f32 + APPLET_PADDING as f32 * 2.0)
                    .min_width(width as f32 + APPLET_PADDING as f32 * 2.0)
                    .max_width(width as f32 + APPLET_PADDING as f32 * 2.0),
            )
            .resizable(None)
            .default_text_size(18.0)
            .default_font(crate::font::FONT)
            .transparent(true);
        if let Some(theme) = self.theme() {
            settings = settings.theme(theme);
        }
        settings
    }

    #[must_use]
    pub fn icon_button<'a, Message: 'static>(
        &self,
        icon_name: &'a str,
    ) -> iced::widget::Button<'a, Message, Renderer> {
        crate::widget::button(theme::Button::Text)
            .icon(theme::Svg::Symbolic, icon_name, self.suggested_size().0)
            .padding(8)
    }

    // TODO popup container which tracks the size of itself and requests the popup to resize to match
    pub fn popup_container<'a, Message: 'static>(
        &self,
        content: impl Into<Element<'a, Message>>,
    ) -> Container<'a, Message, Renderer> {
        let (vertical_align, horizontal_align) = match self.anchor {
            PanelAnchor::Left => (Vertical::Center, Horizontal::Left),
            PanelAnchor::Right => (Vertical::Center, Horizontal::Right),
            PanelAnchor::Top => (Vertical::Top, Horizontal::Center),
            PanelAnchor::Bottom => (Vertical::Bottom, Horizontal::Center),
        };

        Container::<Message, Renderer>::new(Container::<Message, Renderer>::new(content).style(
            theme::Container::custom(|theme| Appearance {
                text_color: Some(theme.cosmic().background.on.into()),
                background: Some(Color::from(theme.cosmic().background.base).into()),
                border_radius: 12.0.into(),
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            }),
        ))
        .width(Length::Shrink)
        .height(Length::Shrink)
        .align_x(horizontal_align)
        .align_y(vertical_align)
    }

    #[must_use]
    #[allow(clippy::cast_possible_wrap)]
    pub fn get_popup_settings(
        &self,
        parent: window::Id,
        id: window::Id,
        size: Option<(u32, u32)>,
        width_padding: Option<i32>,
        height_padding: Option<i32>,
    ) -> SctkPopupSettings {
        let (width, height) = self.suggested_size();
        let pixel_offset = 8;
        let (offset, anchor, gravity) = match self.anchor {
            PanelAnchor::Left => ((pixel_offset, 0), Anchor::Right, Gravity::Right),
            PanelAnchor::Right => ((-pixel_offset, 0), Anchor::Left, Gravity::Left),
            PanelAnchor::Top => ((0, pixel_offset), Anchor::Bottom, Gravity::Bottom),
            PanelAnchor::Bottom => ((0, -pixel_offset), Anchor::Top, Gravity::Top),
        };
        SctkPopupSettings {
            parent,
            id,
            positioner: SctkPositioner {
                anchor,
                gravity,
                offset,
                size,
                anchor_rect: Rectangle {
                    x: 0,
                    y: 0,
                    width: width_padding.unwrap_or(APPLET_PADDING as i32) * 2 + i32::from(width),
                    height: height_padding.unwrap_or(APPLET_PADDING as i32) * 2 + i32::from(height),
                },
                reactive: true,
                constraint_adjustment: 15, // slide_y, slide_x, flip_x, flip_y
                ..Default::default()
            },
            parent_size: None,
            grab: true,
        }
    }

    #[must_use]
    pub fn theme(&self) -> Option<theme::Theme> {
        match self.background {
            CosmicPanelBackground::Dark => Some(theme::Theme::dark()),
            CosmicPanelBackground::Light => Some(theme::Theme::light()),
            _ => None,
        }
    }
}

/// Launch the application with the given settings.
///
/// # Errors
///
/// Returns error on application failure.
pub fn run<App: Application>(autosize: bool, flags: App::Flags) -> iced::Result {
    let helper = CosmicAppletHelper::default();
    let mut settings = helper.window_settings();
    settings.autosize = autosize;
    if autosize {
        settings.size_limits = Limits::NONE;
    }

    if let Some(icon_theme) = settings.default_icon_theme {
        crate::icon_theme::set_default(icon_theme);
    }

    let mut core = Core::default();
    core.window.show_window_menu = false;
    core.window.show_headerbar = false;
    core.window.sharp_corners = true;
    core.window.show_maximize = false;
    core.window.show_minimize = false;
    core.window.use_template = false;

    core.debug = settings.debug;
    core.set_scale_factor(settings.scale_factor);
    core.set_window_width(settings.size.0);
    core.set_window_height(settings.size.1);

    THEME.with(move |t| {
        let mut cosmic_theme = t.borrow_mut();
        cosmic_theme.set_theme(settings.theme.theme_type);
    });

    let mut iced = iced::Settings::with_flags((core, flags));

    iced.antialiasing = settings.antialiasing;
    iced.default_font = settings.default_font;
    iced.default_text_size = settings.default_text_size;
    iced.id = Some(App::APP_ID.to_owned());

    {
        use iced::wayland::actions::window::SctkWindowSettings;
        use iced_sctk::settings::InitialSurface;
        iced.initial_surface = InitialSurface::XdgWindow(SctkWindowSettings {
            app_id: Some(App::APP_ID.to_owned()),
            autosize: settings.autosize,
            client_decorations: settings.client_decorations,
            resizable: settings.resizable,
            size: settings.size,
            size_limits: settings.size_limits,
            title: None,
            transparent: settings.transparent,
            ..SctkWindowSettings::default()
        });
    }

    <cosmic::Cosmic<App> as iced::Application>::run(iced)
}

#[must_use]
pub fn style() -> <crate::Theme as iced_style::application::StyleSheet>::Style {
    <crate::Theme as iced_style::application::StyleSheet>::Style::Custom(Box::new(|theme| {
        iced_style::application::Appearance {
            background_color: Color::from_rgba(0.0, 0.0, 0.0, 0.0),
            text_color: theme.cosmic().on_bg_color().into(),
        }
    }))
}
