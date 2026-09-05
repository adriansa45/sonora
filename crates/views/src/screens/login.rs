use gpui::prelude::*;
use gpui::{
    ClipboardItem, Context, Entity, FontWeight, IntoElement, Pixels, Render, SharedString, Window,
    div, px, svg,
};
use i18n::t;
use music::{SignIn, SignInPrompt};
use state::{Session, SessionState, Sonora, Usage};
use ui::ActiveTheme as _;
use ui::{Button, Checkbox, TabBar, Text};

const COLUMN: Pixels = px(280.);
const LOGO: Pixels = px(48.);

struct Column {
    slug: &'static str,
    name: &'static str,
    options: Vec<SignIn>,
    disabled: bool,
    cancel: bool,
}

pub struct LoginView {
    session: Entity<Session>,
    usage: Entity<Usage>,
    tab: usize,
}

impl LoginView {
    pub fn new(session: Entity<Session>, cx: &mut Context<Self>) -> Self {
        cx.observe(&session, |_, _, cx| cx.notify()).detach();
        let usage = Sonora::global(cx).usage.clone();
        cx.observe(&usage, |_, _, cx| cx.notify()).detach();
        Self {
            session,
            usage,
            tab: 0,
        }
    }

    fn acted(&self, cx: &mut Context<Self>) {
        self.usage.update(cx, |usage, cx| usage.report(cx));
    }

    fn consent(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let checked = self.usage.read(cx).consented();
        Checkbox::new("usage-consent", checked)
            .label(t!("login-usage-consent"))
            .max_w(COLUMN)
            .text_color(cx.theme().muted_foreground)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.usage
                    .update(cx, |usage, cx| usage.consent(!checked, cx));
            }))
    }

    fn abandon(&mut self, cx: &mut Context<Self>) {
        self.acted(cx);
        self.session
            .update(cx, |session, cx| session.cancel_sign_in(cx));
    }

    fn start(&self, slug: &'static str, method: SignIn, cx: &mut Context<Self>) {
        self.acted(cx);
        self.session
            .update(cx, |session, cx| session.sign_in(slug, method, cx));
    }

    fn option_button(
        &self,
        slug: &'static str,
        provider: &str,
        method: &SignIn,
        disabled: bool,
        cx: &mut Context<Self>,
    ) -> Button {
        let (id, label) = match method {
            SignIn::Default => (
                format!("sign-in-{slug}"),
                t!("login-sign-in", provider = provider),
            ),
            SignIn::Anonymous => (
                format!("sign-in-{slug}-guest"),
                t!("login-use", provider = provider),
            ),
            SignIn::Path(_) => (
                format!("sign-in-{slug}-path"),
                t!("login-sign-in", provider = provider),
            ),
        };
        let primary = matches!(method, SignIn::Default | SignIn::Anonymous);
        let method = method.clone();
        let button = Button::new(SharedString::from(id))
            .label(label)
            .w_full()
            .disabled(disabled)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.start(slug, method.clone(), cx);
            }));
        match primary {
            true => button.primary(),
            false => button.outline(),
        }
    }

    fn column(&self, column: Column, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let Column {
            slug,
            name,
            options,
            disabled,
            cancel,
        } = column;
        let options: Vec<&SignIn> = options
            .iter()
            .filter(|option| !matches!(option, SignIn::Anonymous))
            .collect();

        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_3()
            .w(COLUMN)
            .child(
                svg()
                    .path(icons::path(crate::shared::provider_logo(slug)))
                    .size(LOGO)
                    .flex_none()
                    .text_color(theme.foreground),
            )
            .child(
                div()
                    .text_size(theme.text(Text::Large))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(SharedString::from(name.to_string())),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .w_full()
                    .children(
                        options
                            .into_iter()
                            .map(|method| self.option_button(slug, name, method, disabled, cx)),
                    )
                    .when(cancel, |this| {
                        this.child(
                            Button::new("cancel-sign-in")
                                .label(t!("common-cancel"))
                                .outline()
                                .w_full()
                                .on_click(cx.listener(|this, _, _, cx| this.abandon(cx))),
                        )
                    }),
            )
    }

    fn guest_mode(&self, slug: &'static str, pending: bool, cx: &mut Context<Self>) -> Button {
        Button::new("guest-mode")
            .label(t!("login-guest-title"))
            .outline()
            .w_full()
            .disabled(pending)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.start(slug, SignIn::Anonymous, cx);
            }))
    }

    fn code_prompt(&self, code: String, url: String, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let opened = url.clone();
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_size(theme.text(Text::Small))
                    .text_color(theme.muted_foreground)
                    .child(t!("login-device-code", url = &url)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(theme.text(Text::Title))
                            .font_weight(FontWeight::BOLD)
                            .child(SharedString::from(code.clone())),
                    )
                    .child(
                        Button::new("copy-code")
                            .icon("icons/copy.svg")
                            .ghost()
                            .small()
                            .on_click(move |_, _, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(code.clone()));
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
    }

    fn url_prompt(&self, url: String) -> impl IntoElement {
        Button::new("copy-login-url")
            .icon("icons/copy.svg")
            .label(t!("menu-copy-link"))
            .outline()
            .small()
            .on_click(move |_, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(url.clone()));
            })
    }
}

impl Render for LoginView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.session.read(cx).state().clone();
        let pending = self.session.read(cx).is_pending();
        let providers: Vec<state::ProviderInfo> = self.session.read(cx).providers().collect();
        let guest = providers
            .iter()
            .filter(|info| {
                info.options
                    .iter()
                    .any(|option| matches!(option, SignIn::Anonymous))
            })
            .map(|info| info.slug)
            .next();
        let waiting = matches!(state, SessionState::Authorizing(_));
        let tabs = providers
            .iter()
            .enumerate()
            .map(|(index, info)| {
                Button::new(SharedString::from(format!("login-tab-{}", info.slug)))
                    .label(SharedString::from(info.name))
                    .small()
                    .ghost()
                    .selected(index == self.tab)
                    .flex_1()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.acted(cx);
                        this.tab = index;
                        cx.notify();
                    }))
            })
            .collect::<Vec<_>>();
        let column = providers.into_iter().nth(self.tab).map(|info| Column {
            slug: info.slug,
            name: info.name,
            options: info.options,
            disabled: pending,
            cancel: waiting && info.pending,
        });

        let failure = match &state {
            SessionState::Failed(failure) => Some(failure.clone()),
            _ => None,
        };

        let status = match &state {
            SessionState::SignedOut => t!("login-signed-out"),
            SessionState::Restoring => t!("login-restoring"),
            SessionState::Authorizing(_) => t!("login-authorizing"),
            SessionState::SignedIn(profile) => t!("login-signed-in", name = &profile.display_name),
            SessionState::Failed(_) => t!("login-signed-out"),
        };

        let prompt = match &state {
            SessionState::Authorizing(prompt) => prompt.clone(),
            _ => None,
        };
        let code = match prompt {
            Some(SignInPrompt::Code { code, url }) => Some((code, url)),
            _ => None,
        };
        let url = match &state {
            SessionState::Authorizing(Some(SignInPrompt::Url(url))) => Some(url.clone()),
            _ => None,
        };

        let theme = *cx.theme();
        let asking = self.usage.read(cx).asking();
        let orphan = asking && guest.is_none();

        div()
            .relative()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_6()
            .size_full()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .child("Sonora")
                            .text_size(theme.text(Text::Display))
                            .font_weight(FontWeight::BOLD),
                    )
                    .child(
                        div()
                            .max_w(px(560.))
                            .text_center()
                            .text_size(theme.text(Text::Body))
                            .text_color(theme.muted_foreground)
                            .child(status),
                    ),
            )
            .when_some(failure, |this, failure| {
                this.child(crate::shared::trouble::trouble(failure, true))
            })
            .when_some(code, |this, (code, url)| {
                this.child(self.code_prompt(code, url, cx).into_any_element())
            })
            .when_some(url, |this, url| this.child(self.url_prompt(url)))
            .child(TabBar::new().w(COLUMN).items(tabs))
            .when_some(column, |this, column| this.child(self.column(column, cx)))
            .when_some(guest, |this, slug| {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_2()
                        .w(COLUMN)
                        .child(self.guest_mode(slug, pending, cx))
                        .child(
                            div()
                                .text_center()
                                .text_size(theme.text(Text::Small))
                                .text_color(theme.muted_foreground)
                                .child(t!("login-guest-detail")),
                        )
                        .when(asking, |this| this.child(self.consent(cx))),
                )
            })
            .when(orphan, |this| this.child(self.consent(cx)))
    }
}
