use livid::{document::Document, enums::WidgetType::*, widget::Widget};
use std::option::Option as StdOption;
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{MessageEvent, Response, WebSocket};

#[wasm_bindgen]
extern "C" {
    fn setInterval(closure: &Closure<dyn FnMut()>, millis: u32) -> f64;
}

fn recv_ws_msg(e: MessageEvent) -> StdOption<()> {
    if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
        let string = ToString::to_string(&txt);
        let num: usize = string.parse().ok()?;

        if num < 10000 {
            let secs = Widget::from_id("secs")?;
            secs.set_text_content(Some(&string));
        } else {
            let clock = Widget::from_id("clock")?;
            clock.set_text_content(Some(&string));
            let secs = Widget::from_id("secs")?;
            secs.set_text_content(Some("0"));
        }
    }
    Some(())
}

fn tick_secs() -> StdOption<()> {
    let secs = Widget::from_id("secs")?;
    let n: usize = secs.text_content()?.parse().ok()?;
    secs.set_text_content(Some(&format!("{}", n + 1)));
    Some(())
}

fn main() {
    Document::get().set_title("Block Height");
    Document::add_css_link("/main.css");
    let main = Widget::new(Main);
    let clock = Widget::new(Span);
    clock.set_id("clock");
    main.append(&clock);

    let secs = Widget::new(Span);
    secs.set_id("secs");
    main.append(&secs);

    spawn_local(async move {
        let window = web_sys::window().unwrap();
        if let Ok(resp_value) = JsFuture::from(window.fetch_with_str("/blocks")).await {
            let resp: Response = resp_value.dyn_into().unwrap();
            let text = JsFuture::from(resp.text().unwrap()).await.unwrap();
            let w = Widget::from_id("clock").unwrap();
            w.set_text_content(Some(&format!("{}", text.as_string().unwrap())));
        }

        let ws = WebSocket::new("ws://blocks.mag.wiki:3012").unwrap();
        let onmessage_callback = Closure::wrap(
            Box::new(move |e| recv_ws_msg(e).unwrap_or(())) as Box<dyn FnMut(MessageEvent)>
        );
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        let tick_secs = Closure::new(|| tick_secs().unwrap_or(()));

        setInterval(&tick_secs, 1000);

        tick_secs.forget();
    });
}
