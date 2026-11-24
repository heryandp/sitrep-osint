'use strict';

const ISDOWN_API = 'https://isdownapi.com/api/check?domain=';

const DJP_PORTALS = Object.freeze([
  { id: 'pajak', name: 'Pajak.go.id', domain: 'pajak.go.id', url: 'https://www.pajak.go.id' },
  { id: 'coretax', name: 'Coretax DJP', domain: 'coretaxdjp.pajak.go.id', url: 'https://coretaxdjp.pajak.go.id' },
  { id: 'djponline', name: 'DJP Online', domain: 'djponline.pajak.go.id', url: 'https://djponline.pajak.go.id' },
  { id: 'kemenkeu', name: 'Kemenkeu', domain: 'kemenkeu.go.id', url: 'https://www.kemenkeu.go.id' },
]);

const SLOW_MS = 2500;

function normalizeCheckResult(portal, raw, now = Date.now()) {
  const statusRaw = String((raw && raw.status) || '').toLowerCase();
  const ms = raw && Number.isFinite(Number(raw.response_time_ms)) && raw.response_time_ms !== null
    ? Math.round(Number(raw.response_time_ms))
    : null;
  const httpCode = raw && raw.http_code != null ? Number(raw.http_code) : null;
  const isUp = statusRaw === 'up';
  const state = !isUp ? 'down' : (ms != null && ms >= SLOW_MS ? 'slow' : 'up');
  return {
    id: portal.id,
    name: portal.name,
    domain: portal.domain,
    url: portal.url,
    status: isUp ? 'up' : 'down',
    state,
    httpCode: Number.isFinite(httpCode) ? httpCode : null,
    responseMs: ms,
    ip: raw && raw.ip ? String(raw.ip) : null,
    error: raw && raw.error ? String(raw.error).slice(0, 140) : null,
    checkedAt: (raw && raw.checked_at && Date.parse(raw.checked_at)) || now,
  };
}

function createUptimeService({
  portals = DJP_PORTALS,
  fetchFn = null,
  ttlMs = 90000,
  nowFn = () => Date.now(),
} = {}) {
  let cache = null;
  let cacheAt = 0;
  let inFlight = null;
  const history = new Map();

  function recordHistory(id, state) {
    const arr = history.get(id) || [];
    arr.push(state);
    while (arr.length > 12) arr.shift();
    history.set(id, arr);
    return [...arr];
  }

  async function checkOne(portal) {
    const url = ISDOWN_API + encodeURIComponent(portal.domain);
    const doFetch = fetchFn || ((u, opt, ms) => fetch(u, { ...opt, signal: AbortSignal.timeout(ms || 12000) }));
    try {
      const r = await doFetch(url, { headers: { accept: 'application/json' } }, 12000);
      if (!r.ok) {
        return {
          ...normalizeCheckResult(portal, { status: 'down', error: 'isdownapi HTTP ' + r.status }, nowFn()),
          probeError: true,
        };
      }
      const data = await r.json();
      return normalizeCheckResult(portal, data, nowFn());
    } catch (e) {
      return {
        ...normalizeCheckResult(portal, { status: 'down', error: String(e && e.message || e).slice(0, 120) }, nowFn()),
        probeError: true,
      };
    }
  }

  async function checkAll(force = false) {
    const now = nowFn();
    if (!force && cache && now - cacheAt < ttlMs) {
      return { ...cache, cached: true };
    }
    if (inFlight) return await inFlight;

    inFlight = (async () => {
      const items = await Promise.all(portals.map(p => checkOne(p)));
      const enriched = items.map(it => ({
        ...it,
        history: recordHistory(it.id, it.state),
      }));
      const upCount = enriched.filter(x => x.status === 'up').length;
      const downCount = enriched.filter(x => x.status !== 'up').length;
      const slowCount = enriched.filter(x => x.state === 'slow').length;
      const payload = {
        ok: true,
        provider: 'isdownapi.com',
        total: enriched.length,
        upCount,
        slowCount,
        downCount,
        items: enriched,
        at: nowFn(),
        cached: false,
      };
      cache = payload;
      cacheAt = payload.at;
      return payload;
    })();

    try {
      return await inFlight;
    } finally {
      inFlight = null;
    }
  }

  return {
    portals,
    checkAll,
    getCached: () => cache,
  };
}

module.exports = {
  DJP_PORTALS,
  SLOW_MS,
  normalizeCheckResult,
  createUptimeService,
};
