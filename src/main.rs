// src/main.rs
use dioxus::prelude::*;
use meval::eval_str;

fn main() {
    dioxus::launch(app);
}

#[component]
fn app() -> Element {
    let mut input  = use_signal(String::new);
    let mut result = use_signal(String::new);

    // استخدام use_callback لكل دالة تعتمد على أخرى
    let insert = use_callback(move |txt: &str| {
        let mut s = input();
        s.push_str(txt);
        input.set(s);
    });

    let insert_op = use_callback(move |op: &str| {
        let op = match op {
            "×" => "*",
            "÷" => "/",
            other => other,
        };
        insert(op);
    });

    let clear_all = use_callback(move |_| {
        input.set(String::new());
        result.set(String::new());
    });

    let backspace = use_callback(move |_| {
        let mut s = input();
        s.pop();
        input.set(s);
    });

    let toggle_sign = use_callback(move |_| {
        let s = input();
        if s.trim().is_empty() {
            input.set("-".to_string());
        } else {
            input.set(format!("(-({}))", s));
        }
    });

    let calculate = use_callback(move |_| {
        let expr = input().replace('×', "*").replace('÷', "/");
        if expr.trim().is_empty() {
            result.set(String::new());
            return;
        }
        match eval_str(&expr) {
            Ok(v) => {
                let out = if v.fract().abs() < 1e-12 {
                    format!("{}", v.round() as i64)
                } else {
                    let s = format!("{:.12}", v);
                    s.trim_end_matches('0').trim_end_matches('.').to_string()
                };
                result.set(out);
            }
            Err(_) => result.set("خطأ في الصيغة".to_string()),
        }
    });

    let on_key = move |e: KeyboardEvent| {
        match e.key() {
            Key::Character(s) => {
                if s == "," {
                    insert(".");
                } else if s.chars().all(|c| "0123456789.+-*/()%".contains(c)) {
                    insert(&s);
                }
            }
            Key::Enter     => calculate(()),
            Key::Backspace => backspace(()),
            _ => {}
        }
    };

    // الأنماط
    let root_style = "min-height:100vh;display:flex;align-items:center;justify-content:center;background:#0f172a;color:white;font-family:system-ui,sans-serif;";
    let card_style = "width:360px;background:#111827;border:1px solid #1f2937;border-radius:18px;padding:18px;box-shadow:0 10px 30px rgba(0,0,0,.35);";
    let screen_style = "background:#0b1220;border:1px solid #1f2937;border-radius:14px;padding:14px;display:flex;flex-direction:column;gap:6px;margin-bottom:12px;";
    let input_style = "font-size:18px;opacity:.9;word-break:break-all;text-align:right;min-height:24px;";
    let result_style = "font-size:28px;font-weight:700;text-align:right;word-break:break-all;min-height:34px;";
    let grid_style = "display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:10px;";

    rsx! {
        div { style: "{root_style}",
            div { style: "{card_style}", tabindex: 0, onkeydown: on_key,
                div { style: "{screen_style}",
                    div { style: "{input_style}", "{input}" }
                    div { style: "{result_style}", "{result}" }
                }
                div { style: "{grid_style}",
                    button { onclick: move |_| clear_all(()), "C" }
                    button { onclick: move |_| insert("("), "(" }
                    button { onclick: move |_| insert(")"), ")" }
                    button { onclick: move |_| insert_op("÷"), "÷" }

                    button { onclick: move |_| insert("7"), "7" }
                    button { onclick: move |_| insert("8"), "8" }
                    button { onclick: move |_| insert("9"), "9" }
                    button { onclick: move |_| insert_op("×"), "×" }

                    button { onclick: move |_| insert("4"), "4" }
                    button { onclick: move |_| insert("5"), "5" }
                    button { onclick: move |_| insert("6"), "6" }
                    button { onclick: move |_| insert("-"), "-" }

                    button { onclick: move |_| insert("1"), "1" }
                    button { onclick: move |_| insert("2"), "2" }
                    button { onclick: move |_| insert("3"), "3" }
                    button { onclick: move |_| insert("+"), "+" }

                    button { onclick: move |_| insert("%"), "%" }
                    button { onclick: move |_| insert("0"), "0" }
                    button { onclick: move |_| insert("."), "." }
                    button { onclick: move |_| calculate(()), "=" }

                    button { class: "col-span-2", onclick: move |_| toggle_sign(()), "±" }
                    button { class: "col-span-2", onclick: move |_| backspace(()), "⬅️" }
                }
            }
        }
    }
}


