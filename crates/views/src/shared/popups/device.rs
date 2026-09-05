use std::rc::Rc;

use gpui::prelude::*;
use gpui::{App, ClipboardItem, FontWeight, RenderOnce, SharedString, Window, div, px};
use i18n::t;
use ui::{ActiveTheme as _, Button, Modal, Text};

type Cancel = Rc<dyn Fn(&(), &mut Window, &mut App)>;

#[derive(IntoElement)]
pub(crate) struct DevicePrompt {
    code: String,
    url: String,
    cancel: Option<Cancel>,
}

impl DevicePrompt {
    pub(crate) fn new(code: String, url: String) -> Self {
        Self {
            code,
            url,
            cancel: None,
        }
    }

    pub(crate) fn on_cancel(
        mut self,
        handler: impl Fn(&(), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.cancel = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for DevicePrompt {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self { code, url, cancel } = self;
        let dismissed = cancel.clone();
        let theme = *cx.theme();
        let opened = url.clone();
        let copied = code.clone();

        Modal::new("youtube-device-prompt", t!("login-authorizing"))
            .w(px(440.))
            .detail(t!("login-device-code", url = &url))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(theme.text(Text::Title))
                            .font_weight(FontWeight::BOLD)
                            .child(SharedString::from(code)),
                    )
                    .child(
                        Button::new("copy-device-code")
                            .icon("icons/copy.svg")
                            .ghost()
                            .small()
                            .on_click(move |_, _, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(copied.clone()));
                            }),
                    ),
            )
            .child(
                Button::new("open-device-authorization")
                    .label(SharedString::from(url))
                    .icon("icons/external-link.svg")
                    .outline()
                    .on_click(move |_, _, cx| cx.open_url(&opened)),
            )
            .action(
                Button::new("cancel-device-authorization")
                    .ghost()
                    .label(t!("common-cancel"))
                    .on_click(move |_, window, cx| {
                        if let Some(cancel) = &cancel {
                            cancel(&(), window, cx);
                        }
                    }),
            )
            .on_dismiss(move |_, window, cx| {
                if let Some(dismissed) = &dismissed {
                    dismissed(&(), window, cx);
                }
            })
    }
}
