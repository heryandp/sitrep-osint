'use strict';

const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const PC = require('../src/pajak-cluster');

const ROOT = path.resolve(__dirname, '..');

test('Klastering Khusus Pajak 6 Dimensi (Ideologi, Politik, Sosial & Budaya, Ekonomi, Hukum & HAM, Teknologi)', async (t) => {
  await t.test('exposes exact 6 dimensions in required order', () => {
    assert.deepEqual(
      PC.PAJAK_THEMES.map(x => x.label),
      ['Ideologi', 'Politik', 'Sosial dan Budaya', 'Ekonomi', 'Hukum dan HAM', 'Teknologi']
    );
  });

  await t.test('classifies representative Indonesian tax items into each of the 6 dimensions', () => {
    const samples = [
      ['Seruan boikot pajak dan debat keadilan fiskal Pasal 23A untuk rakyat', 'ideologi'],
      ['Komisi XI DPR dan Menkeu bahas PMK 71/2026 serta RUU Badan Penerimaan Negara', 'politik'],
      ['Kanwil DJP Kalselteng gandeng kampus bentuk Tax Center dan perkuat literasi pajak UMKM', 'sosbud'],
      ['DJP Kepri: Sektor industri topang penerimaan pajak hingga 56% dan target tax ratio APBN', 'ekonomi'],
      ['PPATK temukan dugaan pidana pajak dan faktur fiktif Rp122,9 triliun, DJP jelaskan sita aset', 'hukum'],
      ['Coretax DJP error gagal login OTP dan e-Faktur tidak bisa unggah faktur pajak hari ini', 'teknologi'],
    ];
    for (const [text, expected] of samples) {
      const cls = PC.classifyPajakThemes(text);
      assert.equal(cls.primary, expected, `Expected "${text}" -> ${expected}, got ${cls.primary}`);
    }
  });

  await t.test('clusters cross-source news and social posts on same Coretax outage and flags crossVerified + kritis', () => {
    const now = Date.now();
    const res = PC.buildPajakClusters({
      now,
      win: '24h',
      newsItems: [
        {
          title: 'Wajib Pajak Keluhkan Coretax Error Gagal Login dan e-Faktur di Jakarta',
          desc: 'Gangguan akses login Coretax dan penerbitan e-Faktur terjadi sejak pagi.',
          url: 'https://example.com/coretax-error-1',
          pub: 'DDTCNews',
          cat: 'pajak',
          ts: now - 10 * 60e3,
        },
        {
          title: 'DJP Kepri Catat Penerimaan Pajak Industri Tembus 56 Persen',
          desc: 'Sektor manufaktur mendominasi setoran pajak daerah.',
          url: 'https://example.com/kepri-pajak',
          pub: 'ANTARA Kepri',
          cat: 'pajak',
          ts: now - 30 * 60e3,
        },
        {
          title: 'Pajak jadian makan bareng temen habis nonton drakor',
          url: 'https://example.com/noise',
          pub: 'Spam',
          ts: now - 5 * 60e3,
        },
      ],
      socialPosts: [
        {
          text: 'Coretax error lagi hari ini di Jakarta, gagal login terus saat mau buat e-Faktur',
          url: 'https://threads.net/@wp_jkt/post/1',
          username: 'wp_jkt',
          from: 'Threads',
          ts: now - 8 * 60e3,
          assessment: { topics: ['coretax'] },
        },
      ],
    });

    assert.equal(res.ok, true);
    assert.equal(res.totalItems, 3, 'noise item must be excluded');
    assert.equal(res.totalNews, 2);
    assert.equal(res.totalSocial, 1);
    assert.equal(res.themes.length, 6);

    const techCluster = res.clusters.find(c => c.theme === 'teknologi');
    assert.ok(techCluster, 'must produce teknologi cluster');
    assert.equal(techCluster.crossVerified, true, 'news + social Coretax reports must merge into crossVerified cluster');
    assert.equal(techCluster.urgency, 'kritis', 'Coretax outage must be kritis');
    assert.equal(techCluster.newsCount, 1);
    assert.equal(techCluster.socialCount, 1);
    assert.ok(techCluster.place, 'Jakarta location must attach to Coretax cluster');
  });

  await t.test('separates Pajak Nasional (DJP) vs Pajak Daerah (Pemda/Samsat) and prioritizes Nasional', () => {
    assert.equal(PC.classifyTaxScope('DJP Kepri: Penerimaan PPh dan PPN naik 12%'), 'nasional');
    assert.equal(PC.classifyTaxScope('Coretax error saat unggah e-Faktur di KPP Pratama'), 'nasional');
    assert.equal(PC.classifyTaxScope('Pemprov DKI Beri Keringanan Pokok Pajak Kendaraan Bermotor di Samsat'), 'daerah');
    assert.equal(PC.classifyTaxScope('Bapenda Kejar Target PAD dari PBB-P2 dan Pajak Restoran PBJT'), 'daerah');

    const now = Date.now();
    const both = PC.buildPajakClusters({
      now,
      newsItems: [
        { title: 'Pemprov DKI Beri Pemutihan Pajak Kendaraan Bermotor Samsat', url: 'https://a/1', pub: 'MediaA', cat: 'pajak', ts: now - 1000 },
        { title: 'DJP Raup Penerimaan Pajak Rp898 Miliar dari Program PSA', url: 'https://a/2', pub: 'DDTCNews', cat: 'pajak', ts: now - 2000 },
      ],
    });
    assert.equal(both.nasionalClusters, 1);
    assert.equal(both.daerahClusters, 1);
    assert.equal(both.clusters[0].taxScope, 'nasional', 'Pajak Nasional (DJP) must rank above Pajak Daerah by default');

    const nasOnly = PC.buildPajakClusters({
      now,
      scope: 'nasional',
      newsItems: [
        { title: 'Pemprov DKI Beri Pemutihan Pajak Kendaraan Bermotor Samsat', url: 'https://a/1', pub: 'MediaA', cat: 'pajak', ts: now - 1000 },
        { title: 'DJP Raup Penerimaan Pajak Rp898 Miliar dari Program PSA', url: 'https://a/2', pub: 'DDTCNews', cat: 'pajak', ts: now - 2000 },
      ],
    });
    assert.equal(nasOnly.totalClusters, 1);
    assert.equal(nasOnly.clusters[0].taxScope, 'nasional');
  });

  await t.test('compiled public/index.html wires Klaster Pajak window, trigger button, and /api/pajak/clusters', () => {
    const html = fs.readFileSync(path.join(ROOT, 'public', 'index.html'), 'utf8');
    assert.ok(html.includes('id="btn-pajak"'), 'Geobar must include #btn-pajak trigger button');
    assert.ok(html.includes('id="pajakwrap"'), 'DOM must include #pajakwrap window');
    assert.ok(html.includes('id="pj-scope"'), 'Klaster Pajak window must expose Nasional/Daerah scope switcher');
    assert.ok(html.includes('/api/pajak/clusters'), 'client script must fetch /api/pajak/clusters');
    assert.ok(html.includes('Sosial dan Budaya') && html.includes('Hukum dan HAM'), 'client must define all 6 theme labels');
  });
});
