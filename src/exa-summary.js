'use strict';

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const EXA_CONTENTS_URL = 'https://api.exa.ai/contents';
const EXA_ANSWER_URL = 'https://api.exa.ai/answer';

const SUMMARY_SCHEMA = {
  type: 'object',
  properties: {
    ringkasan: { type: 'string' },
    dampak_djp: { type: 'string' },
    entitas_kunci: { type: 'array', items: { type: 'string' } },
    atensi: { type: 'string' },
  },
  required: ['ringkasan', 'dampak_djp', 'entitas_kunci', 'atensi'],
};

const SUMMARY_QUERY = 'Ringkas kejadian perpajakan atau keuangan negara Indonesia ini dalam Bahasa Indonesia baku untuk pimpinan Direktorat Jenderal Pajak (DJP): (1) ringkasan 2-3 kalimat padat atas fakta kejadian; (2) dampak_djp terhadap penerimaan pajak, APBN, Coretax, kepatuhan wajib pajak, atau reputasi DJP; (3) entitas_kunci maksimal 6 item berupa instansi, regulasi/PMK, atau angka penting (Rp/persen); (4) atensi singkat 1 kalimat rekomendasi pemantauan atau respons komunikasi.';

const PRIVATE_HOST_RE = /^(?:localhost|127\.|10\.|192\.168\.|169\.254\.|0\.0\.0\.0|\[::1\]|172\.(?:1[6-9]|2\d|3[01])\.)/i;
const GNEWS_HOST_RE = /(?:^|\.)news\.google\.com$/i;

function isValidPublicHttpUrl(raw) {
  try {
    const u = new URL(String(raw || '').trim());
    if (u.protocol !== 'http:' && u.protocol !== 'https:') return false;
    if (u.username || u.password) return false;
    if (!u.hostname || !u.hostname.includes('.') || PRIVATE_HOST_RE.test(u.hostname)) return false;
    return true;
  } catch (e) {
    return false;
  }
}

function wibDateKey(now = Date.now()) {
  const d = new Date(now + 7 * 3600e3);
  return d.toISOString().slice(0, 10);
}

function hashKey(s) {
  return crypto.createHash('sha1').update(String(s || '')).digest('hex').slice(0, 20);
}

function extractKeyEntities(text) {
  const s = String(text || '');
  const found = [];
  const add = (v) => {
    const clean = String(v || '').trim();
    if (clean && !found.some(x => x.toLowerCase() === clean.toLowerCase())) found.push(clean);
  };
  for (const m of s.matchAll(/\b(?:Rp\s?[\d.,]+\s*(?:triliun|miliar|juta|T|M)\b|\d+(?:[.,]\d+)?%)/gi)) add(m[0]);
  const patterns = [
    /\bCoretax\b/i, /\bDJP\b/, /\bKemenkeu\b/i, /\bMenkeu\b/i, /\bAPBN\b/, /\bPNBP\b/,
    /\bDJBC\b/, /\bBea Cukai\b/i, /\bPPATK\b/, /\bKPP\s+(?:Pratama|Madya|Khusus)\b/i,
    /\bPengadilan Pajak\b/i, /\bKomisi XI\b/i, /\bPPN\b/, /\bPPh(?:\s*(?:21|22|23|25|29|Final|Badan))?\b/i,
    /\bSPT(?:\s+Tahunan)?\b/i, /\bNPWP\b/, /\bPMK\s*(?:No\.?|Nomor)?\s*\d+[^\s,.;]*/i,
    /\bUU\s+HPP\b/i, /\bTax\s+Ratio\b/i, /\bBapenda\b/i, /\bSamsat\b/i,
  ];
  for (const re of patterns) {
    const m = s.match(re);
    if (m) add(m[0]);
    if (found.length >= 6) break;
  }
  return found.slice(0, 6);
}

function buildFallbackSummary({ title = '', desc = '', themeLabel = '', urls = [], reason = '' }) {
  const cleanTitle = String(title || '').trim();
  const cleanDesc = String(desc || '').replace(/\s+/g, ' ').trim();
  const combined = (cleanTitle + ' ' + cleanDesc).trim();
  const entities = extractKeyEntities(combined);
  const ringkasan = cleanDesc && cleanDesc.length > 36
    ? (cleanDesc.length > 340 ? cleanDesc.slice(0, 340).replace(/\s+\S*$/, '') + '…' : cleanDesc)
    : (cleanTitle || 'Ringkasan berita perpajakan tidak tersedia.');
  const dampak = themeLabel
    ? `Terkait dimensi ${themeLabel} pada pemantauan perpajakan & keuangan negara${entities.length ? ' (' + entities.slice(0, 3).join(', ') + ')' : ''}.`
    : 'Memerlukan pemantauan lanjutan terkait implikasi kebijakan fiskal, penerimaan negara, atau layanan wajib pajak DJP.';
  const atensi = /coretax|error|gangguan|down|pidana|sita|perampasan|boikot|ppatk/i.test(combined)
    ? 'Perlu verifikasi teknis/humas segera dan pemantauan sentimen lintas kanal.'
    : 'Pantau perkembangan pemberitaan lanjutan serta respons pemangku kepentingan.';
  return {
    ok: true,
    mode: 'fallback',
    fallbackReason: reason || 'local_extractive',
    ringkasan,
    dampak_djp: dampak,
    entitas_kunci: entities,
    atensi,
    sources: urls.slice(0, 3).map(u => ({ url: u, title: cleanTitle })),
  };
}

function parseExaSummaryObject(rawSummary, fallbackTitle, fallbackDesc) {
  if (rawSummary && typeof rawSummary === 'object') {
    return {
      ringkasan: String(rawSummary.ringkasan || '').trim(),
      dampak_djp: String(rawSummary.dampak_djp || '').trim(),
      entitas_kunci: Array.isArray(rawSummary.entitas_kunci) ? rawSummary.entitas_kunci.map(x => String(x || '').trim()).filter(Boolean).slice(0, 6) : [],
      atensi: String(rawSummary.atensi || '').trim(),
    };
  }
  const str = String(rawSummary || '').trim();
  if (str.startsWith('{')) {
    try {
      const obj = JSON.parse(str);
      if (obj && typeof obj === 'object') return parseExaSummaryObject(obj, fallbackTitle, fallbackDesc);
    } catch (e) {}
  }
  if (str) {
    return {
      ringkasan: str.slice(0, 500),
      dampak_djp: 'Perlu pemantauan dampak terhadap penerimaan pajak, kebijakan fiskal, dan layanan DJP.',
      entitas_kunci: extractKeyEntities(str + ' ' + fallbackTitle),
      atensi: 'Tinjau sumber asli untuk tindak lanjut unit kerja terkait.',
    };
  }
  const fb = buildFallbackSummary({ title: fallbackTitle, desc: fallbackDesc });
  return {
    ringkasan: fb.ringkasan,
    dampak_djp: fb.dampak_djp,
    entitas_kunci: fb.entitas_kunci,
    atensi: fb.atensi,
  };
}

function createExaSummaryService({
  histDir = '',
  fetchFn = null,
  env = process.env,
  persist = true,
  nowFn = () => Date.now(),
} = {}) {
  const cachePath = histDir ? path.join(histDir, 'exa-summary-cache.json') : '';
  const ttlMs = Math.max(60e3, Number(env.EXA_CACHE_TTL_MS) || 12 * 3600e3);
  const briefTtlMs = Math.max(60e3, Number(env.EXA_BRIEF_TTL_MS) || 30 * 60e3);
  const dailyCap = Math.max(1, Number(env.EXA_DAILY_CAP) || 150);
  const maxEntries = 400;

  const memCache = new Map();
  const inFlight = new Map();
  let dailyDate = wibDateKey(nowFn());
  let dailyUsed = 0;
  let cooldownUntil = 0;
  let saveTimer = null;

  function loadDiskCache() {
    if (!persist || !cachePath) return;
    try {
      if (!fs.existsSync(cachePath)) return;
      const raw = JSON.parse(fs.readFileSync(cachePath, 'utf8'));
      const today = wibDateKey(nowFn());
      if (raw && raw.dailyDate === today && Number.isFinite(raw.dailyUsed)) {
        dailyDate = raw.dailyDate;
        dailyUsed = raw.dailyUsed;
      }
      if (raw && Array.isArray(raw.entries)) {
        const now = nowFn();
        for (const [k, v] of raw.entries) {
          if (k && v && v.at && now - v.at < ttlMs) memCache.set(k, v);
        }
      }
    } catch (e) {}
  }

  function scheduleSave() {
    if (!persist || !cachePath) return;
    if (saveTimer) return;
    saveTimer = setTimeout(() => {
      saveTimer = null;
      try {
        fs.mkdirSync(path.dirname(cachePath), { recursive: true });
        const entries = [...memCache.entries()].slice(-maxEntries);
        const tmp = cachePath + '.tmp';
        fs.writeFileSync(tmp, JSON.stringify({ dailyDate, dailyUsed, entries }), 'utf8');
        fs.renameSync(tmp, cachePath);
      } catch (e) {}
    }, 600);
    if (saveTimer.unref) saveTimer.unref();
  }

  function resetDailyIfNeeded() {
    const today = wibDateKey(nowFn());
    if (today !== dailyDate) {
      dailyDate = today;
      dailyUsed = 0;
    }
  }

  function getApiKey() {
    return String(env.EXA_API_KEY || '').trim();
  }

  function status() {
    resetDailyIfNeeded();
    const now = nowFn();
    return {
      ok: true,
      configured: Boolean(getApiKey()),
      dailyUsed,
      dailyCap,
      dailyRemaining: Math.max(0, dailyCap - dailyUsed),
      dailyDate,
      cacheEntries: memCache.size,
      cooldownUntil: cooldownUntil > now ? cooldownUntil : 0,
    };
  }

  function cacheGet(key, customTtl) {
    const hit = memCache.get(key);
    if (!hit) return null;
    if (nowFn() - hit.at > (customTtl || ttlMs)) {
      memCache.delete(key);
      return null;
    }
    memCache.delete(key);
    memCache.set(key, hit);
    return hit.val;
  }

  function cacheSet(key, val) {
    memCache.set(key, { at: nowFn(), val });
    while (memCache.size > maxEntries) {
      memCache.delete(memCache.keys().next().value);
    }
    scheduleSave();
  }

  async function postExa(endpoint, body, timeoutMs = 18000) {
    const key = getApiKey();
    if (!key) throw Object.assign(new Error('EXA_API_KEY belum dikonfigurasi'), { code: 'NO_KEY' });
    resetDailyIfNeeded();
    if (nowFn() < cooldownUntil) throw Object.assign(new Error('Exa API sedang cooldown'), { code: 'COOLDOWN' });
    if (dailyUsed >= dailyCap) throw Object.assign(new Error('Batas harian Exa API tercapai'), { code: 'DAILY_CAP' });

    const doFetch = fetchFn || ((url, opt) => fetch(url, { ...opt, signal: AbortSignal.timeout(timeoutMs) }));
    const res = await doFetch(endpoint, {
      method: 'POST',
      headers: {
        'x-api-key': key,
        'content-type': 'application/json',
        accept: 'application/json',
      },
      body: JSON.stringify(body),
    }, timeoutMs);

    if (!res.ok) {
      const txt = await res.text().catch(() => '');
      if (res.status === 402 || res.status === 429) {
        cooldownUntil = nowFn() + 15 * 60e3;
      }
      const err = new Error(`Exa HTTP ${res.status}: ${txt.slice(0, 140)}`);
      err.status = res.status;
      throw err;
    }
    dailyUsed++;
    scheduleSave();
    return await res.json();
  }

  function pickDirectUrls(urls) {
    const valid = [];
    for (const raw of (Array.isArray(urls) ? urls : [])) {
      const s = String(raw || '').trim();
      if (!isValidPublicHttpUrl(s)) continue;
      if (!valid.includes(s)) valid.push(s);
    }
    valid.sort((a, b) => {
      const ga = GNEWS_HOST_RE.test(new URL(a).hostname) ? 1 : 0;
      const gb = GNEWS_HOST_RE.test(new URL(b).hostname) ? 1 : 0;
      return ga - gb;
    });
    return valid.slice(0, 2);
  }

  async function summarize({ url = '', urls = [], title = '', desc = '', themeLabel = '', clusterId = '' } = {}) {
    const inputUrls = [...(Array.isArray(urls) ? urls : []), ...(url ? [url] : [])];
    const cleanUrls = pickDirectUrls(inputUrls);
    const cacheKey = 'sum:' + hashKey((clusterId ? clusterId + '|' : '') + (cleanUrls.join('|') || title));

    const cached = cacheGet(cacheKey, ttlMs);
    if (cached) return { ...cached, cached: true, exaConfigured: Boolean(getApiKey()) };

    if (inFlight.has(cacheKey)) {
      const shared = await inFlight.get(cacheKey);
      return { ...shared, cached: true, exaConfigured: Boolean(getApiKey()) };
    }

    const run = (async () => {
      const directOnly = cleanUrls.filter(u => !GNEWS_HOST_RE.test(new URL(u).hostname));
      const targetUrls = directOnly.length ? directOnly.slice(0, 2) : cleanUrls.slice(0, 1);

      if (!targetUrls.length || !getApiKey()) {
        return {
          ...buildFallbackSummary({
            title,
            desc,
            themeLabel,
            urls: cleanUrls,
            reason: !getApiKey() ? 'no_exa_key' : 'no_public_url',
          }),
          cached: false,
          exaConfigured: Boolean(getApiKey()),
          at: nowFn(),
        };
      }

      try {
        const data = await postExa(EXA_CONTENTS_URL, {
          urls: targetUrls,
          text: false,
          summary: {
            query: SUMMARY_QUERY,
            schema: SUMMARY_SCHEMA,
          },
          maxAgeHours: 24,
          livecrawlTimeout: 9000,
        }, 18000);

        const results = Array.isArray(data && data.results) ? data.results.filter(r => r && r.summary) : [];
        if (!results.length) {
          return {
            ...buildFallbackSummary({ title, desc, themeLabel, urls: targetUrls, reason: 'empty_exa_result' }),
            cached: false,
            exaConfigured: true,
            at: nowFn(),
          };
        }

        const primary = parseExaSummaryObject(results[0].summary, title || results[0].title, desc);
        const entities = [...primary.entitas_kunci];
        let mergedSummary = primary.ringkasan;
        if (results.length > 1) {
          const second = parseExaSummaryObject(results[1].summary, results[1].title || '', '');
          for (const e of second.entitas_kunci) {
            if (!entities.some(x => x.toLowerCase() === e.toLowerCase()) && entities.length < 6) entities.push(e);
          }
          if (second.ringkasan && mergedSummary.length < 280) {
            mergedSummary = mergedSummary + ' ' + second.ringkasan;
          }
        }

        const out = {
          ok: true,
          mode: 'exa',
          ringkasan: mergedSummary,
          dampak_djp: primary.dampak_djp,
          entitas_kunci: entities.slice(0, 6),
          atensi: primary.atensi,
          sources: results.map(r => ({ url: r.url || targetUrls[0], title: r.title || title || '' })),
          cached: false,
          exaConfigured: true,
          at: nowFn(),
        };
        cacheSet(cacheKey, out);
        return out;
      } catch (e) {
        return {
          ...buildFallbackSummary({
            title,
            desc,
            themeLabel,
            urls: targetUrls,
            reason: e.code || String(e.message || e).slice(0, 80),
          }),
          cached: false,
          exaConfigured: Boolean(getApiKey()),
          at: nowFn(),
        };
      }
    })();

    inFlight.set(cacheKey, run);
    try {
      return await run;
    } finally {
      inFlight.delete(cacheKey);
    }
  }

  async function executiveBrief({ clusters = [], win = '24h', scope = 'nasional', theme = '' } = {}) {
    const topClusters = (Array.isArray(clusters) ? clusters : []).slice(0, 5);
    const sig = topClusters.map(c => c.id || c.title).join('|');
    const cacheKey = 'brief:' + hashKey(`${win}|${scope}|${theme}|${sig}`);

    const cached = cacheGet(cacheKey, briefTtlMs);
    if (cached) return { ...cached, cached: true, exaConfigured: Boolean(getApiKey()) };

    if (inFlight.has(cacheKey)) {
      const shared = await inFlight.get(cacheKey);
      return { ...shared, cached: true, exaConfigured: Boolean(getApiKey()) };
    }

    const run = (async () => {
      const directUrls = [];
      for (const c of topClusters) {
        const candUrls = [c.url, ...((c.newsItems || []).map(n => n && n.url))].filter(Boolean);
        const picked = pickDirectUrls(candUrls).find(u => !GNEWS_HOST_RE.test(new URL(u).hostname) && !directUrls.includes(u));
        if (picked) directUrls.push(picked);
        if (directUrls.length >= 3) break;
      }

      const buildLocalBrief = (reason) => ({
        ok: true,
        mode: 'fallback',
        fallbackReason: reason,
        headline: `Briefing Lanskap ${scope === 'daerah' ? 'Pajak Daerah' : 'Pajak Nasional (DJP) & Keuangan Negara'} (${win.toUpperCase()})`,
        highlights: topClusters.slice(0, 4).map(c => ({
          title: c.title || '',
          ringkasan: c.desc || c.title || '',
          dampak_djp: `${c.themeLabel || 'Perpajakan'} · ${c.totalItems || 1} item (${c.sourceCount || 1} sumber)`,
          url: c.url || '',
          urgency: c.urgency || 'normal',
        })),
        entitas_kunci: extractKeyEntities(topClusters.map(c => c.title + ' ' + (c.desc || '')).join(' ')),
        cached: false,
        exaConfigured: Boolean(getApiKey()),
        at: nowFn(),
      });

      if (!topClusters.length) return buildLocalBrief('no_clusters');
      if (!getApiKey()) return buildLocalBrief('no_exa_key');

      try {
        if (directUrls.length) {
          const data = await postExa(EXA_CONTENTS_URL, {
            urls: directUrls,
            text: false,
            summary: {
              query: SUMMARY_QUERY,
              schema: SUMMARY_SCHEMA,
            },
            maxAgeHours: 24,
            livecrawlTimeout: 9000,
          }, 20000);
          const results = Array.isArray(data && data.results) ? data.results.filter(r => r && r.summary) : [];
          if (results.length) {
            const highlights = [];
            const allEnts = [];
            for (const r of results) {
              const parsed = parseExaSummaryObject(r.summary, r.title || '', '');
              highlights.push({
                title: r.title || '',
                ringkasan: parsed.ringkasan,
                dampak_djp: parsed.dampak_djp,
                atensi: parsed.atensi,
                url: r.url || '',
              });
              for (const e of parsed.entitas_kunci) {
                if (!allEnts.some(x => x.toLowerCase() === e.toLowerCase()) && allEnts.length < 8) allEnts.push(e);
              }
            }
            const out = {
              ok: true,
              mode: 'exa',
              headline: `Ringkasan Eksekutif Exa AI · ${scope === 'daerah' ? 'Pajak Daerah' : 'Pajak Nasional (DJP) & Keuangan Negara'} (${win.toUpperCase()})`,
              highlights,
              entitas_kunci: allEnts,
              cached: false,
              exaConfigured: true,
              at: nowFn(),
            };
            cacheSet(cacheKey, out);
            return out;
          }
        }

        const qTopics = topClusters.slice(0, 3).map(c => c.title).join('; ');
        const ans = await postExa(EXA_ANSWER_URL, {
          query: `Buat ringkasan eksekutif singkat (Bahasa Indonesia baku) untuk Direktorat Jenderal Pajak atas perkembangan berita perpajakan & keuangan negara Indonesia terkait: ${qTopics}`,
          text: false,
        }, 20000);
        const answerText = String((ans && ans.answer) || '').trim();
        if (answerText) {
          const out = {
            ok: true,
            mode: 'exa',
            headline: `Ringkasan Eksekutif Exa AI · ${scope === 'daerah' ? 'Pajak Daerah' : 'Pajak Nasional (DJP) & Keuangan Negara'} (${win.toUpperCase()})`,
            highlights: [{
              title: 'Sintesis Lanskap Fiskal & Perpajakan',
              ringkasan: answerText,
              dampak_djp: 'Sintesis lintas sumber melalui Exa AI Answer.',
              url: ((ans.citations && ans.citations[0] && ans.citations[0].url) || ''),
            }],
            entitas_kunci: extractKeyEntities(answerText),
            cached: false,
            exaConfigured: true,
            at: nowFn(),
          };
          cacheSet(cacheKey, out);
          return out;
        }
        return buildLocalBrief('empty_exa_brief');
      } catch (e) {
        return buildLocalBrief(e.code || String(e.message || e).slice(0, 80));
      }
    })();

    inFlight.set(cacheKey, run);
    try {
      return await run;
    } finally {
      inFlight.delete(cacheKey);
    }
  }

  loadDiskCache();

  return {
    status,
    summarize,
    executiveBrief,
    isValidPublicHttpUrl,
    extractKeyEntities,
  };
}

module.exports = {
  createExaSummaryService,
  isValidPublicHttpUrl,
  extractKeyEntities,
  buildFallbackSummary,
  parseExaSummaryObject,
};
