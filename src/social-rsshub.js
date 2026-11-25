'use strict';

const http = require('node:http');
const https = require('node:https');
const path = require('node:path');
const SOC = require('./social');
const HISTORY = require('./social-history');

const DEFAULT_RSSHUB_URL = process.env.RSSHUB_URL || 'http://127.0.0.1:1200';

const DEFAULT_FEEDS = [
  // [S1] Coretax & Layanan Digital
  { route: 'threads/search/coretax', platform: 'threads', topic: 'coretax', stream: 'coretax' },
  { route: 'threads/search/efaktur', platform: 'threads', topic: 'coretax', stream: 'coretax' },
  { route: 'threads/search/ebupot', platform: 'threads', topic: 'coretax', stream: 'coretax' },

  // [S2] Kebijakan & Regulasi
  { route: 'threads/search/pajak', platform: 'threads', topic: 'pajak', stream: 'regulasi' },
  { route: 'threads/search/ppn', platform: 'threads', topic: 'pajak', stream: 'regulasi' },
  { route: 'threads/search/spt', platform: 'threads', topic: 'pajak', stream: 'regulasi' },

  // [S3] Kritis & Akun Otoritas
  { route: 'threads/ditjenpajakri', platform: 'threads', topic: 'pajak', stream: 'kritis', official: true },
  { route: 'twitter/user/ditjenpajakri', platform: 'x', topic: 'pajak', stream: 'kritis', official: true },
  { route: 'twitter/user/kring_pajak', platform: 'x', topic: 'pajak', stream: 'kritis', official: true },
  { route: 'twitter/user/kemenkeuri', platform: 'x', topic: 'pajak', stream: 'kritis', official: true },
];

function decodeHtmlEntities(str) {
  if (!str || typeof str !== 'string') return '';
  return str
    .replace(/&amp;/gi, '&')
    .replace(/&lt;/gi, '<')
    .replace(/&gt;/gi, '>')
    .replace(/&quot;/gi, '"')
    .replace(/&#0*39;|&apos;/gi, "'")
    .replace(/&#(\d+);/g, (_, dec) => {
      try { return String.fromCharCode(parseInt(dec, 10)); } catch { return ''; }
    });
}

function stripHtml(html) {
  if (!html || typeof html !== 'string') return '';
  return decodeHtmlEntities(
    html
      .replace(/<br\s*\/?>/gi, '\n')
      .replace(/<\/p>/gi, '\n')
      .replace(/<[^>]+>/g, ' ')
      .replace(/[ \t]+/g, ' ')
      .replace(/\n\s*\n+/g, '\n')
      .trim()
  );
}

function extractMedia(html) {
  if (!html || typeof html !== 'string') return [];
  const media = [];
  const imgMatches = html.matchAll(/<img[^>]+src=["']([^"']+)["']/gi);
  for (const m of imgMatches) {
    const src = m[1];
    if (src && !src.includes('profile_images') && !src.includes('emoji') && !src.includes('1x1')) {
      media.push(src);
    }
  }
  const videoMatches = html.matchAll(/<video[^>]+poster=["']([^"']+)["']/gi);
  for (const m of videoMatches) {
    const poster = m[1];
    if (poster && !media.includes(poster)) {
      media.push(poster);
    }
  }
  return media;
}

function extractCode(url, fallbackId) {
  if (url && typeof url === 'string') {
    const thMatch = url.match(/(?:\/t\/|\/post\/)([A-Za-z0-9_-]+)/);
    if (thMatch) return thMatch[1];
    const twMatch = url.match(/\/status\/(\d+)/);
    if (twMatch) return twMatch[1];
  }
  if (fallbackId && typeof fallbackId === 'string') {
    const subMatch = fallbackId.match(/([A-Za-z0-9_-]{5,})$/);
    if (subMatch) return subMatch[1];
  }
  return String(fallbackId || Math.random().toString(36).slice(2));
}

function parseRsshubItem(item, feedMeta = {}) {
  if (!item || typeof item !== 'object') return null;

  const url = item.url || item.id || '';
  const code = extractCode(url, item.id);
  const rawHtml = item.content_html || '';
  let text = stripHtml(rawHtml || item.title || item.content_text || '');

  let author = (item.authors && item.authors[0] && item.authors[0].name) || '';
  let username = author.replace(/^@/, '').toLowerCase().trim();

  const titlePrefixMatch = (item.title || '').match(/^@([A-Za-z0-9_.]+):/);
  if (titlePrefixMatch) {
    if (!username) username = titlePrefixMatch[1].toLowerCase().trim();
    if (!author) author = titlePrefixMatch[1];
  }

  if (text.startsWith(`@${username}:`)) {
    text = text.slice(username.length + 2).trim();
  }

  if (!username && feedMeta.route) {
    const userRouteMatch = feedMeta.route.match(/(?:user|channel)\/([A-Za-z0-9_]+)/);
    if (userRouteMatch) {
      username = userRouteMatch[1].toLowerCase();
      if (!author) author = userRouteMatch[1];
    }
  }

  const media = extractMedia(rawHtml);
  const ts = Date.parse(item.date_published || item.date_modified || '') || Date.now();
  const platform = feedMeta.platform || (url.includes('twitter.com') || url.includes('x.com') ? 'x' : 'threads');

  return {
    id: String(item.id || code),
    code,
    url,
    text,
    author: author || username || 'Anon',
    username: username || 'anon',
    handle: username || 'anon',
    created_at: new Date(ts).toISOString(),
    ts,
    platform,
    from: platform === 'threads' ? 'Threads' : platform === 'x' ? 'X' : 'Telegram',
    media,
    images: media,
    like_count: 0,
    likes: 0,
    metrics: { views: 0, likes: 0, reposts: 0, replies: 0 },
    q: feedMeta.route || '',
    stream: feedMeta.stream || 'coretax',
    official: Boolean(feedMeta.official),
  };
}

function fetchJsonFeed(feedUrl, timeoutMs = 8000) {
  return new Promise((resolve, reject) => {
    try {
      const u = new URL(feedUrl);
      const isHttps = u.protocol === 'https:';
      const client = isHttps ? https : http;

      const req = client.get(feedUrl, {
        timeout: timeoutMs,
        headers: {
          'Accept': 'application/json',
          'User-Agent': 'SITREP-DJP/2.0 (+http://pajak.go.id)',
        },
      }, (res) => {
        if (res.statusCode < 200 || res.statusCode >= 300) {
          res.resume();
          return reject(new Error(`HTTP ${res.statusCode} from ${feedUrl}`));
        }
        let data = '';
        res.setEncoding('utf8');
        res.on('data', chunk => { data += chunk; });
        res.on('end', () => {
          try {
            resolve(JSON.parse(data));
          } catch (e) {
            reject(new Error(`Invalid JSON from ${feedUrl}: ${e.message}`));
          }
        });
      });

      req.on('timeout', () => {
        req.destroy(new Error(`Timeout fetching ${feedUrl}`));
      });
      req.on('error', (err) => {
        reject(err);
      });
    } catch (err) {
      reject(err);
    }
  });
}

async function pollRsshubFeeds(options = {}) {
  const baseUrl = (options.baseUrl || DEFAULT_RSSHUB_URL).replace(/\/+$/, '');
  const feeds = options.feeds || DEFAULT_FEEDS;
  const timeoutMs = options.timeoutMs || 8000;

  const results = [];
  const errs = [];

  for (const feed of feeds) {
    const routeUrl = `${baseUrl}/${feed.route.replace(/^\/+/, '')}?format=json`;
    try {
      const json = await fetchJsonFeed(routeUrl, timeoutMs);
      const items = Array.isArray(json.items) ? json.items : [];
      for (const it of items) {
        const post = parseRsshubItem(it, feed);
        if (post && post.text) {
          results.push(post);
        }
      }
    } catch (err) {
      errs.push({ route: feed.route, err: err.message });
    }
  }

  return { posts: results, errs };
}

function createRsshubConnector(deps = {}) {
  const baseUrl = deps.baseUrl || DEFAULT_RSSHUB_URL;
  const feeds = deps.feeds || DEFAULT_FEEDS;
  const tHist = deps.tHist;
  const xHist = deps.xHist;
  const histMerge = deps.histMerge;
  const histDir = deps.histDir;
  const pollIntervalMs = deps.pollIntervalMs || 180000;

  let timer = null;
  let running = false;
  let lastPollTs = 0;
  let lastStats = { fetched: 0, ingested: 0, errs: [] };

  async function pollAndIngest() {
    if (running) return lastStats;
    running = true;
    lastPollTs = Date.now();
    try {
      const { posts, errs } = await pollRsshubFeeds({ baseUrl, feeds });
      let nIngested = 0;
      const thBatch = [];
      const xBatch = [];

      for (const p of posts) {
        const isOfficial = p.official || SOC.SOC_WATCH_X.includes(p.handle);
        const tagged = HISTORY.tag(p, [], Date.now(), {
          platform: p.platform,
          discoverySource: 'rsshub',
          isOfficial,
          isSocialStream: true,
          topic: p.topic || 'pajak',
          maxAge: isOfficial ? 72 * 3600e3 : 48 * 3600e3,
        });

        if (tagged) {
          nIngested++;
          if (p.platform === 'threads' && tHist) {
            thBatch.push(p);
          } else if (p.platform === 'x' && xHist) {
            xBatch.push(p);
          }
        }
      }

      if (thBatch.length && histMerge && tHist) {
        histMerge(tHist, thBatch, 1500, histDir ? path.join(histDir, 'threads-history.json') : null);
      }
      if (xBatch.length && histMerge && xHist) {
        histMerge(xHist, xBatch, 1500, histDir ? path.join(histDir, 'x-history.json') : null);
      }

      lastStats = { fetched: posts.length, ingested: nIngested, errs };
      return lastStats;
    } catch (err) {
      lastStats = { fetched: 0, ingested: 0, errs: [{ route: '*', err: err.message }] };
      return lastStats;
    } finally {
      running = false;
    }
  }

  function start() {
    if (timer) return;
    pollAndIngest().catch(() => {});
    timer = setInterval(() => {
      pollAndIngest().catch(() => {});
    }, pollIntervalMs);
    if (timer.unref) timer.unref();
  }

  function stop() {
    if (timer) {
      clearInterval(timer);
      timer = null;
    }
  }

  function status() {
    return {
      active: !!timer,
      running,
      baseUrl,
      feedsCount: feeds.length,
      lastPollTs,
      lastStats,
    };
  }

  return {
    start,
    stop,
    refresh: pollAndIngest,
    status,
    pollAndIngest,
  };
}

module.exports = {
  DEFAULT_RSSHUB_URL,
  DEFAULT_FEEDS,
  stripHtml,
  decodeHtmlEntities,
  extractMedia,
  extractCode,
  parseRsshubItem,
  fetchJsonFeed,
  pollRsshubFeeds,
  createRsshubConnector,
};
