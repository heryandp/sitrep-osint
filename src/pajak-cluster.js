'use strict';

const GEO = require('./geocode');

const TAX_RE = /(?:\bcoretax\b|core\s*tax|\bdjp\b|ditjen\s+pajak|direktorat\s+jenderal\s+pajak|kring\s+pajak|kanwil\s+djp|\bkpp\s+(?:pratama|madya|wajib\s+pajak|khusus|bumn|pma)\b|kantor\s+pelayanan\s+pajak|perpajakan|wajib\s+pajak|(?:uang|bayar|tarif|kenaikan|aturan|kebijakan|bebas|insentif|setor|lapor|kepatuhan|literasi|penerimaan|setoran|tunggakan|restitusi|pemeriksaan|penagihan|pidana|pengemplang|pengampunan|keringanan|relaksasi|sengketa|boikot|tolak|keadilan)\s+pajak|tax\s+(?:amnesty|ratio|center|holiday|allowance)|rasio\s+pajak|faktur\s+pajak|e-?faktur|e-?bupot|e-?filing|e-?billing|\bspt\s+(?:tahunan|masa|pph|ppn)\b|lapor\s+spt|pelaporan\s+spt|\bnpwp\b|nik\s*(?:jadi|menjadi)?\s*npwp|\bppn\s*(?:\d{1,2}%|bm)?\b|\bppnbm\b|\bpph\s*(?:pasal\s*\d+|21|22|23|25|26|29|final|badan|orang\s+pribadi)?\b|pajak\s+(?:penghasilan|pertambahan\s+nilai|bumi\s+dan\s+bangunan|kendaraan|daerah|karbon|digital|kripto|natura|umkm|dividen|progresif|impor|ekspor|hiburan|warisan)|ter\s+pph|pengadilan\s+pajak|konsultan\s+pajak|fiskus|gijzeling|penyanderaan\s+pajak|sita\s+aset\s+pajak|keuangan\s+negara|\bapbn\b|defisit\s+apbn|surplus\s+apbn|penerimaan\s+negara|belanja\s+negara|utang\s+negara|\bpnbp\b|\bdjbc\b|bea\s+(?:dan\s+)?cukai|\bdja\b|\bdjppr\b|kemenkeu|kementerian\s+keuangan|\bmenkeu\b|menteri\s+keuangan|kebijakan\s+fiskal|surat\s+berharga\s+negara|\bsbn\b|\bpmk\s*(?:no\.?|nomor)?\s*\d+)/i;

const TAX_NOISE_RE = /(?:pajak\s+(?:jadian|nikah|jodoh|ganteng|cantik|makan\s+bareng)|judi\s+online|slot\s+gacor|drakor|k-?pop)/i;

const TAX_NASIONAL_RE = /(?:\bdjp\b|ditjen\s+pajak|direktorat\s+jenderal\s+pajak|dirjen\s+pajak|kanwil\s+djp|\bkpp\s+(?:pratama|madya|wajib\s+pajak|khusus|bumn|pma)\b|kantor\s+pelayanan\s+pajak|kring\s+pajak|\bcoretax\b|core\s*tax|djp\s+online|e-?faktur|e-?bupot|e-?filing|e-?billing|faktur\s+pajak|\bspt\s+(?:tahunan|masa|pph|ppn)\b|lapor\s+spt|pelaporan\s+spt|\bnpwp\b|nik\s*(?:jadi|menjadi)?\s*npwp|\bppn\s*(?:\d{1,2}%|bm)?\b|\bppnbm\b|\bpph\s*(?:pasal\s*\d+|21|22|23|25|26|29|final|badan|orang\s+pribadi)?\b|ter\s+pph|bea\s+meterai|pajak\s+penghasilan|pajak\s+pertambahan\s+nilai|pajak\s+(?:digital|kripto|karbon|natura|dividen|impor|ekspor|pusat)|kemenkeu|kementerian\s+keuangan|\bmenkeu\b|menteri\s+keuangan|keuangan\s+negara|\bapbn\b|defisit\s+apbn|surplus\s+apbn|penerimaan\s+negara|belanja\s+negara|utang\s+negara|\bpnbp\b|\bdjbc\b|bea\s+(?:dan\s+)?cukai|\bdja\b|\bdjppr\b|kebijakan\s+fiskal|surat\s+berharga\s+negara|\bsbn\b|uu\s+hpp|\bpmk\s*(?:no\.?|nomor)?\s*\d+|tax\s+(?:ratio|amnesty|holiday|allowance)|rasio\s+pajak|pengadilan\s+pajak|badan\s+penerimaan\s+negara|restitusi\s+pajak|pidana\s+pajak|faktur\s+fiktif|gijzeling|penyanderaan\s+pajak|program\s+psa|dinamisasi|komisi\s+xi|ppatk|ddtc|ortax)/i;

const TAX_DAERAH_RE = /(?:pajak\s+daerah|pajak\s+kendaraan(?:\s+bermotor)?|\bpkb\b|\bbbnkb\b|pemutihan\s+pajak|keringanan\s+(?:pokok\s+)?pajak\s+kendaraan|\bsamsat\b|\bbapenda\b|\bdispenda\b|\bbappenda\b|\bbpkad\b|\bbphtb\b|\bpbb[- ]?p2\b|pajak\s+bumi\s+dan\s+bangunan|pajak\s+(?:hotel|restoran|hiburan|parkir|reklame|air\s+tanah|penerangan\s+jalan|sarang\s+burung\s+walet|mineral\s+bukan\s+logam|mblb|pbjt)|\bpbjt\b|pendapatan\s+asli\s+daerah|\bpad\b|perda\s+pajak|pemprov|pemkab|pemkot|pemda|gubernur|bupati|wali\s*kota|opsen\s+(?:pkb|bbnkb|pajak))/i;

const PAJAK_THEMES = Object.freeze([
  {
    id: 'ideologi',
    label: 'Ideologi',
    short: 'Ideologi',
    desc: 'Keadilan fiskal, nasionalisme pajak, gotong royong, dan narasi boikot/anti-pajak',
    color: '#fb7185',
    lightColor: '#be123c',
    strong: /(?:keadilan\s+(?:pajak|fiskal)|boikot\s+pajak|tolak\s+bayar\s+pajak|mogok\s+bayar\s+pajak|anti[- ]?pajak|bela\s+negara|pancasila|pasal\s+23\s*a|konstitusi|kedaulatan\s+fiskal|gotong\s+royong|pajak\s+kuat\s+indonesia\s+maju|pajak\s+untuk\s+rakyat|ketimpangan\s+(?:pajak|fiskal)|pajak\s+berkeadilan|ke\s*mana\s+(?:sebenarnya\s+)?uang\s+pajak)/i,
    base: /(?:ideologi|nasionalisme|kedaulatan|keadilan|pemerataan|rakyat\s+kecil|kaum\s+kaya|orang\s+kaya|konglomerat|oligarki|ketimpangan|solidaritas|kontribusi\s+untuk\s+negara|pembangunan\s+bangsa)/i,
  },
  {
    id: 'politik',
    label: 'Politik',
    short: 'Politik',
    desc: 'Kebijakan DPR/Komisi XI, sikap fraksi/partai, Menkeu, RUU/UU HPP, dan protes kebijakan tarif',
    color: '#60a5fa',
    lightColor: '#1d4ed8',
    strong: /(?:\bdpr\b|\bdprd\b|komisi\s+xi|fraksi|partai|parlemen|senayan|presiden|wapres|menteri\s+keuangan|\bmenkeu\b|dirjen\s+pajak|badan\s+penerimaan\s+negara|\bbpn\b|uu\s+hpp|ruu\s+pajak|perppu|\bpmk\s*(?:no\.?|nomor)?\s*\d+|unjuk\s+rasa|demo\s+tolak|aksi\s+tolak|protes\s+(?:ppn|pajak)|kebijakan\s+fiskal|pemprov|gubernur|bupati|wali\s*kota)/i,
    base: /(?:kebijakan|regulasi|aturan\s+pajak|pemerintah|kabinet|istana|koalisi|oposisi|pilkada|pemilu|apbd|perda\s+pajak|keputusan\s+menteri|dpr\s+ri|kritik\s+kebijakan|polemik)/i,
  },
  {
    id: 'sosbud',
    label: 'Sosial dan Budaya',
    short: 'Sosial & Budaya',
    desc: 'Kepatuhan sukarela, literasi/Tax Center, kepercayaan publik, UMKM, layanan KPP, dan persepsi warga',
    color: '#fbbf24',
    lightColor: '#9a4208',
    strong: /(?:tax\s+center|literasi\s+pajak|edukasi\s+pajak|sosialisasi\s+(?:pajak|pmk|coretax|spt)|kepatuhan\s+(?:pajak|wajib\s+pajak)|kepercayaan\s+publik|pelayanan\s+(?:pajak|kpp)|kring\s+pajak|antrean\s+(?:kpp|wajib\s+pajak)|konsultan\s+pajak|ikpi|kampus|universitas|mahasiswa|sdgs|gaya\s+hidup\s+(?:pegawai|pejabat)\s+pajak|kelas\s+menengah|kesadaran\s+pajak|budaya\s+(?:bayar\s+)?pajak)/i,
    base: /(?:masyarakat|warga|publik|umkm|pekerja|karyawan|buruh|komunitas|mahasiswa|pelajar|keluhan\s+wajib\s+pajak|wajib\s+pajak|sinergi|pelatihan|seminar|webinar|lapor\s+spt|spt\s+tahunan|keringanan\s+(?:pokok\s+)?pajak|pemutihan\s+pajak)/i,
  },
  {
    id: 'ekonomi',
    label: 'Ekonomi',
    short: 'Ekonomi',
    desc: 'Penerimaan pajak, tax ratio, dinamisasi setoran, APBN, insentif fiskal, PPN/PPh, dan dampak industri',
    color: '#34d399',
    lightColor: '#047857',
    strong: /(?:penerimaan\s+pajak|setoran\s+pajak|realisasi\s+pajak|target\s+pajak|tax\s+ratio|rasio\s+pajak|shortfall|surplus\s+apbn|defisit\s+apbn|dinamisasi|restitusi\s+pajak|insentif\s+pajak|tax\s+holiday|tax\s+allowance|relaksasi\s+pajak|tarif\s+(?:ppn|pph)|ppn\s*12%|pph\s*(?:21|22|23|25|29|final|badan)|ter\s+pph|sektor\s+industri|daya\s+beli|pertumbuhan\s+ekonomi)/i,
    base: /(?:triliun|miliar|apbn|ekonomi|inflasi|investasi|ekspor|impor|emiten|dividen|saham|industri|manufaktur|perdagangan|bisnis|pengusaha|korporasi|kripto|aset\s+kripto|komoditas|tambang|sawit|nikel|batu\s+bara|bea\s+cukai|program\s+psa)/i,
  },
  {
    id: 'hukum',
    label: 'Hukum dan HAM',
    short: 'Hukum & HAM',
    desc: 'Pidana pajak, PPATK, sita/perampasan aset, gijzeling, Pengadilan Pajak, korupsi/OTT, dan hak wajib pajak',
    color: '#c084fc',
    lightColor: '#6d28d9',
    strong: /(?:pidana\s+pajak|ppatk|penggelapan\s+pajak|pengemplang\s+pajak|faktur\s+(?:pajak\s+)?fiktif|pengadilan\s+pajak|sengketa\s+pajak|perampasan\s+aset|harta\s+(?:bisa\s+)?dirampas|sita\s+aset|penyitaan\s+aset|gijzeling|penyanderaan\s+pajak|pemeriksaan\s+pajak|penagihan\s+pajak|tunggakan\s+pajak|suap\s+pajak|korupsi\s+pajak|ott\s+(?:pegawai|pejabat)\s+pajak|hak\s+wajib\s+pajak|uu\s+pdp|privasi\s+data|penipuan\s+(?:mengatasnamakan\s+)?djp)/i,
    base: /(?:hukum|pidana|tersangka|terdakwa|vonis|putusan|kasasi|kejaksaan|kpk|polri|polisi|penyidikan|sanksi|denda\s+pajak|pemblokiran\s+rekening|blokir\s+rekening|cekal|pencekalan|ham\b|hak\s+asasi|keberatan|banding|peninjauan\s+kembali|penipuan|modus|phishing)/i,
  },
  {
    id: 'teknologi',
    label: 'Teknologi',
    short: 'Teknologi',
    desc: 'Coretax DJP, gangguan/error sistem, e-Faktur, e-Bupot, NIK–NPWP 16 digit, PJAP, dan keamanan siber pajak',
    color: '#22d3ee',
    lightColor: '#0b637c',
    strong: /(?:\bcoretax\b|core\s*tax|e-?faktur|e-?bupot|e-?filing|e-?billing|djp\s+online|nik\s*(?:jadi|menjadi)?\s*npwp|npwp\s*16\s*digit|pjap|pajak\s+digital|apk\s+palsu\s+(?:djp|pajak|coretax)|kebocoran\s+data\s+(?:npwp|pajak|djp)|peretasan\s+(?:djp|coretax|pajak)|sistem\s+(?:coretax|perpajakan|djp))/i,
    base: /(?:aplikasi|sistem|server|website|situs|portal|login|otp|error|eror|down|gangguan|lemot|lambat|timeout|bug|crash|maintenance|pemeliharaan|digital|digitalisasi|otomasi|api\b|cloud|siber|cyber|ransomware|phishing|malware|peretasan|data\s+breach|enkripsi|sertifikat\s+elektronik)/i,
  },
]);

const THEME_BY_ID = Object.fromEntries(PAJAK_THEMES.map(t => [t.id, t]));

const URGENT_CRIT_RE = /(?:coretax\s+(?:error|eror|down|lumpuh|gangguan|gagal|crash)|(?:error|eror|down|lumpuh|gangguan)\s+coretax|pidana\s+pajak|penggelapan\s+pajak|faktur\s+fiktif|ott\b|suap\s+pajak|korupsi\s+pajak|boikot\s+pajak|tolak\s+bayar\s+pajak|demo\s+tolak|unjuk\s+rasa|kebocoran\s+data|peretasan|apk\s+palsu|phishing)/i;
const URGENT_WARN_RE = /(?:gangguan|kendala|keluhan|antrean|tunggakan|sita\s+aset|dirampas|perampasan\s+aset|gijzeling|penyanderaan|sengketa|restitusi|pemeriksaan|penagihan|denda|sanksi|tarif\s+naik|kenaikan\s+ppn|ppn\s*12%|shortfall|protes|polemik)/i;

const CLUSTER_STOP = new Set((
  'yang dan di ke dari untuk pada dengan adalah ini itu ada tidak akan juga telah sudah bisa lebih sangat ' +
  'para oleh hari kini tadi baru saja info update berita kabar terkini viral breaking news ' +
  'pajak perpajakan djp ditjen direktorat jenderal kementerian keuangan kemenkeu indonesia nasional ' +
  'wajib tahun bulan hari ini sebut tegas minta dorong siap terus jadi bagi soal terkait melalui hingga'
).split(' '));

function isTaxText(text) {
  const s = String(text || '').trim();
  if (!s || TAX_NOISE_RE.test(s)) return false;
  return TAX_RE.test(s);
}

function classifyTaxScope(text) {
  const s = String(text || '');
  const nasHits = s.match(new RegExp(TAX_NASIONAL_RE.source, 'gi'));
  const daeHits = s.match(new RegExp(TAX_DAERAH_RE.source, 'gi'));
  const n = nasHits ? nasHits.length : 0;
  const d = daeHits ? daeHits.length : 0;
  if (d > 0 && n === 0) return 'daerah';
  if (d > 0 && n > 0) {
    const coreCentral = /\b(?:djp|ditjen\s+pajak|direktorat\s+jenderal\s+pajak|coretax|core\s*tax|pph|ppn|spt|npwp|e-?faktur|e-?bupot|kemenkeu|menkeu|apbn|tax\s+ratio|pengadilan\s+pajak|kanwil\s+djp|kpp\s+(?:pratama|madya))\b/i.test(s);
    return coreCentral ? 'nasional' : (d > n ? 'daerah' : 'nasional');
  }
  return 'nasional';
}

function classifyPajakThemes(text) {
  const s = String(text || '');
  const scores = [];
  for (const t of PAJAK_THEMES) {
    let sc = 0;
    const strongHits = s.match(new RegExp(t.strong.source, 'gi'));
    const baseHits = s.match(new RegExp(t.base.source, 'gi'));
    if (strongHits) sc += Math.min(3, strongHits.length) * 3;
    if (baseHits) sc += Math.min(3, baseHits.length) * 1.25;
    if (sc > 0) scores.push({ id: t.id, score: sc });
  }
  scores.sort((a, b) => b.score - a.score);
  if (!scores.length) {
    if (/coretax|e-?faktur|e-?bupot|e-?filing|e-?billing|npwp/i.test(s)) {
      return { primary: 'teknologi', secondary: [], scores: { teknologi: 1 } };
    }
    if (/kpp|kanwil|wajib\s+pajak|spt/i.test(s)) {
      return { primary: 'sosbud', secondary: [], scores: { sosbud: 1 } };
    }
    return { primary: 'ekonomi', secondary: [], scores: { ekonomi: 1 } };
  }
  const primary = scores[0].id;
  const secondary = scores.slice(1).filter(x => x.score >= 2).slice(0, 2).map(x => x.id);
  const scoreMap = Object.fromEntries(scores.map(x => [x.id, Number(x.score.toFixed(2))]));
  return { primary, secondary, scores: scoreMap };
}

function tokenize(text) {
  const norm = String(text || '')
    .toLowerCase()
    .replace(/https?:\/\/\S+/g, ' ')
    .replace(/@[\w.]+/g, ' ')
    .replace(/[^\p{L}\p{N}]+/gu, ' ')
    .trim();
  if (!norm) return [];
  const out = [];
  const seen = new Set();
  for (const w of norm.split(/\s+/)) {
    if (w.length < 3 || /^\d+$/.test(w) || CLUSTER_STOP.has(w) || seen.has(w)) continue;
    seen.add(w);
    out.push(w);
  }
  return out;
}

function tokenOverlap(a, b) {
  if (!a.length || !b.length) return { shared: 0, jaccard: 0 };
  const setA = new Set(a);
  let shared = 0;
  for (const w of b) if (setA.has(w)) shared++;
  const union = setA.size + b.length - shared;
  return { shared, jaccard: union > 0 ? shared / union : 0 };
}

function windowCutoff(win, now) {
  if (win === '2h') return now - 2 * 3600e3;
  if (win === 'today') {
    const wib = new Date(now + 7 * 3600e3);
    return Date.UTC(wib.getUTCFullYear(), wib.getUTCMonth(), wib.getUTCDate()) - 7 * 3600e3;
  }
  return now - 24 * 3600e3;
}

function normalizeItems(newsItems, socialPosts, cutoff, now) {
  const out = [];
  const seenKey = new Set();

  for (const it of (newsItems || [])) {
    if (!it || !it.title) continue;
    const ts = Number(it.ts) || 0;
    if (ts && (ts < cutoff || ts > now + 10 * 60e3)) continue;
    const text = [it.title, it.desc || ''].filter(Boolean).join(' ');
    const taggedTax = it.cat === 'pajak' || (it.assessment && Array.isArray(it.assessment.topics) &&
      (it.assessment.topics.includes('pajak') || it.assessment.topics.includes('coretax')));
    if (!taggedTax && !isTaxText(text)) continue;
    if (TAX_NOISE_RE.test(text)) continue;
    const dedupeKey = 'n:' + String(it.url || it.title).trim().toLowerCase();
    if (seenKey.has(dedupeKey)) continue;
    seenKey.add(dedupeKey);

    const cls = classifyPajakThemes(text);
    const taxScope = classifyTaxScope(text);
    const loc = (it.la && it.lo && it.name)
      ? { name: it.name, la: it.la, lo: it.lo, prov: it.prov || '' }
      : GEO.locate(it.title);

    out.push({
      kind: 'news',
      id: dedupeKey,
      title: String(it.title).trim(),
      desc: String(it.desc || '').trim(),
      url: it.url || '',
      src: String(it.pub || it.src || 'Media').trim(),
      ts: ts || now,
      theme: cls.primary,
      secondaryThemes: cls.secondary,
      taxScope,
      tokens: tokenize(it.title),
      place: loc ? loc.name : '',
      prov: loc ? (loc.prov || '') : '',
      la: loc && typeof loc.la === 'number' ? loc.la : null,
      lo: loc && typeof loc.lo === 'number' ? loc.lo : null,
      urgentCrit: URGENT_CRIT_RE.test(text),
      urgentWarn: URGENT_WARN_RE.test(text),
    });
  }

  for (const p of (socialPosts || [])) {
    if (!p) continue;
    const txt = String(p.text || p.title || '').trim();
    if (!txt) continue;
    const ts = Number(p.ts) || (p.created_at ? Date.parse(p.created_at) : 0) || 0;
    if (ts && (ts < cutoff || ts > now + 10 * 60e3)) continue;
    const topics = (p.assessment && Array.isArray(p.assessment.topics)) ? p.assessment.topics : (Array.isArray(p.topics) ? p.topics : []);
    const taggedTax = topics.includes('pajak') || topics.includes('coretax');
    if (!taggedTax && !isTaxText(txt)) continue;
    if (TAX_NOISE_RE.test(txt)) continue;
    const handle = String(p.username || p.handle || p.user || '').replace(/^@/, '').trim();
    const dedupeKey = 's:' + String(p.url || p.code || (handle + ':' + txt.slice(0, 80))).toLowerCase();
    if (seenKey.has(dedupeKey)) continue;
    seenKey.add(dedupeKey);

    const cls = classifyPajakThemes(txt);
    const taxScope = classifyTaxScope(txt);
    const loc = GEO.locate(txt);

    out.push({
      kind: 'social',
      platform: p.from || (p.code ? 'Threads' : 'X'),
      id: dedupeKey,
      title: txt.replace(/\s+/g, ' ').slice(0, 220),
      desc: txt,
      url: p.url || '',
      src: handle ? '@' + handle : (p.from || 'Sosial'),
      handle,
      author: String(p.name || p.author || handle || 'Warganet').trim(),
      provenance: p.assessment && p.assessment.provenance ? p.assessment.provenance.sourceType : 'anonymous',
      ts: ts || now,
      theme: cls.primary,
      secondaryThemes: cls.secondary,
      taxScope,
      tokens: tokenize(txt),
      place: loc ? loc.name : '',
      prov: loc ? (loc.prov || '') : '',
      la: loc && typeof loc.la === 'number' ? loc.la : null,
      lo: loc && typeof loc.lo === 'number' ? loc.lo : null,
      urgentCrit: URGENT_CRIT_RE.test(txt),
      urgentWarn: URGENT_WARN_RE.test(txt),
    });
  }

  out.sort((a, b) => (b.ts - a.ts) || (a.kind === 'news' ? -1 : 1));
  return out;
}

function buildPajakClusters({ newsItems = [], socialPosts = [], now = Date.now(), win = '24h', theme = '', scope = '' } = {}) {
  const cutoff = windowCutoff(win, now);
  const items = normalizeItems(newsItems, socialPosts, cutoff, now);
  const clusters = [];

  for (const it of items) {
    let best = null;
    let bestScore = 0;
    for (const cl of clusters) {
      if (cl.theme !== it.theme || cl.taxScope !== it.taxScope) continue;
      if (Math.abs(cl.latestTs - it.ts) > 18 * 3600e3) continue;
      const ov = tokenOverlap(it.tokens, cl._tokens);
      const samePlace = !!(it.place && cl.place && it.place.toLowerCase() === cl.place.toLowerCase());
      const match = ov.shared >= 2 || ov.jaccard >= 0.32 || (samePlace && ov.shared >= 1);
      if (!match) continue;
      const sc = ov.shared * 2 + ov.jaccard * 3 + (samePlace ? 1 : 0);
      if (sc > bestScore) {
        bestScore = sc;
        best = cl;
      }
    }
    if (best) {
      best._items.push(it);
      for (const w of it.tokens) if (best._tokens.length < 24 && !best._tokens.includes(w)) best._tokens.push(w);
      if (it.ts > best.latestTs) best.latestTs = it.ts;
      if (it.ts < best.firstTs) best.firstTs = it.ts;
      if (!best.place && it.place) { best.place = it.place; best.prov = it.prov; best.la = it.la; best.lo = it.lo; }
      for (const st of it.secondaryThemes) if (st !== best.theme && !best.secondaryThemes.includes(st)) best.secondaryThemes.push(st);
    } else {
      clusters.push({
        id: 'pj-' + it.theme + '-' + (clusters.length + 1),
        theme: it.theme,
        taxScope: it.taxScope,
        secondaryThemes: it.secondaryThemes.slice(),
        place: it.place,
        prov: it.prov,
        la: it.la,
        lo: it.lo,
        latestTs: it.ts,
        firstTs: it.ts,
        _tokens: it.tokens.slice(0, 16),
        _items: [it],
      });
    }
  }

  const finalized = clusters.map(cl => {
    const newsList = cl._items.filter(x => x.kind === 'news').sort((a, b) => b.ts - a.ts);
    const socialList = cl._items.filter(x => x.kind === 'social').sort((a, b) => b.ts - a.ts);
    const srcSet = new Set(cl._items.map(x => x.src.toLowerCase()));
    const rep = newsList[0] || socialList[0];
    const hasCrit = cl._items.some(x => x.urgentCrit);
    const hasWarn = cl._items.some(x => x.urgentWarn);
    const crossVerified = newsList.length > 0 && socialList.length > 0;
    const urgency = (hasCrit || srcSet.size >= 4) ? 'kritis'
      : (hasWarn || crossVerified || srcSet.size >= 2) ? 'waspada'
      : 'normal';
    const urgencyRank = urgency === 'kritis' ? 3 : urgency === 'waspada' ? 2 : 1;
    const scopeBonus = cl.taxScope === 'nasional' ? 60 : 0;
    const score = urgencyRank * 100 + scopeBonus + (crossVerified ? 40 : 0) + srcSet.size * 12 + cl._items.length * 4;

    const kwCount = new Map();
    for (const it of cl._items) {
      for (const w of it.tokens) kwCount.set(w, (kwCount.get(w) || 0) + 1);
    }
    const keywords = [...kwCount.entries()]
      .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
      .slice(0, 5)
      .map(([w]) => w);

    const themeMeta = THEME_BY_ID[cl.theme] || PAJAK_THEMES[0];
    return {
      id: cl.id,
      theme: cl.theme,
      themeLabel: themeMeta.label,
      taxScope: cl.taxScope,
      taxScopeLabel: cl.taxScope === 'daerah' ? 'Pajak Daerah' : 'Pajak Nasional · DJP',
      secondaryThemes: cl.secondaryThemes.slice(0, 2),
      urgency,
      score,
      title: rep.title,
      desc: rep.desc && rep.desc !== rep.title ? rep.desc.slice(0, 240) : '',
      url: rep.url,
      repKind: rep.kind,
      place: cl.place || '',
      prov: cl.prov || '',
      la: cl.la,
      lo: cl.lo,
      latestTs: cl.latestTs,
      firstTs: cl.firstTs,
      totalItems: cl._items.length,
      newsCount: newsList.length,
      socialCount: socialList.length,
      sourceCount: srcSet.size,
      crossVerified,
      keywords,
      newsItems: newsList.slice(0, 10).map(x => ({
        title: x.title,
        url: x.url,
        src: x.src,
        ts: x.ts,
        place: x.place,
      })),
      socialItems: socialList.slice(0, 10).map(x => ({
        title: x.title,
        text: x.desc,
        url: x.url,
        src: x.src,
        handle: x.handle,
        author: x.author,
        platform: x.platform,
        provenance: x.provenance,
        ts: x.ts,
        place: x.place,
      })),
    };
  });

  finalized.sort((a, b) => (b.score - a.score) || (b.latestTs - a.latestTs));

  const filterScope = (scope === 'nasional' || scope === 'daerah') ? scope : '';
  const scopedItems = filterScope ? items.filter(x => x.taxScope === filterScope) : items;
  const scopedClusters = filterScope ? finalized.filter(c => c.taxScope === filterScope) : finalized;

  const totalItems = scopedItems.length;
  const totalNews = scopedItems.filter(x => x.kind === 'news').length;
  const totalSocial = scopedItems.filter(x => x.kind === 'social').length;
  const totalNasional = items.filter(x => x.taxScope === 'nasional').length;
  const totalDaerah = items.filter(x => x.taxScope === 'daerah').length;
  const nasionalClusters = finalized.filter(c => c.taxScope === 'nasional').length;
  const daerahClusters = finalized.filter(c => c.taxScope === 'daerah').length;

  const themes = PAJAK_THEMES.map(t => {
    const tClusters = scopedClusters.filter(c => c.theme === t.id);
    const tItems = scopedItems.filter(x => x.theme === t.id);
    const tNews = tItems.filter(x => x.kind === 'news').length;
    const tSocial = tItems.filter(x => x.kind === 'social').length;
    const tNas = tClusters.filter(c => c.taxScope === 'nasional').length;
    const tDae = tClusters.filter(c => c.taxScope === 'daerah').length;
    const urgentCount = tClusters.filter(c => c.urgency === 'kritis' || c.urgency === 'waspada').length;
    const regMap = new Map();
    for (const it of tItems) {
      if (it.place) regMap.set(it.place, (regMap.get(it.place) || 0) + 1);
    }
    const topRegions = [...regMap.entries()]
      .sort((a, b) => b[1] - a[1])
      .slice(0, 3)
      .map(([name, count]) => ({ name, count }));
    return {
      id: t.id,
      label: t.label,
      short: t.short,
      desc: t.desc,
      color: t.color,
      lightColor: t.lightColor,
      totalItems: tItems.length,
      newsCount: tNews,
      socialCount: tSocial,
      clusterCount: tClusters.length,
      nasionalClusters: tNas,
      daerahClusters: tDae,
      urgentCount,
      sharePct: totalItems > 0 ? Math.round((tItems.length / totalItems) * 100) : 0,
      latestTs: tClusters.length ? Math.max(...tClusters.map(c => c.latestTs)) : 0,
      topRegions,
    };
  });

  const filterTheme = theme && THEME_BY_ID[theme] ? theme : '';
  const visibleClusters = filterTheme
    ? scopedClusters.filter(c => c.theme === filterTheme || c.secondaryThemes.includes(filterTheme))
    : scopedClusters;

  return {
    ok: true,
    at: now,
    win: ['24h', 'today', '2h'].includes(win) ? win : '24h',
    theme: filterTheme,
    scope: filterScope,
    totalItems,
    totalNews,
    totalSocial,
    totalNasional,
    totalDaerah,
    nasionalClusters,
    daerahClusters,
    totalClusters: scopedClusters.length,
    crossVerifiedCount: scopedClusters.filter(c => c.crossVerified).length,
    themes,
    clusters: visibleClusters.slice(0, 80),
  };
}

module.exports = {
  PAJAK_THEMES,
  THEME_BY_ID,
  isTaxText,
  classifyTaxScope,
  classifyPajakThemes,
  buildPajakClusters,
};
