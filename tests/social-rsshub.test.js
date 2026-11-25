'use strict';

const { test } = require('node:test');
const assert = require('node:assert/strict');
const RSSHUB = require('../src/social-rsshub');
const HISTORY = require('../src/social-history');

test('social-rsshub: HTML stripping and entity decoding', () => {
  const html = '<p><strong>@ditjenpajakri</strong>:</p><p>Selamat Hari Guru!<br>Pajak kita &amp; pendidikan &quot;maju&quot;.</p>';
  const clean = RSSHUB.stripHtml(html);
  assert.ok(clean.includes('Selamat Hari Guru!'));
  assert.ok(clean.includes('Pajak kita & pendidikan "maju".'));
  assert.ok(!clean.includes('<p>'));
  assert.ok(!clean.includes('&amp;'));
});

test('social-rsshub: extractMedia parses img and video posters', () => {
  const html = '<p>Text</p><p><img src="https://cdn.example.com/photo1.jpg" /><img src="https://cdn.example.com/profile_images/avatar.jpg" /><video poster="https://cdn.example.com/video_thumb.jpg"></video></p>';
  const media = RSSHUB.extractMedia(html);
  assert.equal(media.length, 2);
  assert.equal(media[0], 'https://cdn.example.com/photo1.jpg');
  assert.equal(media[1], 'https://cdn.example.com/video_thumb.jpg');
});

test('social-rsshub: extractCode extracts post ID or slug from URLs', () => {
  assert.equal(RSSHUB.extractCode('https://www.threads.com/t/DgJglKRksEu'), 'DgJglKRksEu');
  assert.equal(RSSHUB.extractCode('https://www.threads.com/@user/post/DgIMfYUk1E6'), 'DgIMfYUk1E6');
  assert.equal(RSSHUB.extractCode('https://x.com/KemenkeuRI/status/2125475667731189989'), '2125475667731189989');
  assert.equal(RSSHUB.extractCode('', 'fallback_123'), 'fallback_123');
});

test('social-rsshub: parseRsshubItem constructs valid PostRecord for Threads', () => {
  const sample = {
    id: 'https://www.threads.com/t/DgJglKRksEu',
    url: 'https://www.threads.com/t/DgJglKRksEu',
    title: '@ditjenpajakri: Selamat Hari Guru! Di balik setiap generasi yang tumbuh...',
    content_html: '<p><strong>@ditjenpajakri</strong>:</p><p>Selamat Hari Guru! Di balik setiap generasi yang tumbuh...</p><p><img src="https://cdn.example.com/img1.jpg" /></p>',
    date_published: '2026-11-25T02:57:10.000Z',
    authors: [{ name: 'ditjenpajakri' }]
  };

  const post = RSSHUB.parseRsshubItem(sample, {
    route: 'threads/ditjenpajakri',
    platform: 'threads',
    stream: 'kritis',
    official: true
  });

  assert.equal(post.code, 'DgJglKRksEu');
  assert.equal(post.username, 'ditjenpajakri');
  assert.equal(post.platform, 'threads');
  assert.equal(post.from, 'Threads');
  assert.equal(post.stream, 'kritis');
  assert.equal(post.official, true);
  assert.equal(post.media.length, 1);
  assert.ok(post.text.includes('Selamat Hari Guru!'));
  assert.ok(!post.text.startsWith('@ditjenpajakri:'));

  const tagged = HISTORY.tag(post, [], Date.now(), {
    platform: post.platform,
    discoverySource: 'rsshub',
    isOfficial: true,
    topic: 'pajak',
    maxAge: 72 * 3600e3,
  });

  assert.equal(tagged, true);
  assert.ok(post.topics.includes('pajak'));
  assert.equal(post.assessment.feedEligible, true);
  assert.ok(post.assessment.provenance.sourceType.startsWith('official'));
});

test('social-rsshub: parseRsshubItem constructs valid PostRecord for Twitter', () => {
  const sample = {
    id: 'https://twitter.com/kring_pajak/status/2053741794287051074',
    url: 'https://twitter.com/kring_pajak/status/2053741794287051074',
    title: 'Pelayanan informasi perpajakan dapat diakses melalui kring pajak 1500200...',
    content_html: '<p>Pelayanan informasi perpajakan dapat diakses melalui kring pajak 1500200...</p>',
    date_published: '2026-11-25T03:00:00.000Z',
    authors: [{ name: 'kring_pajak' }]
  };

  const post = RSSHUB.parseRsshubItem(sample, {
    route: 'twitter/user/kring_pajak',
    platform: 'x',
    stream: 'kritis',
    official: true
  });

  assert.equal(post.code, '2053741794287051074');
  assert.equal(post.username, 'kring_pajak');
  assert.equal(post.platform, 'x');
  assert.equal(post.from, 'X');
  assert.equal(post.official, true);

  const tagged = HISTORY.tag(post, [], Date.now(), {
    platform: post.platform,
    discoverySource: 'rsshub',
    isOfficial: true,
    topic: 'pajak',
  });

  assert.equal(tagged, true);
  assert.ok(post.topics.includes('pajak'));
  assert.equal(post.assessment.feedEligible, true);
});

test('social-rsshub: parseRsshubItem constructs valid PostRecord for Bluesky', () => {
  const sample = {
    id: 'https://bsky.app/profile/jasafinance.bsky.social/post/3n34lunyw722g',
    url: 'https://bsky.app/profile/jasafinance.bsky.social/post/3n34lunyw722g',
    title: 'BUTUH JASA KONSULTASI PERPAJAKAN? PPh 21, SPT Tahunan, PPN...',
    content_html: '<p>BUTUH JASA KONSULTASI PERPAJAKAN? PPh 21, SPT Tahunan, PPN</p>',
    date_published: new Date(Date.now() - 1800e3).toISOString(),
    authors: [{ name: 'Jasa Akuntansi, Finance & Tax', avatar: 'https://cdn.bsky.app/avatar.jpg' }]
  };

  const post = RSSHUB.parseRsshubItem(sample, {
    route: 'bsky/keyword/pajak',
    platform: 'bsky',
    stream: 'regulasi',
  });

  assert.equal(post.platform, 'bsky');
  assert.equal(post.from, 'Bluesky');
  assert.equal(post.username, 'jasafinance.bsky.social');
  assert.equal(post.author, 'Jasa Akuntansi, Finance & Tax');
  assert.equal(post.avatar, 'https://cdn.bsky.app/avatar.jpg');
  assert.equal(post.stream, 'regulasi');

  const tagged = HISTORY.tag(post, [], Date.now(), {
    platform: post.platform,
    discoverySource: 'rsshub',
    topic: 'pajak',
  });
  assert.equal(tagged, true);
  assert.ok(post.topics.includes('pajak'));
});

test('social-rsshub: parseRsshubItem constructs valid PostRecord for YouTube', () => {
  const sample = {
    id: 'https://www.youtube.com/watch?v=AXFJYS2FyQA',
    url: 'https://www.youtube.com/watch?v=AXFJYS2FyQA',
    title: 'Yuk Lapor Pajak pakai Coretax DJP',
    content_html: '<iframe src="https://www.youtube-nocookie.com/embed/AXFJYS2FyQA"></iframe>',
    image: 'https://i.ytimg.com/vi/AXFJYS2FyQA/maxresdefault.jpg',
    date_published: new Date(Date.now() - 3600e3).toISOString(),
    authors: [{ name: 'Direktorat Jenderal Pajak' }]
  };

  const post = RSSHUB.parseRsshubItem(sample, {
    route: 'youtube/user/DitjenPajakRI',
    platform: 'youtube',
    stream: 'coretax',
    official: true,
  });

  assert.equal(post.platform, 'youtube');
  assert.equal(post.from, 'YouTube');
  assert.equal(post.username, 'ditjenpajakri');
  assert.equal(post.official, true);
  assert.equal(post.stream, 'coretax');
  assert.ok(post.media.includes('https://i.ytimg.com/vi/AXFJYS2FyQA/maxresdefault.jpg'));
  assert.ok(post.text.includes('Yuk Lapor Pajak pakai Coretax DJP'));

  const tagged = HISTORY.tag(post, [], Date.now(), {
    platform: post.platform,
    discoverySource: 'rsshub',
    isOfficial: true,
    topic: 'coretax',
  });
  assert.equal(tagged, true);
  assert.ok(post.topics.includes('coretax'));
});

test('social-rsshub: createRsshubConnector lifecycle', () => {
  const tHist = new Map();
  const xHist = new Map();
  const connector = RSSHUB.createRsshubConnector({
    baseUrl: 'http://127.0.0.1:1200',
    tHist,
    xHist,
    pollIntervalMs: 60000,
  });

  const st1 = connector.status();
  assert.equal(st1.active, false);
  assert.equal(st1.baseUrl, 'http://127.0.0.1:1200');
  assert.ok(st1.feedsCount >= 4);

  connector.stop();
});

test('social-rsshub: parseXmlRssFeed parses direct XML RSS items', () => {
  const xmlSample = `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Google News - site:x.com coretax</title>
    <item>
      <title>Hai, Kak. Di Coretax tidak ada pengecekan NTPN - X</title>
      <link>https://news.google.com/rss/articles/CBMiYkFVX3lxTFBTcXlGaDFlUTBuS3kxajlOZVNXbjVBTElJU1J1NDFtQ18xUQ?oc=5</link>
      <guid isPermaLink="false">CBMiYkFVX3lxTFBTcXlGaDFlUTBuS3kxajlOZVNXbjVBTElJU1J1NDFtQ18xUQ</guid>
      <pubDate>Wed, 25 Nov 2026 05:00:00 GMT</pubDate>
      <description>&lt;a href="https://news.google.com/rss/articles/..." target="_blank"&gt;@kring_pajak Hai, Kak. Di Coretax tidak ada pengecekan NTPN seperti rumah konfirmasi dokumen&lt;/a&gt;&amp;nbsp;&amp;nbsp;&lt;font color="#6f6f6f"&gt;X&lt;/font&gt;</description>
      <source url="https://x.com">X</source>
    </item>
  </channel>
</rss>`;

  const posts = RSSHUB.parseXmlRssFeed(xmlSample, {
    platform: 'x',
    topic: 'coretax',
    stream: 'coretax',
  });

  assert.equal(posts.length, 1);
  const p = posts[0];
  assert.equal(p.platform, 'x');
  assert.equal(p.from, 'X');
  assert.equal(p.username, 'kring_pajak');
  assert.equal(p.official, true);
  assert.equal(p.stream, 'coretax');
  assert.ok(p.text.includes('Di Coretax tidak ada pengecekan NTPN'));
  assert.ok(!p.text.includes('<a href'));

  const tagged = HISTORY.tag(p, [], Date.now(), {
    platform: p.platform,
    discoverySource: 'direct_rss',
    isOfficial: p.official,
    topic: p.topic,
  });
  assert.equal(tagged, true);
  assert.ok(p.topics.includes('coretax'));
});
