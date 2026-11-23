//! Source strings are generated from the upstream table, with Windows-specific UI terms.
include!("../assets/localization.rs");

#[derive(Clone, Debug)]
pub struct L10n {
    pub language: String,
}
impl L10n {
    pub fn new(saved: &str) -> Self {
        let language = normalize(saved)
            .or_else(system_language)
            .unwrap_or_else(|| "en".into());
        Self { language }
    }
    pub fn set_language(&mut self, code: &str) -> bool {
        if let Some(code) = normalize(code) {
            self.language = code;
            true
        } else {
            false
        }
    }
    pub fn text(&self, text: Text) -> String {
        let row = STRINGS
            .iter()
            .find(|(code, _)| *code == self.language)
            .unwrap_or(&STRINGS[0]);
        row.1[text as usize].replace("Finder", "Explorer")
    }
    pub fn hint(&self, key: &str) -> String {
        self.text(Text::OptionHint)
            .replace("⌥ Option", key)
            .replace('⌥', key)
    }
    pub fn theme(&self, theme: &str) -> String {
        match theme {
            "turk" => return self.text(Text::ThemeTurk),
            "kurd" => return self.text(Text::ThemeKurd),
            "egypt" => return self.text(Text::ThemeEgypt),
            "khayamiya" => return self.text(Text::ThemeKhaya),
            "custom" => return self.text(Text::CustomRug).replace('…', ""),
            _ => {}
        }
        let language = self.language.as_str();
        let entries: &[(&str, &str)] = match theme {
            "china" => &[
                ("tr", "Çin (ipek, şou)"),
                ("en", "Chinese (silk, shou)"),
                ("ku", "Çînî (hevrîşim)"),
                ("zh", "中国丝毯(寿)"),
                ("ja", "中国シルク(寿)"),
            ],
            "ayasofya" => &[
                ("tr", "Ayasofya (mozaik)"),
                ("en", "Hagia Sophia (mosaic)"),
                ("ku", "Ayasofya (mozaîk)"),
                ("ar", "آيا صوفيا (فسيفساء)"),
                ("fa", "ایاصوفیه (موزاییک)"),
                ("de", "Hagia Sophia (Mosaik)"),
                ("ru", "Айя-София (мозаика)"),
            ],
            "iznik" => &[
                ("tr", "İznik çinisi (Sultanahmet)"),
                ("en", "Iznik tiles (Blue Mosque)"),
                ("ku", "Çînîyên Îznîkê (Sultanahmet)"),
            ],
            "selcuklu" => &[
                ("tr", "Selçuklu (Konya)"),
                ("en", "Seljuk (Konya)"),
                ("ku", "Selçûqî (Konya)"),
                ("de", "Seldschuken (Konya)"),
                ("fr", "Seldjoukide (Konya)"),
                ("es", "Selyúcida (Konya)"),
                ("ru", "Сельджукский (Конья)"),
                ("ar", "سلجوقي (قونية)"),
                ("fa", "سلجوقی (قونیه)"),
            ],
            "berber" => &[
                ("tr", "Fas Berberi (Beni Ourain)"),
                ("en", "Moroccan (Beni Ourain)"),
                ("ku", "Fasî (Beni Ourain)"),
            ],
            "kazak" => &[
                ("tr", "Kafkas (Kazak)"),
                ("en", "Caucasian (Kazak)"),
                ("ku", "Qefqasî (Kazak)"),
            ],
            "kashan" => &[
                ("tr", "Kaşan (kırmızı)"),
                ("en", "Kashan (red)"),
                ("ku", "Kaşan (sor)"),
            ],
            "isfahan" => &[
                ("tr", "İsfahan (lacivert)"),
                ("en", "Isfahan (navy)"),
                ("ku", "Îsfehan (şîn)"),
            ],
            "tebriz" => &[
                ("tr", "Tebriz (bej)"),
                ("en", "Tabriz (beige)"),
                ("ku", "Tebrîz (bej)"),
            ],
            "turkmen" => &[
                ("tr", "Türkmen / Buhara"),
                ("en", "Turkmen / Bukhara"),
                ("ku", "Tirkmen / Bûxara"),
            ],
            "kilim" => &[("tr", "Kilim"), ("en", "Kilim"), ("ku", "Kilîm")],
            _ => &[],
        };
        entries
            .iter()
            .find(|(code, _)| *code == language)
            .map(|(_, name)| *name)
            .unwrap_or(match theme {
                "china" => "Chinese silk (寿)",
                "ayasofya" => "Hagia Sophia",
                "iznik" => "Iznik (Sultanahmet)",
                "selcuklu" => "Seljuk (Konya)",
                "berber" => "Beni Ourain (Morocco)",
                "kazak" => "Kazak (Caucasus)",
                "kashan" => "Kashan",
                "isfahan" => "Isfahan",
                "tebriz" => "Tebriz",
                "turkmen" => "Turkmen / Bukhara",
                "kilim" => "Kilim",
                _ => theme,
            })
            .into()
    }
    /// Pattern-list label for the rug woven from the user's picture.
    ///
    /// Upstream derives it from the file-picker string by dropping the ellipsis, which leaves two
    /// menu entries that read exactly alike ("…" aside). Windows names the pattern after the noun
    /// instead so the checked entry and the picker action stay distinguishable.
    pub fn custom_theme(&self) -> String {
        let entries: &[(&str, &str)] = &[
            ("en", "Custom image rug"),
            ("zh", "自定义图片地毯"),
            ("hi", "अपनी तस्वीर का गलीचा"),
            ("es", "Alfombra de imagen propia"),
            ("ar", "سجادة من صورة"),
            ("fr", "Tapis d'après image"),
            ("bn", "নিজের ছবির কার্পেট"),
            ("pt", "Tapete da própria imagem"),
            ("ru", "Ковёр из своей картинки"),
            ("ur", "اپنی تصویر کا قالین"),
            ("id", "Karpet dari gambar sendiri"),
            ("de", "Teppich aus eigenem Bild"),
            ("ja", "自分の画像のラグ"),
            ("ko", "내 이미지 러그"),
            ("it", "Tappeto da immagine"),
            ("vi", "Thảm từ ảnh của bạn"),
            ("fa", "فرش از تصویر شما"),
            ("pl", "Dywan z własnego obrazu"),
            ("nl", "Kleed van eigen afbeelding"),
            ("tr", "Kendi resminden halı"),
            ("ku", "Xalîçeya ji wêneya te"),
        ];
        entries
            .iter()
            .find(|(code, _)| *code == self.language)
            .map(|(_, name)| (*name).to_string())
            .unwrap_or_else(|| {
                self.text(Text::CustomRug)
                    .replace('…', "")
                    .trim()
                    .to_string()
            })
    }
    /// Menu entry that lays the current rug pattern over the whole desktop as one sheet.
    /// Windows-only feature: upstream has no counterpart, so the caption lives here.
    pub fn fullscreen_label(&self) -> String {
        let entries: &[(&str, &str)] = &[
            ("en", "Full-screen cloth"),
            ("zh", "全屏桌布"),
            ("hi", "फ़ुल-स्क्रीन चादर"),
            ("es", "Mantel a pantalla completa"),
            ("ar", "غطاء الشاشة الكامل"),
            ("fr", "Nappe plein écran"),
            ("bn", "ফুল-স্ক্রিন চাদর"),
            ("pt", "Toalha em tela cheia"),
            ("ru", "Скатерть на весь экран"),
            ("ur", "پوری اسکرین کا میزپوش"),
            ("id", "Taplak layar penuh"),
            ("de", "Bildschirmgroße Tischdecke"),
            ("ja", "全画面クロス"),
            ("ko", "전체 화면 천"),
            ("it", "Tovaglia a schermo intero"),
            ("vi", "Khăn phủ toàn màn hình"),
            ("fa", "روتختی تمام‌صفحه"),
            ("pl", "Obrus na cały ekran"),
            ("nl", "Kleed over het hele scherm"),
            ("tr", "Tam ekran örtü"),
            ("ku", "Dapxeta tevahiya ekranê"),
        ];
        entries
            .iter()
            .find(|(code, _)| *code == self.language)
            .map(|(_, name)| (*name).to_string())
            .unwrap_or_else(|| "Full-screen cloth".into())
    }
    pub fn menu_order() -> Vec<(&'static str, &'static str)> {
        ["tr", "en", "ku"]
            .into_iter()
            .filter_map(|code| LANGUAGES.iter().copied().find(|(key, _)| *key == code))
            .chain(
                LANGUAGES
                    .iter()
                    .copied()
                    .filter(|(code, _)| !["tr", "en", "ku"].contains(code)),
            )
            .collect()
    }
    pub fn validate() -> Vec<String> {
        LANGUAGES
            .iter()
            .filter(|&(code, _)| !STRINGS
                    .iter()
                    .any(|(key, row)| key == code && row.iter().all(|value| !value.is_empty()))).map(|(code, _)| (*code).to_string())
            .collect()
    }
}
fn normalize(value: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    let code = lower.split(['-', '_']).next().unwrap_or_default();
    let code = match code {
        "ckb" | "kmr" => "ku",
        "in" => "id",
        _ => code,
    };
    LANGUAGES
        .iter()
        .any(|(key, _)| *key == code)
        .then(|| code.to_string())
}
fn system_language() -> Option<String> {
    use windows::Win32::Globalization::{GetUserPreferredUILanguages, MUI_LANGUAGE_NAME};
    // SAFETY: the first call queries a UTF-16 buffer length; the second owns a buffer of that length.
    unsafe {
        let mut count = 0;
        let mut length = 0;
        if GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut count, None, &mut length).is_err()
            || length == 0
        {
            return None;
        }
        let mut names = vec![0u16; length as usize];
        GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &mut count,
            Some(windows::core::PWSTR(names.as_mut_ptr())),
            &mut length,
        )
        .ok()?;
        names
            .split(|value| *value == 0)
            .filter(|name| !name.is_empty())
            .find_map(|name| normalize(&String::from_utf16_lossy(name)))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_upstream_languages_are_complete_and_aliases_work() {
        assert_eq!(LANGUAGES.len(), 21);
        assert_eq!(TEXT_COUNT, 24);
        assert!(L10n::validate().is_empty());
        for code in ["ckb-IQ", "kmr-TR"] {
            assert_eq!(normalize(code).as_deref(), Some("ku"));
        }
        assert_eq!(normalize("in-ID").as_deref(), Some("id"));
        assert!(normalize("xx").is_none());
        for (code, _) in LANGUAGES {
            let mut language = L10n::new(code);
            assert!(language.set_language(code));
            assert!(!language.text(Text::Quit).is_empty());
        }
    }
    #[test]
    fn custom_pattern_label_differs_from_picker_action() {
        for (code, _) in LANGUAGES {
            let language = L10n::new(code);
            let pattern = language.custom_theme();
            let action = language.text(Text::CustomRug);
            assert!(!pattern.is_empty(), "{code}: empty custom pattern label");
            assert_ne!(
                pattern, action,
                "{code}: pattern label equals the picker action"
            );
            assert!(
                !action.starts_with(&pattern),
                "{code}: pattern label {pattern:?} is a prefix of the picker action, so both entries read alike"
            );
        }
    }
}
