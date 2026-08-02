# Supported Languages

hypermail-rs ships with **189 locale files** embedded at compile time.
The active language is selected via the `language` setting in the `.hmrc`
configuration file, e.g.:

```ini
language = el        # Greek
language = ja        # Japanese
language = x-klingon # Klingon
```

Language resolution order:

1. **Exact match** after alias normalisation (e.g. `el`, `x-klingon`)
2. **Base subtag** – `pt-BR` resolves to `pt`
3. **English fallback** if the tag is unknown

Known aliases: `gr`→`el`, `no`→`nb`, `in`→`id`, `iw`→`he`, `ji`→`yi`,
`jw`→`jv`, `zh-cn`/`zh-hans`/`zh-sg`→`zh`,
`zh-hant`/`zh-hk`→`zh-tw`.

---

## ISO 639-1 Languages (181)

The 181 codes below each have their own dedicated locale file. The remaining
deprecated ISO 639-1 codes (`no`, `in`, `iw`, `ji`, `jw`) are normalised to
their modern equivalents (`nb`, `id`, `he`, `yi`, `jv`); they are listed in
the table for discoverability and resolve via the alias table in `i18n.rs`.

| Code | Language | Spoken in |
|------|----------|-----------|
| `aa` | Qafár af (Afar) | Ethiopia, Eritrea, Djibouti |
| `ab` | Аҧсшәа (Abkhazian) | Abkhazia, Georgia |
| `ae` | 𐬀𐬎𐬎𐬆𐬯𐬙𐬀 / Avestā (Avestan) | historical/liturgical only (Zoroastrian scripture) |
| `af` | Afrikaans (Afrikaans) | South Africa, Namibia |
| `ak` | Akan (Akan) | Ghana |
| `am` | አማርኛ (Amharic) | Ethiopia |
| `an` | Aragonés (Aragonese) | Aragon, Spain |
| `ar` | Arabic (العربية) | Arab world (22 countries) |
| `as` | অসমীয়া (Assamese) | Assam, India |
| `av` | Магӏарул мацӏ (Avaric) | Dagestan, Russia |
| `ay` | Aymar aru (Aymara) | Bolivia, Peru, Chile |
| `az` | Azerbaijani (Azərbaycan dili) | Azerbaijan, Iran |
| `ba` | Башҡорт теле (Bashkir) | Bashkortostan, Russia |
| `be` | Беларуская (Belarusian) | Belarus |
| `bg` | Български (Bulgarian) | Bulgaria |
| `bi` | Bislama (Bislama) | Vanuatu |
| `bm` | Bamanankan (Bambara) | Mali |
| `bn` | বাংলা (Bengali) | Bangladesh, India |
| `bo` | བོད་སྐད་ (Tibetan) | Tibet, Nepal, India |
| `br` | Brezhoneg (Breton) | Brittany, France |
| `bs` | Bosanski (Bosnian) | Bosnia and Herzegovina |
| `ca` | Català (Catalan) | Catalonia, Valencia, Andorra |
| `ce` | Нохчийн мотт (Chechen) | Chechnya, Russia |
| `ch` | Chamoru (Chamorro) | Guam, Northern Mariana Islands |
| `co` | Corsu (Corsican) | Corsica, France |
| `cr` | Nēhiyawēwin (Cree) | Canada |
| `cs` | Čeština (Czech) | Czech Republic |
| `cu` | Ѩзыкъ словѣньскъ (Church Slavonic) | Liturgical (Orthodox Christianity) |
| `cv` | Чӑваш чӗлхи (Chuvash) | Russia (Chuvash Republic) |
| `cy` | Cymraeg (Welsh) | Wales, United Kingdom |
| `da` | Dansk (Danish) | Denmark |
| `de` | Deutsch (German) | Germany, Austria, Switzerland |
| `dv` | ދިވެހި (Divehi) | Maldives |
| `dz` | རྫོང་ཁ (Dzongkha) | Bhutan |
| `ee` | Eʋegbe (Ewe) | Ghana, Togo |
| `el` | Ελληνικά (Greek) | Greece, Cyprus |
| `en` | English (English) | Worldwide |
| `eo` | Esperanto (Esperanto) | International constructed language |
| `es` | Español (Spanish) | Spain, Latin America, USA |
| `et` | Eesti (Estonian) | Estonia |
| `eu` | Euskara (Basque) | Basque Country (Spain/France) |
| `fa` | فارسی (Persian/Farsi) | Iran, Afghanistan, Tajikistan |
| `ff` | Fulfulde / Pulaar (Fula) | West Africa |
| `fi` | Suomi (Finnish) | Finland |
| `fj` | Vosa Vakaviti (Fijian) | Fiji |
| `fo` | Føroyskt (Faroese) | Faroe Islands |
| `fr` | Français (French) | France, Belgium, Switzerland, Canada, Africa |
| `fy` | West-Frysk (Western Frisian) | Friesland, Netherlands |
| `ga` | Gaeilge (Irish) | Ireland |
| `gd` | Gàidhlig (Scottish Gaelic) | Scotland |
| `gl` | Galego (Galician) | Galicia, Spain |
| `gn` | Avañe'ẽ (Guaraní) | Paraguay, Bolivia, Argentina |
| `gu` | ગુજરાતી (Gujarati) | India (Gujarat) |
| `gv` | Gaelg (Manx) | Isle of Man |
| `ha` | Hausa (Hausa) | Nigeria, Niger, West Africa |
| `he` | עברית (Hebrew) | Israel |
| `hi` | हिन्दी (Hindi) | India |
| `hr` | Hrvatski (Croatian) | Croatia |
| `hu` | Magyar (Hungarian) | Hungary |
| `hy` | Հայերեն (Armenian) | Armenia |
| `hz` | Otjiherero (Herero) | Namibia, Botswana |
| `ia` | Interlingua (Interlingua) | International (IALA constructed) |
| `id` | Bahasa Indonesia (Indonesian) | Indonesia |
| `ie` | Interlingue (Interlingue) | International (constructed) |
| `ig` | Igbo (Igbo) | Nigeria |
| `ii` | ꆈꌠꉙ (Sichuan Yi) | Sichuan, China |
| `ik` | Iñupiaq (Inupiaq) | Alaska, USA |
| `io` | Ido (Ido) | International (reformed Esperanto) |
| `is` | Íslenska (Icelandic) | Iceland |
| `it` | Italiano (Italian) | Italy, Switzerland, San Marino |
| `iu` | ᐃᓄᒃᑎᑐᑦ (Inuktitut) | Canada (Nunavut) |
| `ja` | 日本語 (Japanese) | Japan |
| `jv` | Basa Jawa (Javanese) | Indonesia (Java) |
| `ka` | ქართული (Georgian) | Georgia |
| `kg` | Kikongo (Kongo) | DR Congo, Angola |
| `ki` | Gĩkũyũ (Kikuyu) | Kenya |
| `kj` | Oshikwanyama (Kwanyama) | Namibia, Angola |
| `kk` | Қазақ тілі (Kazakh) | Kazakhstan |
| `kl` | Kalaallisut (Kalaallisut) | Greenland |
| `km` | ភាសាខ្មែរ (Khmer) | Cambodia |
| `kn` | ಕನ್ನಡ (Kannada) | India (Karnataka) |
| `ko` | 한국어 (Korean) | South Korea, North Korea |
| `kr` | Kānūrī (Kanuri) | Nigeria, Niger, Chad |
| `ks` | کٲشُر (Kashmiri) | Kashmir |
| `ku` | Kurdî (Kurdish/Kurmanji) | Turkey, Syria, Iraq, Iran |
| `kv` | Коми кыв (Komi) | Russia (Komi Republic) |
| `kw` | Kernewek (Cornish) | Cornwall, UK |
| `ky` | Кыргыз тили (Kyrgyz) | Kyrgyzstan |
| `la` | Latina (Latin) | Vatican City, classical/liturgical use |
| `lb` | Lëtzebuergesch (Luxembourgish) | Luxembourg |
| `lg` | Luganda (Ganda) | Uganda |
| `li` | Limburgs (Limburgish) | Netherlands, Belgium, Germany |
| `ln` | Lingála (Lingala) | DR Congo, Republic of Congo |
| `lo` | ພາສາລາວ (Lao) | Laos |
| `lt` | Lietuvių (Lithuanian) | Lithuania |
| `lu` | Kiluba (Luba-Katanga) | DR Congo |
| `lv` | Latviešu (Latvian) | Latvia |
| `mg` | Malagasy (Malagasy) | Madagascar |
| `mh` | Kajin M̧ajeļ (Marshallese) | Marshall Islands |
| `mi` | Te Reo Māori (Māori) | New Zealand |
| `ml` | മലയാളം (Malayalam) | India (Kerala) |
| `mn` | Монгол хэл (Mongolian) | Mongolia |
| `mr` | मराठी (Marathi) | India (Maharashtra) |
| `ms` | Bahasa Melayu (Malay) | Malaysia, Brunei, Singapore |
| `mt` | Malti (Maltese) | Malta |
| `my` | မြန်မာဘာသာ (Burmese) | Myanmar |
| `na` | Dorerin Naoero (Nauru) | Nauru |
| `nb` | Norsk bokmål (Norwegian Bokmål) | Norway |
| `nd` | isiNdebele (North Ndebele) | Zimbabwe |
| `ne` | नेपाली (Nepali) | Nepal, India |
| `ng` | Oshindonga (Ndonga) | Namibia |
| `nl` | Nederlands (Dutch) | Netherlands, Belgium, Suriname |
| `nn` | Nynorsk (Norwegian Nynorsk) | Norway |
| `nr` | isiNdebele (South Ndebele) | South Africa |
| `nv` | Diné bizaad (Navajo) | USA (Navajo Nation) |
| `ny` | Chichewa (Chichewa/Nyanja) | Malawi, Zambia, Mozambique |
| `oc` | Occitan (Occitan) | Southern France, Italy, Spain |
| `oj` | Anishinaabemowin (Ojibwe) | Canada, USA |
| `om` | Afaan Oromoo (Oromo) | Ethiopia, Kenya |
| `or` | ଓଡ଼ିଆ (Odia/Oriya) | Odisha, India |
| `os` | Ирон æвзаг (Ossetian) | North/South Ossetia |
| `pa` | ਪੰਜਾਬੀ (Punjabi) | Pakistan, India |
| `pi` | पाऴि (Pali) | Liturgical/scholarly Buddhist use |
| `pl` | Polski (Polish) | Poland |
| `ps` | پښتو (Pashto) | Afghanistan, Pakistan |
| `pt` | Português (Portuguese) | Brazil, Portugal, Angola, Mozambique |
| `qu` | Runasimi (Quechua) | Peru, Bolivia, Ecuador |
| `rm` | Rumantsch (Romansh) | Switzerland (Graubünden) |
| `rn` | Ikirundi (Kirundi) | Burundi |
| `ro` | Română (Romanian) | Romania, Moldova |
| `ru` | Русский (Russian) | Russia, Belarus, Kazakhstan |
| `rw` | Ikinyarwanda (Kinyarwanda) | Rwanda |
| `sa` | संस्कृतम् (Sanskrit) | Classical/liturgical Indian use |
| `sc` | Sardu (Sardinian) | Sardinia, Italy |
| `sd` | سنڌي (Sindhi) | Pakistan, India |
| `se` | Davvisámegiella (Northern Sami) | Norway, Sweden, Finland |
| `sg` | Sängö (Sango) | Central African Republic |
| `si` | සිංහල (Sinhala) | Sri Lanka |
| `sk` | Slovenčina (Slovak) | Slovakia |
| `sl` | Slovenščina (Slovenian) | Slovenia |
| `sm` | Gagana Samoa (Samoan) | Samoa, American Samoa |
| `sn` | chiShona (Shona) | Zimbabwe |
| `so` | Soomaali (Somali) | Somalia, Ethiopia, Kenya, Djibouti |
| `sq` | Shqip (Albanian) | Albania, Kosovo |
| `sr` | Srpski (Serbian) | Serbia, Bosnia, Montenegro |
| `ss` | SiSwati (Swati/Swazi) | Eswatini, South Africa |
| `st` | Sesotho (Sesotho/Southern Sotho) | Lesotho, South Africa |
| `su` | Basa Sunda (Sundanese) | Indonesia (West Java) |
| `sv` | Svenska (Swedish) | Sweden, Finland |
| `sw` | Kiswahili (Swahili) | Tanzania, Kenya, Uganda, East Africa |
| `ta` | தமிழ் (Tamil) | India, Sri Lanka, Singapore |
| `te` | తెలుగు (Telugu) | India (Andhra Pradesh, Telangana) |
| `tg` | Тоҷикӣ (Tajik) | Tajikistan |
| `th` | ภาษาไทย (Thai) | Thailand |
| `ti` | ትግርኛ (Tigrinya) | Eritrea, Ethiopia |
| `tk` | Türkmençe (Turkmen) | Turkmenistan |
| `tl` | Filipino (Filipino/Tagalog) | Philippines |
| `tn` | Setswana (Tswana/Setswana) | Botswana, South Africa |
| `to` | Lea faka-Tonga (Tongan) | Tonga |
| `tr` | Türkçe (Turkish) | Turkey, Cyprus |
| `ts` | Xitsonga (Tsonga) | South Africa, Mozambique |
| `tt` | Татар теле (Tatar) | Russia (Tatarstan) |
| `tw` | Twi (Twi) | Ghana (Ashanti) |
| `ty` | Reo Tahiti (Tahitian) | French Polynesia |
| `ug` | ئۇيغۇرچە (Uyghur) | Xinjiang, China |
| `uk` | Українська (Ukrainian) | Ukraine |
| `ur` | اردو (Urdu) | Pakistan, India |
| `uz` | Oʻzbek tili (Uzbek) | Uzbekistan |
| `va` | Valencià (Valencian) | Valencia, Spain |
| `ve` | Tshivenḓa (Venda) | South Africa, Zimbabwe |
| `vi` | Tiếng Việt (Vietnamese) | Vietnam |
| `vo` | Volapük (Volapük) | International (constructed) |
| `wa` | Walon (Walloon) | Belgium (Wallonia) |
| `wo` | Wolof (Wolof) | Senegal, Gambia |
| `xh` | isiXhosa (Xhosa) | South Africa |
| `yi` | ייִדיש (Yiddish) | Jewish diaspora |
| `yo` | Yorùbá (Yoruba) | Nigeria, Benin, Togo |
| `za` | Vahcuengh (Zhuang) | Guangxi, China |
| `zh` | 普通话/简体中文 (Chinese Simplified) | China, Singapore |
| `zh-tw` | 繁體中文 (Chinese Traditional) | Taiwan, Hong Kong, Macau |
| `zu` | isiZulu (Zulu) | South Africa |

### Deprecated ISO 639-1 codes handled as aliases

| Alias | Resolves to | Notes |
|-------|-------------|----|
| `no` | `nb` | Norwegian → Bokmål |
| `in` | `id` | Old Indonesian code |
| `iw` | `he` | Old Hebrew code |
| `ji` | `yi` | Old Yiddish code |
| `jw` | `jv` | Old Javanese code |
| `gr` | `el` | Informal alias for Greek |
| `zh-cn` / `zh-hans` / `zh-sg` | `zh` | Simplified Chinese variants |
| `zh-hant` / `zh-hk` | `zh-tw` | Traditional Chinese variants |

---

## ISO 639-2 / non-639-1 (1)

| Code | Language | Notes |
|------|----------|----|
| `grc` | Ἑλληνική (Ancient Greek) | Classical Attic dialect, polytonic Greek script. ISO 639-2 code. |

---

## IETF BCP 47 Private-Use Languages (`x-*`) (7)

These are fictional or constructed languages that have no ISO 639-1 code.
The `x-` prefix is the IETF BCP 47 standard for private-use language tags.

| Code | Language | Universe | Notes |
|------|----------|----------|----|
| `x-dothraki` | Dothraki | Game of Thrones / House of the Dragon | Created by David J. Peterson |
| `x-klingon` | tlhIngan Hol (Klingon) | Star Trek | Created by Marc Okrand; Latin transliteration (pIqaD script) |
| `x-lojban` | la .lojban. (Lojban) | Real constructed-language community | ISO 639-3: `jbo` |
| `x-navii` | Na'vi | Avatar | Created by Paul Frommer |
| `x-quenya` | Quenya (High Elvish) | Tolkien's Middle-earth | ISO 639-3: `qya` |
| `x-sindarin` | Sindarin (Grey Elvish) | Tolkien's Middle-earth | ISO 639-3: `sjn` |
| `x-valyrian` | Valyrio Muño Ēngos (High Valyrian) | Game of Thrones | Created by David J. Peterson |

---

## Adding a new language

1. Create `src/locale/<code>.json` with all 27 required keys (see `src/locale/en.json` for the template).
2. The `_comment` field is mandatory — use the format:
   ```
   "ISO 639-1: xx | BCP 47: xx | Language: NativeName (EnglishName) | Spoken in: Countries | Spoken in: ~NM"
   ```
3. Add the code to the `LOCALES` array in `src/i18n.rs`.
4. Run `cargo test i18n` — the `all_locales_have_required_keys` and `all_locales_load_via_new` tests will catch any issues.
