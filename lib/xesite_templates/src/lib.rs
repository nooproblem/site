use maud::{html, Markup, PreEscaped};
use xesite_types::mastodon::{Toot, User};

pub fn talk_warning() -> Markup {
    html! {
        div.warning {
            (conv("Cadey".to_string(), "coffee".to_string(), html!{
                "So you are aware: you are reading the written version of a conference talk. This is written in a different style that is more lighthearted, conversational and different than the content normally on this blog. The words being said are the verbatim words that were spoken at the conference. The slides are the literal slides for each spoken utterance. If you want to hide the non-essential slides, please press this button: "
                (xeact_component("NoFunAllowed", serde_json::Value::Null))
            }))
        }
    }
}

pub fn slide(name: String, essential: bool) -> Markup {
    html! {
        div.hero.{@if essential {("xeblog-slides-essential")} @else {("xeblog-slides-fluff")}} {
            picture style="margin:0" {
                source type="image/avif" srcset={"https://cdn.xeiaso.net/file/christine-static/talks/" (name) ".avif"};
                source type="image/webp" srcset={"https://cdn.xeiaso.net/file/christine-static/talks/" (name) ".webp"};
                img style="padding:0" loading="lazy" src={"https://cdn.xeiaso.net/file/christine-static/talks/" (name) "-smol.png"};
            }
        }
    }
}

pub fn picture(path: String) -> Markup {
    html! {
        a href={"https://cdn.xeiaso.net/file/christine-static/" (path) ".jpg"} target="_blank" {
            picture.picture style="margin:0" {
                source type="image/avif" srcset={"https://cdn.xeiaso.net/file/christine-static/" (path) ".avif"};
                source type="image/webp" srcset={"https://cdn.xeiaso.net/file/christine-static/" (path) ".webp"};
                img.picture style="padding:0" loading="lazy" alt={"hero image " (path)} src={"https://cdn.xeiaso.net/file/christine-static/" (path) "-smol.png"};
            }
        }
    }
}

pub fn hero(file: String, prompt: Option<String>, ai: Option<String>) -> Markup {
    let ai = ai.unwrap_or("MidJourney".to_string());
    html! {
        meta property="og:image" content={"https://cdn.xeiaso.net/file/christine-static/hero/" (file) "-smol.png"};
        figure.hero style="margin:0" {
            picture style="margin:0" {
                source type="image/avif" srcset={"https://cdn.xeiaso.net/file/christine-static/hero/" (file) ".avif"};
                source type="image/webp" srcset={"https://cdn.xeiaso.net/file/christine-static/hero/" (file) ".webp"};
                img style="padding:0" loading="lazy" alt={"hero image " (file)} src={"https://cdn.xeiaso.net/file/christine-static/hero/" (file) "-smol.png"};
            }
            figcaption {
                (ai)
                @if let Some(prompt) = prompt { " -- " (prompt) }
            }
        }
    }
}

pub fn conv(name: String, mood: String, body: Markup) -> Markup {
    let name_lower = name.clone().to_lowercase();
    let name = name.replace("_", " ");

    html! {
        .conversation {
            ."conversation-standalone" {
                picture {
                    source type="image/avif" srcset={"https://cdn.xeiaso.net/file/christine-static/stickers/" (name_lower) "/" (mood) ".avif"};
                    source type="image/webp" srcset={"https://cdn.xeiaso.net/file/christine-static/stickers/" (name_lower) "/" (mood) ".webp"};
                    img style="max-height:4.5rem" alt={(name) " is " (mood)} loading="lazy" src={"https://cdn.xeiaso.net/file/christine-static/stickers/" (name_lower) "/" (mood) ".png"};
                }
            }
            ."conversation-chat" {
                "<"
                a href={"/characters#" (name_lower)} { b { (name) } }
                "> "
                (body)
            }
        }
    }
}

pub fn sticker(name: String, mood: String) -> Markup {
    let name_lower = name.to_lowercase();
    html! {
        center {
            picture {
                source type="image/avif" srcset={"https://cdn.xeiaso.net/file/christine-static/stickers/" (name_lower) "/" (mood) ".avif"};
                source type="image/webp" srcset={"https://cdn.xeiaso.net/file/christine-static/stickers/" (name_lower) "/" (mood) ".webp"};
                img alt={(name) " is " (mood)} src={"https://cdn.xeiaso.net/file/christine-static/stickers/" (name_lower) "/" (mood) ".png"};
            }
        }
    }
}

pub fn video(path: String) -> Markup {
    xeact_component("Video", serde_json::json!({"path": path}))
}

pub fn advertiser_nag(nag: Option<Markup>) -> Markup {
    html! {
        script async src="https://media.ethicalads.io/media/client/ethicalads.min.js" { "" }
        div.adaptive data-ea-publisher="christinewebsite" data-ea-type="text" data-ea-style="fixedfooter" {
            .warning {
                @if let Some(nag) = nag {
                    (nag)
                } @else {
                    (conv(
                        "Cadey".into(),
                        "coffee".into(),
                        html! {
                            "Hello! Thank you for visiting my website. You seem to be using an ad-blocker. I understand why you do this, but I'd really appreciate if it you would turn it off for my website. These ads help pay for running the website and are done by "
                            a href="https://www.ethicalads.io/" { "Ethical Ads" }
                            ". I do not receive detailed analytics on the ads and from what I understand neither does Ethical Ads. If you don't want to disable your ad blocker, please consider donating on "
                            a href="https://www.patreon.com/cadey" { "Patreon" }
                            " or sending some extra cash to "
                            code { "xeiaso.eth" }
                            " or "
                            code { "0xeA223Ca8968Ca59e0Bc79Ba331c2F6f636A3fB82" }
                            ". It helps fund the website's hosting bills and pay for the expensive technical editor that I use for my longer articles. Thanks and be well!"
                        },
                    ))
                }
            }
        }
    }
}

pub fn toot_embed(u: User, t: Toot) -> Markup {
    let content = html! {
        (PreEscaped::<String>(t.content))

        @for att in &t.attachment {
            @if att.media_type.starts_with("image/") {
                a href=(att.url) {
                    img width="100%" height="100%" src=(att.url) alt=(att.name.clone().unwrap_or("no description provided".into()));
                }
            }

            @if att.media_type.starts_with("video/") {
                video width="100%" height="100%" controls {
                    source src=(att.url) type=(att.media_type);
                    "Your browser does not support the video tag, see this URL: "
                        a href=(att.url) {(att.url)}
                }
            }
        }
        @if t.attachment.len() != 0 {
            br;
        }

        a href=(t.url.unwrap_or(t.id)) { "Link" }
    };
    html! {
        .media {
            .media-left {
                .avatarholder {
                    img src=(u.icon.url) alt={"the profile picture for " (u.preferred_username)};
                }
            }
            .media-body {
                .media-heading {
                    (u.name.replace(":verified:", ""))
                    @if u.id == "https://pony.social/users/cadey" {
                        img.verified src="https://cdn.xeiaso.net/file/christine-static/blog/verified.png";
                    }
                    " "
                    a href=(u.url) {"@" (u.preferred_username)}
                    br;
                    (t.published.format("M%m %d %Y %H:%M (UTC)").to_string())
                }
                .media-content {
                    @if let Some(warning) = t.summary {
                        details {
                            summary { "Content warning: " (warning) }
                            (content)
                        }
                    } @else {
                        (content)
                    }
                }
            }
        }
    }
}

pub fn xeact_component(name: &str, data: serde_json::Value) -> Markup {
    let uuid = uuid::Uuid::new_v4();
    let uuid = format!("{uuid}").replace("-", "");

    let script = PreEscaped(format!(
        r#"
<script type="module">
import Component from "/static/xeact/{name}.js?cacheBuster={uuid}";

const g = (name) => document.getElementById(name);
const x = (elem) => {{
    while (elem.lastChild) {{
        elem.removeChild(elem.lastChild);
    }}
}};

const root = g("{uuid}");
x(g);

root.appendChild(Component({data}))
</script>
"#,
        data=serde_json::to_string(&data).unwrap(),
    ));
    
    html! {
        div id=(uuid) {
            noscript {
                div.warning {
                    (conv("Aoi".into(), "coffee".into(), PreEscaped("This dynamic component requires JavaScript to function, sorry!".to_string())))
                }
            }
        }
        (script)
    }
}
