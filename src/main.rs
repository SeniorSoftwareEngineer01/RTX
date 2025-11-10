// src/main.rs - الجزء 2: الاستيرادات
use dioxus::prelude::*;
use meval::eval_str;
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use sha2::{Sha256, Digest};
use rand::RngCore;
use std::fs;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

// هيكل بيانات الملف المشفر
#[derive(Clone, Serialize, Deserialize)]
struct EncryptedFile {
    name: String,
    original_extension: String,
    encrypted_path: String,
    size: u64,
    created_at: String,
}

fn main() {
    dioxus::launch(app);
}
// الجزء 3: دالة App الرئيسية
#[component]
fn app() -> Element {
    let mut input = use_signal(String::new);
    let mut result = use_signal(String::new);
    let mut show_vault = use_signal(|| false);
    let mut encrypted_files = use_signal(Vec::<EncryptedFile>::new);
    let mut selected_file_index = use_signal(|| None::<usize>);

    // تحميل الملفات المشفرة عند البدء
    use_effect(move || {
        load_encrypted_files(&mut encrypted_files);
    });

    // دالة إدخال النص
    let insert = use_callback(move |txt: &str| {
        let mut s = input();
        s.push_str(txt);
        input.set(s);
    });

    // دالة إدخال العمليات الحسابية
    let insert_op = use_callback(move |op: &str| {
        let op = match op {
            "×" => "*",
            "÷" => "/",
            other => other,
        };
        insert(op);
    });

    // دالة مسح الكل
    let clear_all = use_callback(move |_| {
        input.set(String::new());
        result.set(String::new());
    });

    // دالة الحذف
    let backspace = use_callback(move |_| {
        let mut s = input();
        s.pop();
        input.set(s);
    });

    // دالة تغيير الإشارة
    let toggle_sign = use_callback(move |_| {
        let s = input();
        if s.trim().is_empty() {
            input.set("-".to_string());
        } else {
            input.set(format!("(-({}))", s));
        }
    });

    // الجزء 4: دالة الحساب والتحقق من الرقم السري 49
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
                result.set(out.clone());
                
                // 🔐 إذا كانت النتيجة 49، افتح الخزنة السرية!
                if out == "49" {
                    show_vault.set(true);
                }
            }
            Err(_) => result.set("خطأ في الصيغة".to_string()),
        }
    });

    // معالج لوحة المفاتيح
    let on_key = move |e: KeyboardEvent| {
        match e.key() {
            Key::Character(s) => {
                let owned_s = s.to_string();
                if owned_s == "," {
                    let mut current = input();
                    current.push('.');
                    input.set(current);
                } else if owned_s.chars().all(|c| "0123456789.+-*/()%".contains(c)) {
                    let mut current = input();
                    current.push_str(&owned_s);
                    input.set(current);
                }
            }
            Key::Enter => calculate(()),
            Key::Backspace => backspace(()),
            _ => {}
        }
    };

// الجزء 5: دوال إدارة الملفات (رفع، فتح، حذف)
    
    // دالة رفع الملفات
    // دالة رفع الملفات - نسخة محدثة بدون rfd
    let upload_file = move |_| {
        // استخدام مربع حوار سطر الأوامر البسيط
        spawn(async move {
            // طباعة رسالة للمستخدم
            println!("📁 اسحب ملف إلى المجلد التالي للتشفير:");
            let vault_dir = get_vault_dir();
            let upload_dir = vault_dir.join("upload");
            let _ = fs::create_dir_all(&upload_dir);
            println!("   {}", upload_dir.display());
            
            // مراقبة المجلد للملفات الجديدة
            if let Ok(entries) = fs::read_dir(&upload_dir) {
                for entry in entries.flatten() {
                    if let Ok(metadata) = entry.metadata() {
                        if metadata.is_file() {
                            if let Ok(file_name) = entry.file_name().into_string() {
                                if let Ok(data) = fs::read(entry.path()) {
                                    // تشفير الملف
                                    if let Ok(encrypted_info) = encrypt_file(&file_name, &data).await {
                                        let mut files = encrypted_files();
                                        files.push(encrypted_info);
                                        save_encrypted_files(&files);
                                        encrypted_files.set(files);
                                        
                                        // حذف الملف الأصلي
                                        let _ = fs::remove_file(entry.path());
                                        println!("✅ تم تشفير: {}", file_name);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    };

    // دالة فك التشفير والفتح
    let decrypt_and_open = move |index: usize| {
        spawn(async move {
            let files = encrypted_files();
            if let Some(file) = files.get(index) {
                if let Ok(decrypted_data) = decrypt_file(&file.encrypted_path).await {
                    // حفظ الملف المفكوك مؤقتاً وفتحه
                    let temp_dir = std::env::temp_dir();
                    let temp_path = temp_dir.join(&file.name);
                    if fs::write(&temp_path, decrypted_data).is_ok() {
                        let _ = open::that(&temp_path);
                    }
                }
            }
        });
    };

    // دالة حذف الملف
    let delete_file = move |index: usize| {
        let mut files = encrypted_files();
        if let Some(file) = files.get(index) {
            let _ = fs::remove_file(&file.encrypted_path);
            files.remove(index);
            save_encrypted_files(&files);
            encrypted_files.set(files);
            selected_file_index.set(None);
        }
    };


// الجزء 6: عرض الواجهة المناسبة
    if show_vault() {
        // عرض واجهة الخزنة السرية
        rsx! {
            div { 
                style: "min-height:100vh;background:linear-gradient(135deg,#667eea 0%,#764ba2 100%);color:white;padding:20px;font-family:system-ui,sans-serif;",
                
                // رأس الخزنة
                div { 
                    style: "max-width:800px;margin:0 auto;background:rgba(255,255,255,0.1);backdrop-filter:blur(20px);padding:20px;border-radius:20px;margin-bottom:20px;display:flex;justify-content:space-between;align-items:center;",
                    h2 { "🔐 الخزنة السرية" }
                    button {
                        style: "background:#f5576c;border:none;border-radius:12px;padding:12px 24px;color:white;font-weight:700;cursor:pointer;",
                        onclick: move |_| show_vault.set(false),
                        "إغلاق"
                    }
                }
                
                div { style: "max-width:800px;margin:0 auto;",
                    // زر رفع ملف جديد
                    button {
                        style: "width:100%;background:linear-gradient(135deg,#4facfe 0%,#00f2fe 100%);border:none;border-radius:16px;padding:16px;color:white;font-size:18px;font-weight:700;cursor:pointer;margin-bottom:20px;",
                        onclick: move |_| upload_file(()),
                        "📤 رفع ملف جديد"
                    }

                    // قائمة الملفات
                    div { 
                        style: "background:rgba(255,255,255,0.1);backdrop-filter:blur(20px);border-radius:20px;padding:20px;max-height:600px;overflow-y:auto;",
                        
                        if encrypted_files().is_empty() {
                            div { 
                                style: "text-align:center;padding:40px;opacity:0.6;",
                                "📂 لا توجد ملفات مشفرة"
                            }
                        } else {
                            for (index, file) in encrypted_files().iter().enumerate() {
                                div {
                                    key: "{index}",
                                    style: "background:rgba(255,255,255,0.15);border-radius:16px;padding:16px;margin-bottom:12px;cursor:pointer;",
                                    onclick: move |_| {
                                        if selected_file_index() == Some(index) {
                                            selected_file_index.set(None);
                                        } else {
                                            selected_file_index.set(Some(index));
                                        }
                                    },
                                    
                                    div { 
                                        style: "display:flex;align-items:center;gap:12px;",
                                        div { style: "font-size:32px;", "{get_file_icon(&file.original_extension)}" }
                                        div {
                                            div { style: "font-weight:700;font-size:16px;", "{file.name}" }
                                            div { style: "opacity:0.7;font-size:13px;", 
                                                "{format_size(file.size)} • {file.created_at}"
                                            }
                                        }
                                    }

                                    if selected_file_index() == Some(index) {
                                        div { 
                                            style: "display:flex;gap:10px;margin-top:12px;padding-top:12px;border-top:1px solid rgba(255,255,255,0.2);",
                                            button {
                                                style: "background:#4facfe;border:none;border-radius:10px;padding:10px 20px;color:white;font-weight:600;cursor:pointer;",
                                                onclick: move |e| {
                                                    e.stop_propagation();
                                                    decrypt_and_open(index);
                                                },
                                                "فتح 📂"
                                            }
                                            button {
                                                style: "background:#f5576c;border:none;border-radius:10px;padding:10px 20px;color:white;font-weight:600;cursor:pointer;",
                                                onclick: move |e| {
                                                    e.stop_propagation();
                                                    delete_file(index);
                                                },
                                                "حذف 🗑️"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        // عرض الآلة الحاسبة العادية
        render_calculator(input, result, on_key, clear_all, insert, insert_op, calculate, toggle_sign, backspace)
    }
}
// نهاية دالة app


// الجزء 7: دالة عرض الآلة الحاسبة
fn render_calculator(
    input: Signal<String>,
    result: Signal<String>,
    on_key: impl Fn(KeyboardEvent) + 'static,
    clear_all: Callback<()>,
    insert: Callback<&'static str>,
    insert_op: Callback<&'static str>,
    calculate: Callback<()>,
    toggle_sign: Callback<()>,
    backspace: Callback<()>,
) -> Element {
    let root_style = "min-height:100vh;display:flex;align-items:center;justify-content:center;background:linear-gradient(135deg,#667eea 0%,#764ba2 100%);color:white;font-family:system-ui,sans-serif;padding:20px;";
    let card_style = "width:380px;background:rgba(255,255,255,0.1);backdrop-filter:blur(20px);border:1px solid rgba(255,255,255,0.2);border-radius:24px;padding:24px;box-shadow:0 20px 60px rgba(0,0,0,0.3);";
    let screen_style = "background:rgba(0,0,0,0.3);border:1px solid rgba(255,255,255,0.1);border-radius:16px;padding:20px;display:flex;flex-direction:column;gap:8px;margin-bottom:20px;min-height:100px;";
    let input_style = "font-size:20px;opacity:0.8;word-break:break-all;text-align:right;min-height:28px;";
    let result_style = "font-size:36px;font-weight:700;text-align:right;word-break:break-all;min-height:42px;";
    let grid_style = "display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:12px;";
    
    let btn_number = "background:rgba(255,255,255,0.15);border:1px solid rgba(255,255,255,0.2);border-radius:16px;padding:20px;font-size:24px;font-weight:600;color:white;cursor:pointer;";
    let btn_operator = "background:linear-gradient(135deg,#f093fb 0%,#f5576c 100%);border:none;border-radius:16px;padding:20px;font-size:24px;font-weight:700;color:white;cursor:pointer;";
    let btn_clear = "background:linear-gradient(135deg,#fa709a 0%,#fee140 100%);border:none;border-radius:16px;padding:20px;font-size:22px;font-weight:700;color:white;cursor:pointer;";
    let btn_equals = "background:linear-gradient(135deg,#4facfe 0%,#00f2fe 100%);border:none;border-radius:16px;padding:20px;font-size:28px;font-weight:700;color:white;cursor:pointer;";
    let btn_special = "background:rgba(255,255,255,0.1);border:1px solid rgba(255,255,255,0.2);border-radius:16px;padding:20px;font-size:20px;font-weight:600;color:white;cursor:pointer;grid-column:span 2;";

    rsx! {
        div { style: "{root_style}",
            div { style: "{card_style}", tabindex: 0, onkeydown: on_key,
                div { style: "{screen_style}",
                    div { style: "{input_style}", "{input}" }
                    div { style: "{result_style}", "{result}" }
                }
                div { style: "{grid_style}",
                    button { style: "{btn_clear}", onclick: move |_| clear_all(()), "C" }
                    button { style: "{btn_number}", onclick: move |_| insert("("), "(" }
                    button { style: "{btn_number}", onclick: move |_| insert(")"), ")" }
                    button { style: "{btn_operator}", onclick: move |_| insert_op("÷"), "÷" }

                    button { style: "{btn_number}", onclick: move |_| insert("7"), "7" }
                    button { style: "{btn_number}", onclick: move |_| insert("8"), "8" }
                    button { style: "{btn_number}", onclick: move |_| insert("9"), "9" }
                    button { style: "{btn_operator}", onclick: move |_| insert_op("×"), "×" }

                    button { style: "{btn_number}", onclick: move |_| insert("4"), "4" }
                    button { style: "{btn_number}", onclick: move |_| insert("5"), "5" }
                    button { style: "{btn_number}", onclick: move |_| insert("6"), "6" }
                    button { style: "{btn_operator}", onclick: move |_| insert("-"), "-" }

                    button { style: "{btn_number}", onclick: move |_| insert("1"), "1" }
                    button { style: "{btn_number}", onclick: move |_| insert("2"), "2" }
                    button { style: "{btn_number}", onclick: move |_| insert("3"), "3" }
                    button { style: "{btn_operator}", onclick: move |_| insert("+"), "+" }

                    button { style: "{btn_number}", onclick: move |_| insert("%"), "%" }
                    button { style: "{btn_number}", onclick: move |_| insert("0"), "0" }
                    button { style: "{btn_number}", onclick: move |_| insert("."), "." }
                    button { style: "{btn_equals}", onclick: move |_| calculate(()), "=" }

                    button { style: "{btn_special}", onclick: move |_| toggle_sign(()), "±" }
                    button { style: "{btn_special}", onclick: move |_| backspace(()), "⬅️" }
                }
            }
        }
    }
}

// الجزء 8: دوال التشفير وفك التشفير (AES-256-GCM)

// دالة تشفير الملف
async fn encrypt_file(file_name: &str, data: &[u8]) -> Result<EncryptedFile, Box<dyn std::error::Error>> {
    // توليد مفتاح التشفير من كلمة السر "49"
    let mut hasher = Sha256::new();
    hasher.update(b"49_secret_calculator_key_2024_ultra_secure");
    let key = hasher.finalize();

    let cipher = Aes256Gcm::new(&key.into());
    
    // توليد nonce عشوائي (12 بايت)
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // تشفير البيانات
    let encrypted_data = cipher.encrypt(nonce, data)?;

    // دمج nonce مع البيانات المشفرة
    let mut final_data = nonce_bytes.to_vec();
    final_data.extend_from_slice(&encrypted_data);

    // حفظ الملف المشفر
    let vault_dir = get_vault_dir();
    fs::create_dir_all(&vault_dir)?;
    
    // اسم ملف عشوائي بامتداد .secure
    let encrypted_filename = format!("{}.secure", generate_random_id());
    let encrypted_path = vault_dir.join(&encrypted_filename);
    fs::write(&encrypted_path, &final_data)?;

    let extension = file_name.split('.').last().unwrap_or("").to_string();
    let now = chrono::Local::now();
    
    Ok(EncryptedFile {
        name: file_name.to_string(),
        original_extension: extension,
        encrypted_path: encrypted_path.to_string_lossy().to_string(),
        size: data.len() as u64,
        created_at: now.format("%Y-%m-%d %H:%M").to_string(),
    })
}

// دالة فك تشفير الملف
async fn decrypt_file(encrypted_path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let data = fs::read(encrypted_path)?;
    
    // استخراج nonce والبيانات المشفرة
    if data.len() < 12 {
        return Err("ملف غير صالح".into());
    }
    
    let (nonce_bytes, encrypted_data) = data.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    // توليد نفس المفتاح
    let mut hasher = Sha256::new();
    hasher.update(b"49_secret_calculator_key_2024_ultra_secure");
    let key = hasher.finalize();

    let cipher = Aes256Gcm::new(&key.into());
    
    // فك التشفير
    let decrypted_data = cipher.decrypt(nonce, encrypted_data)?;
    
    Ok(decrypted_data)
}


// الجزء 9: الدوال المساعدة (الأخير!)

// الحصول على مسار مجلد الخزنة
fn get_vault_dir() -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(".calculator_vault");
    path
}

// تحميل قائمة الملفات المشفرة
fn load_encrypted_files(files: &mut Signal<Vec<EncryptedFile>>) {
    let vault_dir = get_vault_dir();
    let index_path = vault_dir.join("index.json");
    
    if let Ok(data) = fs::read_to_string(&index_path) {
        if let Ok(loaded_files) = serde_json::from_str(&data) {
            files.set(loaded_files);
        }
    }
}

// حفظ قائمة الملفات المشفرة
fn save_encrypted_files(files: &[EncryptedFile]) {
    let vault_dir = get_vault_dir();
    let _ = fs::create_dir_all(&vault_dir);
    let index_path = vault_dir.join("index.json");
    
    if let Ok(json) = serde_json::to_string_pretty(files) {
        let _ = fs::write(&index_path, json);
    }
}

// توليد معرّف عشوائي
fn generate_random_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let random_num: u64 = rng.gen();
    format!("{:x}", random_num)
}

// الحصول على أيقونة الملف حسب الامتداد
fn get_file_icon(extension: &str) -> &'static str {
    match extension.to_lowercase().as_str() {
        "pdf" => "📄",
        "doc" | "docx" => "📝",
        "txt" => "📃",
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" => "🖼️",
        "mp4" | "avi" | "mov" | "mkv" | "flv" => "🎬",
        "mp3" | "wav" | "flac" | "m4a" => "🎵",
        "zip" | "rar" | "7z" | "tar" | "gz" => "📦",
        "exe" | "msi" => "⚙️",
        "html" | "css" | "js" | "json" => "💻",
        "ppt" | "pptx" => "📊",
        "xls" | "xlsx" => "📈",
        _ => "📁",
    }
}

// تنسيق حجم الملف
fn format_size(size: u64) -> String {
    if size < 1024 {
        format!("{} B", size)
    } else if size < 1024 * 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

// 🎉 نهاية الكود!