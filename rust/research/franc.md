# Research: Replacing `franc` in Rust

## 1. Exact Usage in `djp-sitrep`
- **Declared in**: `package.json` (`"franc": "6.2.0"` — the sole npm runtime dependency).
- **Single call site**: `src/indonesian-nlp.js:994-995` inside `detectLanguageProfile(rawText)`:
  ```js
  const lang = francModule ? francModule(clean) : 'und';
  const foreignLangDetected = ['nld', 'deu', 'fra', 'spa', 'por', 'ita', 'rus', 'pol', 'dan', 'swe', 'nor'].includes(lang);
  ```
- **Guard before `franc`**: `if (idHits >= 2) return { isForeign: false, lang: 'ind', ... }` (short-circuits Indonesian tweets/headlines via `ID_UNAMBIGUOUS_CORE_RE`).
- **Behavior of `franc@6.2.0`** (primary source: `node_modules/franc/index.js` & `wooorm/franc`):
  - Trigram frequency distance algorithm (Cavnar & Trenkle).
  - Returns `'und'` if input length `< 10` characters (`minLength: 10`).
  - Returns ISO 639-3 three-letter codes (e.g., `'nld'`, `'deu'`, `'fra'`, `'spa'`, `'por'`, `'ita'`, `'rus'`, `'pol'`, `'dan'`, `'swe'`, `'nob'`/`'nor'`).
  - Note: in `franc` v6, Norwegian Bokmål is actually `'nob'` (and Nynorsk `'nno'`), while `'nor'` is an ISO 639-3 macrolanguage code.

## 2. Rust Crate Comparison

| Crate | Algorithm | License | Footprint (Binary/RAM) | Covers Target 11 Euro Langs + `ind`/`eng`? | Verdict |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **`whatlang`** (`0.16`–`0.18`) | Trigram distance (same family as `franc`) | MIT | ~200 KB static tables, negligible RAM | **Yes**: `Nld`, `Deu`, `Fra`, `Spa`, `Por`, `Ita`, `Rus`, `Pol`, `Dan`, `Swe`, `Nob` (+ `Ind`, `Eng`, `Jav`) | **Recommended (ADR-003)** |
| **`lingua`** (`1.x`) | Full n-gram statistical models (1..5-grams) | Apache-2.0 | ~2–5 MB per language model (20–40+ MB for 13 langs, high RAM if preloaded) | Yes (incl. Malay) | Overkill for a secondary European-language filter |
| **`whichlang`** | Tiny quantized trigram model (16 langs) | MIT | ~100 KB | **No** — missing `dan`, `nor`, `pol` | Incomplete coverage |

## 3. Semantic Traps to Handle in Rust Port
1. **Minimum text length**: `whatlang::detect_lang` will attempt detection even on 3–9 character strings, whereas `franc` returns `'und'` for `< 10` chars.
   - **Rule**: Check `if clean.chars().count() < 10 { return "und"; }` before calling `whatlang`.
2. **ISO 639-3 code mapping**: `whatlang::Lang::code()` returns:
   - `Nld -> "nld"`, `Deu -> "deu"`, `Fra -> "fra"`, `Spa -> "spa"`, `Por -> "por"`, `Ita -> "ita"`, `Rus -> "rus"`, `Pol -> "pol"`, `Dan -> "dan"`, `Swe -> "swe"`, `Ind -> "ind"`, `Eng -> "eng"`.
   - `Nob -> "nob"` — map `"nob"` (and `"nno"` if ever encountered) consistently with `franc` (`detectLanguageProfile` checks `['nld', 'deu', 'fra', 'spa', 'por', 'ita', 'rus', 'pol', 'dan', 'swe', 'nor']`; we should verify `franc` on Norwegian samples during `record`/`diff` of `src/indonesian-nlp.js`).
3. **Script/reliability**: `whatlang::detect(clean)` returns `Option<Info>` with `is_reliable()`. If `None`, return `"und"`. During `rust-anything diff` on `src/indonesian-nlp.js`, verify whether requiring `info.is_reliable()` or taking `info.lang().code()` matches `franc` closer on short social posts.

## Sources
- Local source: `src/indonesian-nlp.js` lines 987–1014, `package.json`
- `whatlang-rs`: https://github.com/greyblake/whatlang-rs & https://docs.rs/whatlang
- `lingua-rs`: https://github.com/pemistahl/lingua-rs & https://crates.io/crates/lingua
